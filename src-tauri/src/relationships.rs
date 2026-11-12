//! Character relationships (`DATA_MODEL.md` §10.3), schema v12.
//!
//! Edges are typed and directed. Types declare both a forward and a reverse
//! label, so the UI can say "Mentor of" from one character and "Student of"
//! from the other without storing the fact twice.
//!
//! Direction is normalised here, not in SQL: for symmetric types the pair is
//! stored with the smaller character id as `character_a_id`, so "friend of" is
//! one row regardless of which character the user opened. SQLite forbids
//! subqueries in `CHECK` constraints, so this rule lives in one function and is
//! covered by `symmetric_relationships_are_stored_once`.

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{new_id, now};
use crate::error::{AppError, AppResult};
use crate::models::{CharacterRelationship, RelationshipInput, RelationshipPatch, RelationshipType};

fn type_id(conn: &Connection, code: &str) -> AppResult<(i64, bool)> {
    conn.query_row(
        "SELECT id, is_symmetric FROM relationship_types WHERE code = ?1",
        [code],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? != 0)),
    )
    .optional()?
    .ok_or_else(|| AppError::invalid(format!("unknown relationship type: {code}")))
}

pub fn types(conn: &Connection) -> AppResult<Vec<RelationshipType>> {
    let mut stmt = conn.prepare(
        "SELECT code, forward_label, reverse_label, is_symmetric
         FROM relationship_types ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RelationshipType {
            code: row.get(0)?,
            forward_label: row.get(1)?,
            reverse_label: row.get(2)?,
            is_symmetric: row.get::<_, i64>(3)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Every edge touching the character, with the label resolved for this side.
pub fn for_character(
    conn: &Connection,
    character_id: &str,
) -> AppResult<Vec<CharacterRelationship>> {
    let mut stmt = conn.prepare(
        "SELECT r.id,
                CASE WHEN r.character_a_id = ?1 THEN r.character_b_id ELSE r.character_a_id END,
                c.name, c.series_id, s.title,
                t.code,
                CASE WHEN t.is_symmetric = 1 OR r.character_a_id = ?1
                     THEN t.forward_label ELSE t.reverse_label END,
                r.status, r.description, t.is_symmetric
         FROM character_relationships r
         JOIN relationship_types t ON t.id = r.relationship_type_id
         JOIN characters c
           ON c.id = CASE WHEN r.character_a_id = ?1 THEN r.character_b_id ELSE r.character_a_id END
         LEFT JOIN series s ON s.id = c.series_id
         WHERE r.character_a_id = ?1 OR r.character_b_id = ?1
         ORDER BY t.id, c.sort_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([character_id], |row| {
        Ok(CharacterRelationship {
            id: row.get(0)?,
            character_id: row.get(1)?,
            name: row.get(2)?,
            series_id: row.get(3)?,
            series_title: row.get(4)?,
            relationship_type: row.get(5)?,
            label: row.get(6)?,
            status: row.get(7)?,
            description: row.get(8)?,
            is_symmetric: row.get::<_, i64>(9)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Creates an edge from `character_id` to `input.other_character_id`.
///
/// Symmetric types are stored once, with the smaller id first, so linking A–B
/// and then B–A updates the same row instead of creating a mirror image.
/// Returns the resulting edge as seen from `character_id`.
pub fn create(
    conn: &Connection,
    character_id: &str,
    input: &RelationshipInput,
) -> AppResult<CharacterRelationship> {
    if character_id == input.other_character_id {
        return Err(AppError::invalid(
            "a character cannot have a relationship with itself",
        ));
    }
    let exists: Option<String> = conn
        .query_row("SELECT id FROM characters WHERE id = ?1", [character_id], |row| {
            row.get(0)
        })
        .optional()?;
    if exists.is_none() {
        return Err(AppError::not_found("character"));
    }
    let other: Option<String> = conn
        .query_row(
            "SELECT id FROM characters WHERE id = ?1",
            [&input.other_character_id],
            |row| row.get(0),
        )
        .optional()?;
    if other.is_none() {
        return Err(AppError::not_found("character"));
    }
    let (type_id, symmetric) = type_id(conn, &input.relationship_type)?;
    let status = normalize_status(input.status.as_deref())?;

    let (a, b) = if symmetric && input.other_character_id.as_str() < character_id {
        (input.other_character_id.clone(), character_id.to_string())
    } else {
        (character_id.to_string(), input.other_character_id.clone())
    };

    let timestamp = now();
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM character_relationships
             WHERE character_a_id = ?1 AND character_b_id = ?2 AND relationship_type_id = ?3",
            params![a, b, type_id],
            |row| row.get(0),
        )
        .optional()?;

    let id = match existing {
        Some(id) => {
            conn.execute(
                "UPDATE character_relationships
                 SET status = ?2, description = ?3, updated_at = ?4
                 WHERE id = ?1",
                params![id, status, clean(input.description.as_deref()), timestamp],
            )?;
            id
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO character_relationships
                    (id, character_a_id, character_b_id, relationship_type_id, status,
                     description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    id,
                    a,
                    b,
                    type_id,
                    status,
                    clean(input.description.as_deref()),
                    timestamp
                ],
            )?;
            id
        }
    };

    for_character(conn, character_id)?
        .into_iter()
        .find(|relationship| relationship.id == id)
        .ok_or_else(|| AppError::not_found("relationship"))
}

/// Edits the status/description of an edge. Direction and type are immutable:
/// changing either is a different fact, so it is a delete plus a create.
pub fn update(
    conn: &Connection,
    relationship_id: &str,
    patch: &RelationshipPatch,
) -> AppResult<()> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM character_relationships WHERE id = ?1",
            [relationship_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(AppError::not_found("relationship"));
    }
    if let Some(status) = patch.status.as_deref() {
        let status = normalize_status(Some(status))?;
        conn.execute(
            "UPDATE character_relationships SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![relationship_id, status, now()],
        )?;
    }
    if let Some(description) = patch.description.as_deref() {
        conn.execute(
            "UPDATE character_relationships SET description = ?2, updated_at = ?3 WHERE id = ?1",
            params![relationship_id, clean(Some(description)), now()],
        )?;
    }
    Ok(())
}

pub fn delete(conn: &Connection, relationship_id: &str) -> AppResult<()> {
    let changed = conn.execute(
        "DELETE FROM character_relationships WHERE id = ?1",
        [relationship_id],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("relationship"));
    }
    Ok(())
}

fn normalize_status(value: Option<&str>) -> AppResult<&'static str> {
    Ok(match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("active") => "active",
        Some("former") => "former",
        Some("unknown") => "unknown",
        Some("secret") => "secret",
        Some(other) => {
            return Err(AppError::invalid(format!(
                "relationship status must be active, former, unknown or secret (got {other})"
            )))
        }
    })
}

fn clean(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// Number of edges touching a character, for card badges.
pub fn count_for_character(conn: &Connection, character_id: &str) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM character_relationships
         WHERE character_a_id = ?1 OR character_b_id = ?1",
        [character_id],
        |row| row.get(0),
    )?)
}
