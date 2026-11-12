import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, ProgressBar, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { cx, formatDate, pluralize, progressRatio } from "../lib/utils";
import { AddToCollectionMenu } from "../components/AddToCollection";
import { CharactersPanel } from "../components/Characters";
import { useToast } from "../components/Toast";
import type { IssueCard as SeriesDetailIssue } from "../lib/types";

export default function SeriesDetailPage() {
  const { seriesId = "" } = useParams();
  const navigate = useNavigate();
  const toast = useToast();
  const queryClient = useQueryClient();
  const [bulkBusy, setBulkBusy] = useState(false);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<{
    title: string;
    publisher: string;
    universe: string;
    description: string;
    readingDirection: "" | "ltr" | "rtl";
    coverFileId: string;
    metadataLocked: boolean;
  } | null>(null);
  const [pickingCover, setPickingCover] = useState(false);

  const query = useQuery({
    queryKey: ["series", seriesId],
    queryFn: () => api.seriesDetail(seriesId),
    enabled: Boolean(seriesId),
  });

  const characters = useQuery({
    queryKey: ["characters", seriesId],
    queryFn: () => api.seriesCharacters(seriesId),
    enabled: Boolean(seriesId),
  });

  const coverOptions = useQuery({
    queryKey: ["series-cover-options"],
    queryFn: () => api.library({ sort: "series" }),
    enabled: pickingCover,
  });

  if (query.isLoading) {
    return (
      <div className="grid h-64 place-items-center">
        <Spinner />
      </div>
    );
  }
  if (query.isError || !query.data) {
    return (
      <div className="p-6">
        <EmptyState title="Series not found" body="It may have been deleted or merged." />
      </div>
    );
  }

  const series = query.data;
  const form = draft ?? {
    title: series.title,
    publisher: series.publisher ?? "",
    universe: series.universe ?? "",
    description: series.description ?? "",
    readingDirection: (series.readingDirection ?? "") as "" | "ltr" | "rtl",
    coverFileId: series.coverFileId ?? "",
    metadataLocked: series.metadataLocked ?? false,
  };
  const setForm = (value: typeof form) => setDraft(value);
  const next = series.issues.find((issue) => issue.status !== "completed") ?? series.issues[0];

  const setMissing = async (number: string, reason: "missing" | "not-required" | null) => {
    await api.setMissingReason(seriesId, number, reason);
    await query.refetch();
  };

  const applyToSeries = async (
    run: () => Promise<number>,
    success: (count: number) => string,
  ) => {
    setBulkBusy(true);
    try {
      const count = await run();
      await query.refetch();
      await queryClient.invalidateQueries({ queryKey: ["library"] });
      toast.push(success(count), "success");
    } catch (error) {
      toast.push(String(error), "error");
    } finally {
      setBulkBusy(false);
    }
  };

  const save = async () => {
    await api.updateSeries(seriesId, {
      title: form.title,
      publisher: form.publisher,
      universe: form.universe,
      description: form.description,
      readingDirection: form.readingDirection,
      coverFileId: form.coverFileId || undefined,
      // Only send the lock when it differs so saving other changes keeps the
      // backend's automatic lock-on-edit behaviour.
      metadataLocked:
        draft && draft.metadataLocked !== (series.metadataLocked ?? false)
          ? draft.metadataLocked
          : undefined,
    });
    await query.refetch();
    setDraft(null);
    setEditing(false);
    toast.push("Series updated", "success");
  };

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <button className="btn btn-ghost btn-sm mb-4" onClick={() => navigate(-1)}>
        <Icon name="arrowLeft" className="size-3.5" /> Back
      </button>

      <div className="flex flex-wrap gap-6">
        <Cover
          fileId={series.coverFileId}
          page={series.coverPage ?? 0}
          title={series.title}
          className="h-64 w-44 shrink-0 shadow-2xl shadow-black/50"
        />
        <div className="min-w-0 flex-1">
          <p className="text-[11px] uppercase tracking-[0.18em] text-muted">
            {[series.publisher, series.universe].filter(Boolean).join(" · ") || "Series"}
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">{series.title}</h1>
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-muted">
            <span>{pluralize(series.issueCount, "issue")}</span>
            <span>·</span>
            <span>{series.readCount} read</span>
            {series.year && (
              <>
                <span>·</span>
                <span>{series.year}</span>
              </>
            )}
            {series.universe && (
              <Link
                to={`/universes/${encodeURIComponent(series.universe)}`}
                className="chip chip-accent hover:border-[var(--accent)]"
              >
                {series.universe}
              </Link>
            )}
          </div>
          <div className="mt-4 max-w-md">
            <ProgressBar value={series.issueCount ? series.readCount / series.issueCount : 0} />
          </div>
          <p className="mt-4 max-w-xl text-sm leading-relaxed text-muted">
            {series.description || "No description yet. Add notes about this run, continuity or reading context."}
          </p>
          <div className="mt-5 flex flex-wrap gap-2">
            {next && (
              <Link to={`/reader/${next.id}`} className="btn btn-primary">
                <Icon name="play" className="size-3.5" />
                {series.readCount > 0 ? "Continue reading" : "Start reading"}
              </Link>
            )}
            <button className="btn" onClick={() => setEditing((current) => !current)}>
              <Icon name="edit" className="size-3.5" /> Edit series
            </button>
            {series.metadataLocked && (
              <span className="chip" title="Series metadata is locked against rescans">
                <Icon name="lock" className="size-3" /> Locked
              </span>
            )}
            <AddToCollectionMenu seriesId={seriesId} label="Add series to collection" />
            <div className="flex overflow-hidden rounded-lg border border-[var(--line)]">
              <button
                className="btn btn-sm rounded-none border-0"
                disabled={bulkBusy}
                onClick={() =>
                  applyToSeries(
                    () => api.setSeriesStatus(seriesId, "completed"),
                    (count) => `Marked ${pluralize(count, "issue")} read`,
                  )
                }
                title="Mark every issue in this series read"
              >
                <Icon name="check" className="size-3.5" />
                Mark all read
              </button>
              <button
                className="btn btn-sm rounded-none border-0 border-l border-[var(--line)]"
                disabled={bulkBusy}
                onClick={() =>
                  applyToSeries(
                    () => api.setSeriesStatus(seriesId, "unread"),
                    (count) => `Marked ${pluralize(count, "issue")} unread`,
                  )
                }
                title="Mark every issue in this series unread"
              >
                Mark all unread
              </button>
            </div>
            <select
              className="input h-8 w-auto text-xs"
              value=""
              disabled={bulkBusy}
              aria-label="Set reading direction for every issue in this series"
              onChange={(event) => {
                const value = event.currentTarget.value;
                if (!value) return;
                const direction = value === "default" ? null : (value as "ltr" | "rtl");
                void applyToSeries(
                  () => api.setSeriesDirection(seriesId, direction),
                  (count) =>
                    `Set ${pluralize(count, "file")} to ${
                      direction === null ? "the series default" : direction.toUpperCase()
                    }`,
                );
                event.currentTarget.value = "";
              }}
            >
              <option value="">Direction for all issues…</option>
              <option value="ltr">Left to right</option>
              <option value="rtl">Right to left (manga)</option>
              <option value="default">Clear per-file override</option>
            </select>
          </div>
        </div>
      </div>

      {editing && (
        <div className="mt-6 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-4 animate-rise">
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <label>
              <span className="label">Title</span>
              <input
                className="input"
                value={form.title}
                onChange={(event) => setForm({ ...form, title: event.currentTarget.value })}
              />
            </label>
            <label>
              <span className="label">Publisher</span>
              <input
                className="input"
                value={form.publisher}
                onChange={(event) => setForm({ ...form, publisher: event.currentTarget.value })}
              />
            </label>
            <label>
              <span className="label">Universe</span>
              <input
                className="input"
                value={form.universe}
                onChange={(event) => setForm({ ...form, universe: event.currentTarget.value })}
              />
            </label>
            <label>
              <span className="label">Reading direction</span>
              <select
                className="input"
                value={form.readingDirection}
                onChange={(event) =>
                  setForm({
                    ...form,
                    readingDirection: event.currentTarget.value as "" | "ltr" | "rtl",
                  })
                }
              >
                <option value="">Use app default</option>
                <option value="ltr">Left to right</option>
                <option value="rtl">Right to left (manga)</option>
              </select>
            </label>
            <label className="sm:col-span-3">
              <span className="label">Cover image</span>
              <div className="flex items-center gap-2">
                <Cover
                  fileId={form.coverFileId || null}
                  title={form.title}
                  className="h-16 w-11 shrink-0"
                  width={90}
                  rounded="rounded"
                />
                <select
                  className="input h-8 text-xs"
                  value={form.coverFileId}
                  onChange={(event) => setForm({ ...form, coverFileId: event.currentTarget.value })}
                  onFocus={() => setPickingCover(true)}
                >
                  <option value="">Use first issue cover</option>
                  {(coverOptions.data?.issues ?? []).map((issue) => (
                    <option key={issue.id} value={issue.fileId ?? ""}>
                      {issue.seriesTitle ? `${issue.seriesTitle} · ` : ""}
                      {issue.title}
                    </option>
                  ))}
                </select>
              </div>
              <p className="mt-1 text-[10px] text-muted">
                Overrides the cover used across the library, series list and shelf.
              </p>
            </label>
            <label className="sm:col-span-3">
              <span className="label">Description</span>
              <textarea
                className="input min-h-20"
                value={form.description}
                onChange={(event) => setForm({ ...form, description: event.currentTarget.value })}
              />
            </label>
            <label className="flex items-start gap-2 sm:col-span-3">
              <input
                type="checkbox"
                className="mt-0.5"
                checked={form.metadataLocked}
                onChange={(event) =>
                  setForm({ ...form, metadataLocked: event.currentTarget.checked })
                }
              />
              <span>
                <span className="text-[11px] font-medium">Protect manual edits (metadata lock)</span>
                <span className="mt-0.5 block text-[10px] text-muted">
                  Locked series metadata is never overwritten by rescans or metadata refreshes.
                  Saving any edit locks it automatically.
                </span>
              </span>
            </label>
          </div>
          <div className="mt-3 flex justify-end gap-2">
            <button className="btn btn-sm" onClick={() => setEditing(false)}>
              Cancel
            </button>
            <button className="btn btn-sm btn-primary" onClick={save}>
              Save
            </button>
          </div>
        </div>
      )}

      {series.missing.length > 0 && (
        <section className="mt-6 rounded-xl border border-amber-400/25 bg-amber-400/5 p-4">
          <div className="flex items-center gap-2">
            <Icon name="info" className="size-4 text-amber-300" />
            <h2 className="font-display text-sm font-semibold tracking-tight">
              Missing issues ({series.missing.length})
            </h2>
          </div>
          <p className="mt-1 text-[11px] text-muted">
            Gaps in this series' numbering. Mark one as not required if it was
            intentionally skipped, or ignore the warning if this run simply never
            had that issue.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            {series.missing.map((entry) => (
              <span
                key={entry.number}
                className="group inline-flex items-center gap-1.5 rounded-lg border border-[var(--line)] bg-[var(--panel-raised)] px-2 py-1 text-[11px]"
              >
                <span className="font-mono font-semibold">#{entry.number}</span>
                <button
                  className="text-muted hover:text-amber-300"
                  title="Mark as not required"
                  onClick={() => void setMissing(entry.number, "not-required")}
                >
                  <Icon name="x" className="size-3" />
                </button>
              </span>
            ))}
          </div>
        </section>
      )}

      <CharactersPanel
        seriesId={seriesId}
        characters={characters.data ?? []}
        loading={characters.isLoading}
      />

      <h2 className="mt-8 mb-3 font-display text-sm font-semibold tracking-tight text-[var(--text)]">
        Issues
      </h2>

      {series.issues.length === 0 ? (
        <EmptyState
          title="No issues in this series"
          body="Assign issues to this series from the library metadata editor."
        />
      ) : (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          {series.issues
            .filter((issue) => !issue.variantOf)
            .map((issue) => {
              const variants = series.issues.filter((entry) => entry.variantOf === issue.id);
              return (
                <div key={issue.id}>
                  <IssueRow issue={issue} />
                  {variants.map((variant) => (
                    <IssueRow
                      key={variant.id}
                      issue={variant}
                      variant
                      compareWith={issue.id}
                    />
                  ))}
                </div>
              );
            })}
        </div>
      )}
    </div>
  );
}

