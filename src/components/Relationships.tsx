import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import type { CharacterRelationship } from "../lib/types";
import { useToast } from "./Toast";
import { Icon } from "./Icon";
import { cx } from "../lib/utils";

const STATUSES = ["active", "former", "unknown", "secret"] as const;

export function RelationshipsPanel({
  characterId,
  relationships,
  onOpenCharacter,
}: {
  characterId: string;
  relationships: CharacterRelationship[];
  onOpenCharacter?: (characterId: string) => void;
}) {
  const queryClient = useQueryClient();
  const toast = useToast();
  const [adding, setAdding] = useState(false);
  const [otherId, setOtherId] = useState("");
  const [typeCode, setTypeCode] = useState("ally_of");
  const [status, setStatus] = useState<(typeof STATUSES)[number]>("active");
  const [description, setDescription] = useState("");

  const types = useQuery({ queryKey: ["relationship-types"], queryFn: api.relationshipTypes });
  const characters = useQuery({ queryKey: ["characters"], queryFn: api.allCharacters });

  const bySeries = useMemo(() => {
    const groups = new Map<string, { id: string; name: string }[]>();
    for (const character of characters.data ?? []) {
      if (character.id === characterId) continue;
      const key = character.seriesTitle ?? "No series";
      const list = groups.get(key) ?? [];
      list.push({ id: character.id, name: character.name });
      groups.set(key, list);
    }
    return [...groups.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [characters.data, characterId]);

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["character", characterId] });
    await queryClient.invalidateQueries({ queryKey: ["characters"] });
  };

  const create = useMutation({
    mutationFn: () =>
      api.createRelationship(characterId, {
        otherCharacterId: otherId,
        relationshipType: typeCode,
        status,
        description: description.trim() || undefined,
      }),
    onSuccess: async () => {
      setOtherId("");
      setDescription("");
      setAdding(false);
      await invalidate();
      toast.push("Relationship added", "info");
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const update = useMutation({
    mutationFn: ({ id, next }: { id: string; next: (typeof STATUSES)[number] }) =>
      api.updateRelationship(id, { status: next }),
    onSuccess: invalidate,
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteRelationship(id),
    onSuccess: async () => {
      await invalidate();
      toast.push("Relationship removed", "info");
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  return (
    <div>
      <div className="mb-1 flex items-center justify-between">
        <p className="label">Relationships</p>
        <button
          className="btn btn-ghost btn-sm"
          onClick={() => setAdding((value) => !value)}
          disabled={bySeries.length === 0}
          title={
            bySeries.length === 0
              ? "Import a second character first"
              : "Link this character to another"
          }
        >
          <Icon name={adding ? "x" : "plus"} className="size-3.5" />
          {adding ? "Cancel" : "Add"}
        </button>
      </div>

      {adding && (
        <div className="mb-2 space-y-2 rounded-xl border border-[var(--line)] bg-[var(--surface-2)] p-2.5">
          <div className="grid gap-2 sm:grid-cols-2">
            <label className="block">
              <span className="label">Character</span>
              <select
                className="input"
                value={otherId}
                onChange={(event) => setOtherId(event.currentTarget.value)}
              >
                <option value="">Choose a character…</option>
                {bySeries.map(([series, entries]) => (
                  <optgroup key={series} label={series}>
                    {entries.map((entry) => (
                      <option key={entry.id} value={entry.id}>
                        {entry.name}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
            </label>
            <label className="block">
              <span className="label">Type</span>
              <select
                className="input"
                value={typeCode}
                onChange={(event) => setTypeCode(event.currentTarget.value)}
              >
                {(types.data ?? []).map((type) => (
                  <option key={type.code} value={type.code}>
                    {type.forwardLabel}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            <label className="block">
              <span className="label">Status</span>
              <select
                className="input"
                value={status}
                onChange={(event) =>
                  setStatus(event.currentTarget.value as (typeof STATUSES)[number])
                }
              >
                {STATUSES.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
            </label>
            <label className="block">
              <span className="label">Note</span>
              <input
                className="input"
                value={description}
                placeholder="Optional context"
                onChange={(event) => setDescription(event.currentTarget.value)}
              />
            </label>
          </div>
          <div className="flex justify-end">
            <button
              className="btn btn-primary btn-sm"
              disabled={!otherId || create.isPending}
              onClick={() => create.mutate()}
            >
              {create.isPending ? "Linking…" : "Link characters"}
            </button>
          </div>
        </div>
      )}

      {relationships.length === 0 ? (
        <p className="text-[12px] text-muted">
          No relationships recorded. Links are manual: nothing is inferred from the
          <code className="mx-1">&lt;Characters&gt;</code> tag alone.
        </p>
      ) : (
        <ul className="max-h-56 space-y-1 overflow-y-auto pr-1">
          {relationships.map((relationship) => (
            <li
              key={relationship.id}
              className="flex items-center gap-2 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-2.5 py-1.5 text-[12px]"
            >
              <span className="w-28 shrink-0 text-muted">{relationship.label}</span>
              <button
                className="flex-1 truncate text-left hover:text-[var(--accent)]"
                onClick={() => onOpenCharacter?.(relationship.characterId)}
                disabled={!onOpenCharacter}
                title={relationship.seriesTitle ?? undefined}
              >
                {relationship.name}
              </button>
              <select
                className={cx(
                  "rounded border border-[var(--line)] bg-[var(--surface-2)] px-1 py-0.5 text-[10px] uppercase tracking-wide",
                  relationship.status === "secret" ? "text-muted" : "text-[var(--text)]",
                )}
                value={relationship.status}
                aria-label={`Status of relationship with ${relationship.name}`}
                onChange={(event) =>
                  update.mutate({
                    id: relationship.id,
                    next: event.currentTarget.value as (typeof STATUSES)[number],
                  })
                }
              >
                {STATUSES.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
              <button
                className="btn btn-ghost btn-sm px-1"
                aria-label={`Remove relationship with ${relationship.name}`}
                onClick={() => remove.mutate(relationship.id)}
              >
                <Icon name="trash" className="size-3.5" />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
