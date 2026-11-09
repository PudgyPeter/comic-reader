import { useMemo, useState } from "react";
import type { ImportCandidate, ImportOverride, ImportResult } from "../lib/types";
import { api } from "../lib/api";
import { cx, pluralize } from "../lib/utils";
import { Modal, Spinner } from "./ui";
import { Icon } from "./Icon";
import { useToast } from "./Toast";
import { useImportActivity } from "../store/importActivity";

type Editable = ImportCandidate & {
  edited: boolean;
  keepUnmatched: boolean;
  sourcePath: string;
};

const FIELDS: { key: keyof ImportCandidate; label: string; className?: string }[] = [
  { key: "seriesTitle", label: "Series", className: "sm:col-span-2" },
  { key: "number", label: "Number" },
  { key: "title", label: "Title", className: "sm:col-span-2" },
  { key: "publisher", label: "Publisher" },
  { key: "releaseDate", label: "Release date" },
  { key: "volume", label: "Volume" },
  { key: "writer", label: "Writer" },
  { key: "artist", label: "Artist" },
  { key: "coverArtist", label: "Cover artist" },
];

function toOverride(candidate: Editable): ImportOverride | null {
  if (!candidate.edited && !candidate.keepUnmatched) return null;
  return {
    relativePath: candidate.relativePath,
    seriesTitle: candidate.seriesTitle || undefined,
    title: candidate.title || undefined,
    number: candidate.number || undefined,
    publisher: candidate.publisher || undefined,
    releaseDate: candidate.releaseDate || undefined,
    volume: candidate.volume || undefined,
    writer: candidate.writer || undefined,
    artist: candidate.artist || undefined,
    coverArtist: candidate.coverArtist || undefined,
    description: candidate.description || undefined,
    keepUnmatched: candidate.keepUnmatched || undefined,
  };
}

function mergeResults(a: ImportResult, b: ImportResult): ImportResult {
  return {
    root: a.root,
    filesImported: a.filesImported + b.filesImported,
    filesSkipped: a.filesSkipped + b.filesSkipped,
    seriesCreated: a.seriesCreated + b.seriesCreated,
    issuesCreated: a.issuesCreated + b.issuesCreated,
    warnings: [...a.warnings, ...b.warnings],
  };
}