function IssueRow({
  issue,
  variant = false,
  compareWith,
}: {
  issue: SeriesDetailIssue;
  variant?: boolean;
  compareWith?: string;
}) {
  return (
    <div
      className={cx(
        "flex items-center gap-3 border-b border-[var(--line)] px-3 py-2.5 text-sm last:border-0 hover:bg-[var(--surface-2)]",
        variant && "bg-[var(--surface-1)] pl-10",
      )}
    >
      <span className="w-10 shrink-0 text-center font-mono text-xs text-muted">
        {variant ? "└" : issue.number ? `#${issue.number}` : "—"}
      </span>
      <Cover
        fileId={issue.fileId}
        title={issue.title}
        className="h-11 w-8 shrink-0"
        width={80}
        rounded="rounded"
      />
      <div className="min-w-0 flex-1">
        <p className="truncate font-medium">{issue.title}</p>
        <p className="truncate text-[11px] text-muted">
          {[issue.edition, issue.releaseDate ? formatDate(issue.releaseDate) : null, issue.writer]
            .filter(Boolean)
            .join(" · ") || "No metadata"}
        </p>
      </div>
      {issue.metadataLocked && (
        <span className="text-muted" title="Metadata locked — manual edits are protected">
          <Icon name="lock" className="size-3" />
        </span>
      )}
      {issue.edition && <span className="chip chip-accent hidden sm:inline-flex">{issue.edition}</span>}
      <div className="hidden w-28 shrink-0 items-center gap-2 sm:flex">
        <ProgressBar value={progressRatio(issue)} />
      </div>
      <StatusBadge status={issue.status} />
      {compareWith && (
        <Link
          to={`/reader/${compareWith}?compare=${issue.id}`}
          className="btn btn-ghost btn-sm"
          title="Compare this variant with the base issue side by side"
          aria-label={`Compare ${issue.title} with the base issue`}
        >
          <Icon name="panel" className="size-3.5" />
        </Link>
      )}
      <Link
        to={`/reader/${issue.id}`}
        className="grid size-7 place-items-center rounded text-muted hover:text-[var(--text-strong)]"
        aria-label={`Read ${issue.title}`}
      >
        <Icon name="chevronRight" className="size-4 shrink-0" />
      </Link>
    </div>
  );
}