export function ImportReview({
  sources,
  onClose,
  onImported,
}: {
  sources: { path: string; candidates: ImportCandidate[] }[];
  onClose: () => void;
  onImported: (result: ImportResult) => void;
}) {
  const toast = useToast();
  const beginActivity = useImportActivity((state) => state.start);
  const endActivity = useImportActivity((state) => state.end);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [rows, setRows] = useState<Editable[]>(() =>
    sources.flatMap((source) =>
      source.candidates.map((candidate) => ({
        ...candidate,
        edited: false,
        keepUnmatched: false,
        sourcePath: source.path,
      })),
    ),
  );
  const [busy, setBusy] = useState(false);

  const editedCount = useMemo(() => rows.filter((row) => row.edited).length, [rows]);
  const duplicates = useMemo(() => rows.filter((row) => row.duplicate).length, [rows]);

  const update = (index: number, key: keyof ImportCandidate, value: string) => {
    setRows((current) =>
      current.map((row, rowIndex) =>
        rowIndex === index ? { ...row, [key]: value, edited: true } : row,
      ),
    );
  };

  const toggleUnmatched = (index: number) => {
    setRows((current) =>
      current.map((row, rowIndex) =>
        rowIndex === index ? { ...row, keepUnmatched: !row.keepUnmatched } : row,
      ),
    );
  };

  const commit = async () => {
    setBusy(true);
    beginActivity("Importing reviewed files");
    try {
      // Rows are grouped by the root they were previewed from (a folder, or a
      // single archive file), so each commit gets only its own overrides.
      let result: ImportResult | null = null;
      for (const source of sources) {
        const overrides = rows
          .filter((row) => row.sourcePath === source.path)
          .map(toOverride)
          .filter((override): override is ImportOverride => override !== null);
        const partial = await api.commitImport(source.path, overrides);
        result = result ? mergeResults(result, partial) : partial;
      }
      if (!result) {
        onClose();
        return;
      }
      toast.push(
        `Imported ${pluralize(result.filesImported, "file")} · ${pluralize(
          result.issuesCreated,
          "issue",
        )}${editedCount ? ` · ${editedCount} corrected` : ""}`,
        result.warnings.length ? "info" : "success",
      );
      onImported(result);
      onClose();
    } catch (error) {
      if (error instanceof Error && error.name === "cancelled") {
        toast.push("Import cancelled", "info");
      } else {
        toast.push(String(error), "error");
      }
    } finally {
      endActivity();
      setBusy(false);
    }
  };

  return (
    <Modal
      open
      title={`Review import · ${pluralize(rows.length, "file")}`}
      onClose={onClose}
      width="max-w-3xl"
      footer={
        <>
          <span className="mr-auto text-[11px] text-muted">
            {editedCount > 0 && `${editedCount} corrected · `}
            {duplicates > 0 && `${duplicates} possible duplicate${duplicates === 1 ? "" : "s"} · `}
            Corrections apply before anything is written to the library.
          </span>
          <button className="btn" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={commit} disabled={busy || rows.length === 0}>
            {busy ? <Spinner className="size-3.5" /> : <Icon name="check" className="size-3.5" />}
            Import {pluralize(rows.length, "file")}
          </button>
        </>
      }
    >
      {rows.length === 0 ? (
        <p className="rounded-lg border border-dashed border-[var(--line)] px-3 py-8 text-center text-xs text-muted">
          Nothing new to import. Every readable file in this folder is already indexed.
        </p>
      ) : (
        <div className="max-h-[55vh] space-y-1.5 overflow-y-auto pr-1">
          {rows.map((row, index) => (
            <div
              key={row.relativePath}
              className="rounded-lg border border-[var(--line)] bg-[var(--surface-1)]"
            >
              <button
                className="flex w-full items-center gap-2 px-3 py-2 text-left"
                onClick={() => setEditingIndex(editingIndex === index ? null : index)}
                aria-expanded={editingIndex === index}
              >
                <span className="rounded border border-[var(--line)] px-1 py-0.5 font-mono text-[9px] uppercase text-muted">
                  {row.format}
                </span>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-[12px] font-medium">
                    {row.seriesTitle || "—"} {row.number ? `#${row.number}` : ""}
                  </p>
                  <p className="truncate text-[10px] text-muted" title={row.relativePath}>
                    {row.relativePath}
                  </p>
                </div>
                {row.keepUnmatched && <span className="chip">Unmatched</span>}
                {row.edited && <span className="chip chip-accent">Edited</span>}
                {row.duplicate && (
                  <span className="chip border-amber-400/30 bg-amber-400/10 text-amber-200">
                    Duplicate?
                  </span>
                )}
                {row.warning && (
                  <span className="chip border-rose-400/30 bg-rose-400/10 text-rose-200">
                    {row.warning}
                  </span>
                )}
                <span className="shrink-0 text-[10px] tabular-nums text-muted">
                  {row.pageCount}p
                </span>
                <Icon
                  name="chevronDown"
                  className={cx(
                    "size-3.5 shrink-0 text-muted transition-transform",
                    editingIndex === index && "rotate-180",
                  )}
                />
              </button>
              {editingIndex === index && (
                <div className="grid grid-cols-1 gap-2 border-t border-[var(--line)] p-3 sm:grid-cols-3">
                  {FIELDS.map((field) => (
                    <label key={field.key} className={field.className}>
                      <span className="label">{field.label}</span>
                      <input
                        className="input h-8 text-xs"
                        value={(row[field.key] as string | null) ?? ""}
                        onChange={(event) =>
                          update(index, field.key, event.currentTarget.value)
                        }
                      />
                    </label>
                  ))}
                  <label className="sm:col-span-3">
                    <span className="label">Description</span>
                    <textarea
                      className="input min-h-16 text-xs"
                      value={row.description ?? ""}
                      onChange={(event) => update(index, "description", event.currentTarget.value)}
                    />
                  </label>
                  <label className="flex items-center gap-2 text-[11px] text-muted sm:col-span-3">
                    <input
                      type="checkbox"
                      checked={row.keepUnmatched}
                      onChange={() => toggleUnmatched(index)}
                    />
                    Keep this file unmatched for now (readable, match later without losing progress)
                  </label>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}
