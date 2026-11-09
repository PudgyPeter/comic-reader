import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import type { IssueCard, IssuePatch, LibraryFilter } from "../lib/types";
import { cx, formatDate, formatRelative, pluralize, progressRatio } from "../lib/utils";
import { useProgressiveList } from "../hooks/useProgressiveList";
import { useImportActivity } from "../store/importActivity";
import { Cover, EmptyState, Modal, ProgressBar, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { useSettings } from "../store/settings";
import { IssueEditor } from "../components/IssueEditor";
import { useToast } from "../components/Toast";
import { AddToCollectionMenu } from "../components/AddToCollection";

const SORTS = [
  { value: "recent", label: "Recently read" },
  { value: "series", label: "Series" },
  { value: "title", label: "Title" },
  { value: "number", label: "Issue number" },
  { value: "release", label: "Release date" },
  { value: "added", label: "Recently added" },
];

const STATUSES = [
  { value: "", label: "All statuses" },
  { value: "unread", label: "Unread" },
  { value: "in-progress", label: "In progress" },
  { value: "completed", label: "Completed" },
];

export default function LibraryPage() {
  const settings = useSettings((state) => state.settings);
  const updateSettings = useSettings((state) => state.update);
  const toast = useToast();
  const beginActivity = useImportActivity((state) => state.start);
  const endActivity = useImportActivity((state) => state.end);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("");
  const [universe, setUniverse] = useState("");
  const [publisher, setPublisher] = useState("");
  const [sort, setSort] = useState(settings.librarySort || "recent");
  const [editing, setEditing] = useState<IssueCard | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [rescanning, setRescanning] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [bulkOpen, setBulkOpen] = useState(false);

  const filter: LibraryFilter = useMemo(
    () => ({
      search,
      status: status || null,
      universe: universe || null,
      publisher: publisher || null,
      sort,
      descending: false,
    }),
    [search, status, universe, publisher, sort],
  );

  const snapshot = useQuery({
    queryKey: ["library", filter],
    queryFn: () => api.library(filter),
  });

  const options = useQuery({
    queryKey: ["filter-options"],
    queryFn: api.filterOptions,
  });

  const collections = useQuery({
    queryKey: ["collections"],
    queryFn: api.collections,
  });

  const layout = settings.libraryLayout;
  const issues = snapshot.data?.issues ?? [];
  const series = snapshot.data?.series ?? [];
  const displaySeries = !status && !universe && !publisher && !search;
  const selectedSet = useMemo(() => new Set(selected), [selected]);

  const rescan = async () => {
    setRescanning(true);
    beginActivity("Rescanning library");
    try {
      const results = await api.rescanLibrary();
      const imported = results.reduce((sum, entry) => sum + entry.filesImported, 0);
      toast.push(`Rescan complete · ${pluralize(imported, "new file")}`, "success");
      await snapshot.refetch();
    } catch (error) {
      if (error instanceof Error && error.name === "cancelled") {
        toast.push("Rescan cancelled", "info");
      } else {
        toast.push(String(error), "error");
      }
    } finally {
      setRescanning(false);
      endActivity();
    }
  };

  const toggleSelected = (id: string) => {
    setSelected((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
  };

  const empty = snapshot.isSuccess && issues.length === 0 && series.length === 0;

  const { visible: visibleIssues, sentinelRef, hasMore } = useProgressiveList(
    issues,
    `${layout}|${search}|${status}|${universe}|${publisher}|${sort}`,
  );

  return (
    <div className="mx-auto w-full max-w-[1500px] px-6 py-6">
      <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">Library</h1>
          <p className="mt-1 text-[13px] text-muted">
            {snapshot.data
              ? `${pluralize(snapshot.data.totals.issues, "issue")} · ${pluralize(
                  snapshot.data.totals.series,
                  "series",
                  "series",
                )} · ${snapshot.data.totals.completed} completed`
              : "Loading your shelf…"}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="btn btn-sm" onClick={() => setShowImport(true)}>
            <Icon name="folder" className="size-3.5" />
            Library folders
          </button>
          <button className="btn btn-sm" onClick={rescan} disabled={rescanning}>
            <Icon name="refresh" className={cx("size-3.5", rescanning && "animate-spin")} />
            Rescan
          </button>
          <div className="flex overflow-hidden rounded-lg border border-[var(--line)]">
            {(["grid", "list"] as const).map((mode) => (
              <button
                key={mode}
                onClick={() => {
                  void updateSettings({ libraryLayout: mode });
                }}
                className={cx(
                  "grid size-8 place-items-center",
                  layout === mode ? "bg-[var(--surface-3)] text-[var(--text-strong)]" : "text-muted hover:bg-[var(--surface-2)]",
                )}
                aria-label={`${mode} layout`}
                aria-pressed={layout === mode}
              >
                <Icon name={mode === "grid" ? "grid" : "list"} className="size-3.5" />
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="mb-6 flex flex-wrap items-center gap-2">
        <div className="relative min-w-[240px] flex-1">
          <Icon
            name="search"
            className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted"
          />
          <input
            value={search}
            onChange={(event) => setSearch(event.currentTarget.value)}
            placeholder="Filter by title, writer, number, description…"
            className="input pl-9"
            aria-label="Filter library"
          />
        </div>
        <select
          value={status}
          onChange={(event) => setStatus(event.currentTarget.value)}
          className="input w-auto"
          aria-label="Reading status"
        >
          {STATUSES.map((entry) => (
            <option key={entry.value} value={entry.value}>
              {entry.label}
            </option>
          ))}
        </select>
        <select
          value={universe}
          onChange={(event) => setUniverse(event.currentTarget.value)}
          className="input w-auto"
          aria-label="Universe"
        >
          <option value="">All universes</option>
          {(options.data?.universes ?? []).map((entry) => (
            <option key={entry} value={entry}>
              {entry}
            </option>
          ))}
        </select>
        <select
          value={publisher}
          onChange={(event) => setPublisher(event.currentTarget.value)}
          className="input w-auto"
          aria-label="Publisher"
        >
          <option value="">All publishers</option>
          {(options.data?.publishers ?? []).map((entry) => (
            <option key={entry} value={entry}>
              {entry}
            </option>
          ))}
        </select>
        <select
          value={sort}
          onChange={(event) => {
            setSort(event.currentTarget.value);
            void updateSettings({ librarySort: event.currentTarget.value });
          }}
          className="input w-auto"
          aria-label="Sort"
        >
          {SORTS.map((entry) => (
            <option key={entry.value} value={entry.value}>
              {entry.label}
            </option>
          ))}
        </select>
      </div>

      {selected.length > 0 && (
        <div className="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-[var(--accent)]/30 bg-[var(--accent)]/8 px-3 py-2 text-xs">
          <span className="font-semibold">{selected.length} selected</span>
          <span className="text-muted">·</span>
          <button
            className="btn btn-sm"
            onClick={() => setBulkOpen(true)}
          >
            Edit metadata
          </button>
          <AddToCollectionMenu issueIds={selected} label="Add to collection" />
          <button
            className="btn btn-sm"
            onClick={async () => {
              for (const id of selected) {
                await api.setIssueStatus(id, "completed");
              }
              setSelected([]);
              await snapshot.refetch();
              toast.push("Marked selected issues completed", "success");
            }}
          >
            Mark read
          </button>
          <button
            className="btn btn-sm"
            onClick={async () => {
              for (const id of selected) {
                await api.setIssueStatus(id, "unread");
              }
              setSelected([]);
              await snapshot.refetch();
              toast.push("Marked selected issues unread", "success");
            }}
          >
            Mark unread
          </button>
          <button className="btn btn-sm btn-ghost" onClick={() => setSelected([])}>
            Clear
          </button>
        </div>
      )}

      {snapshot.isLoading && (
        <div className="grid h-64 place-items-center">
          <Spinner />
        </div>
      )}

      {empty && (
        <EmptyState
          title="Your shelf is empty"
          body="Point the app at a folder of CBZ archives. Files are indexed in place — nothing is copied or modified."
          action={
            <button className="btn btn-primary" onClick={() => setShowImport(true)}>
              <Icon name="folder" className="size-3.5" /> Add library folder
            </button>
          }
        />
      )}

      {!empty && displaySeries && series.length > 0 && (
        <section className="mb-8">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="font-display text-sm font-semibold tracking-tight text-[var(--text)]">
              Continue your series
            </h2>
            <Link to="/series" className="text-xs text-muted hover:text-[var(--text-strong)]">
              All series →
            </Link>
          </div>
          <div className="flex gap-4 overflow-x-auto pb-2">
            {series.slice(0, 12).map((entry) => (
              <Link
                key={entry.id}
                to={`/series/${entry.id}`}
                className="group w-[132px] shrink-0"
              >
                <div className="relative">
                  <Cover
                    fileId={entry.coverFileId}
                    page={entry.coverPage ?? 0}
                    title={entry.title}
                    className="aspect-[2/3] w-full transition-transform duration-300 group-hover:-translate-y-1"
                  />
                  {entry.missingCount > 0 && (
                    <span
                      className="chip chip-warn absolute left-1.5 top-1.5"
                      title={`${entry.missingCount} missing ${entry.missingCount === 1 ? "issue" : "issues"}`}
                    >
                      {entry.missingCount} missing
                    </span>
                  )}
                </div>
                <p className="mt-2 truncate text-xs font-medium">{entry.title}</p>
                <div className="mt-1.5">
                  <ProgressBar
                    value={entry.issueCount ? entry.readCount / entry.issueCount : 0}
                  />
                </div>
                <p className="mt-1 text-[10px] text-muted">
                  {entry.readCount}/{entry.issueCount} read
                </p>
              </Link>
            ))}
          </div>
        </section>
      )}

      {!empty && (collections.data?.length ?? 0) > 0 && !search && (
        <section className="mb-8">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="font-display text-sm font-semibold tracking-tight text-[var(--text)]">
              Collections
            </h2>
            <Link to="/collections" className="text-xs text-muted hover:text-[var(--text-strong)]">
              Manage →
            </Link>
          </div>
          <div className="flex flex-wrap gap-2">
            {(collections.data ?? []).map((collection) => (
              <Link
                key={collection.id}
                to={`/collections/${collection.id}`}
                className="group flex items-center gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] py-2 pl-2 pr-3 transition-colors hover:border-[var(--line-strong)]"
              >
                <Cover
                  fileId={collection.coverFileId}
                  title={collection.name}
                  className="h-11 w-8 shrink-0"
                  width={80}
                  rounded="rounded"
                />
                <div className="min-w-0">
                  <p className="truncate text-xs font-medium">{collection.name}</p>
                  <p className="text-[10px] text-muted">
                    {pluralize(collection.itemCount, "issue")}
                  </p>
                </div>
              </Link>
            ))}
          </div>
        </section>
      )}

      {!empty && layout === "grid" && (
        <div className="grid grid-cols-2 gap-x-4 gap-y-7 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {visibleIssues.map((issue) => (
            <IssueTile
              key={issue.id}
              issue={issue}
              selected={selectedSet.has(issue.id)}
              selectable={selected.length > 0}
              onSelect={() => toggleSelected(issue.id)}
              onEdit={() => setEditing(issue)}
            />
          ))}
        </div>
      )}
      {!empty && layout === "grid" && hasMore && (
        <div ref={sentinelRef} className="grid h-16 place-items-center">
          <Spinner />
        </div>
      )}

      {!empty && layout === "list" && (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          <table className="w-full text-left text-[13px]">
            <thead className="border-b border-[var(--line)] bg-[var(--surface-1)] text-[11px] uppercase tracking-wider text-muted">
              <tr>
                <th className="w-10 px-3 py-2.5" />
                <th className="px-3 py-2.5">Title</th>
                <th className="px-3 py-2.5">Series</th>
                <th className="px-3 py-2.5">#</th>
                <th className="px-3 py-2.5">Released</th>
                <th className="px-3 py-2.5">Progress</th>
                <th className="px-3 py-2.5">Last read</th>
              </tr>
            </thead>
            <tbody>
              {visibleIssues.map((issue) => (
                <tr key={issue.id} className="border-b border-[var(--line)] last:border-0 hover:bg-[var(--surface-1)]">
                  <td className="px-3 py-2">
                    <input
                      type="checkbox"
                      checked={selectedSet.has(issue.id)}
                      onChange={() => toggleSelected(issue.id)}
                      aria-label={`Select ${issue.title}`}
                      className="accent-[var(--accent)]"
                    />
                  </td>
                  <td className="px-3 py-2">
                    <Link to={`/reader/${issue.id}`} className="flex items-center gap-2.5">
                      <Cover
                        fileId={issue.fileId}
                        title={issue.title}
                        className="h-12 w-8 shrink-0"
                        width={80}
                        rounded="rounded"
                      />
                      <span className="truncate font-medium">{issue.title}</span>
                    </Link>
                  </td>
                  <td className="px-3 py-2 text-muted">{issue.seriesTitle ?? "—"}</td>
                  <td className="px-3 py-2 text-muted">#{issue.number || "?"}</td>
                  <td className="px-3 py-2 text-muted">{formatDate(issue.releaseDate)}</td>
                  <td className="px-3 py-2">
                    <div className="flex items-center gap-2">
                      <ProgressBar value={progressRatio(issue)} className="w-24" />
                      <span className="text-[11px] text-muted">
                        {Math.round(progressRatio(issue) * 100)}%
                      </span>
                    </div>
                  </td>
                  <td className="px-3 py-2 text-muted">{formatRelative(issue.lastReadAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {hasMore && (
            <div ref={sentinelRef} className="grid h-14 place-items-center border-t border-[var(--line)]">
              <Spinner />
            </div>
          )}
        </div>
      )}

      {snapshot.isSuccess && issues.length === 0 && !empty && (
        <EmptyState
          title="No matching issues"
          body="Adjust the filters or search text to see more of your library."
        />
      )}

      <IssueEditor
        issue={editing}
        onClose={() => setEditing(null)}
        onSaved={async () => {
          await snapshot.refetch();
          setEditing(null);
          toast.push("Metadata saved", "success");
        }}
      />

      <BulkEditor
        open={bulkOpen}
        issueIds={selected}
        onClose={() => setBulkOpen(false)}
        onSaved={async () => {
          setBulkOpen(false);
          setSelected([]);
          await snapshot.refetch();
          toast.push("Bulk metadata updated", "success");
        }}
      />

      {showImport && <LibraryFolders onClose={() => setShowImport(false)} />}
    </div>
  );
}

function IssueTile({
  issue,
  selected,
  selectable,
  onSelect,
  onEdit,
}: {
  issue: IssueCard;
  selected: boolean;
  selectable: boolean;
  onSelect: () => void;
  onEdit: () => void;
}) {
  const ratio = progressRatio(issue);
  return (
    <div className="group relative">
      <Link
        to={`/reader/${issue.id}`}
        className="block focusable rounded-lg"
        aria-label={`Open ${issue.title}`}
      >
        <div className="relative">
          <Cover
            fileId={issue.fileId}
            title={issue.title}
            className={cx(
              "aspect-[2/3] w-full shadow-lg shadow-black/40 transition-all duration-300 group-hover:-translate-y-1 group-hover:shadow-xl",
              selected && "accent-ring",
            )}
          />
          {ratio > 0 && ratio < 1 && (
            <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent p-2">
              <ProgressBar value={ratio} />
            </div>
          )}
          {issue.status === "completed" && (
            <span className="absolute right-1.5 top-1.5 grid size-5 place-items-center rounded-full bg-emerald-500/90 text-black">
              <Icon name="check" className="size-3" />
            </span>
          )}
          {issue.missing && (
            <span className="absolute left-1.5 top-1.5 chip border-rose-400/40 bg-rose-950/80 text-rose-200">
              Missing
            </span>
          )}
          {issue.spoilerProtected && (
            <span className="absolute left-1.5 top-1.5 chip" title="Spoiler protection active">
              <Icon name="eyeOff" className="size-3" />
            </span>
          )}
        </div>
      </Link>
      <div className="mt-2 flex items-start gap-1">
        <div className="min-w-0 flex-1">
          <p className="truncate text-xs font-medium leading-snug" title={issue.title}>
            {issue.title}
          </p>
          <p className="mt-0.5 truncate text-[10px] text-muted">
            {issue.seriesTitle ? `${issue.seriesTitle} · ` : ""}
            {issue.number ? `#${issue.number}` : "No number"}
          </p>
          <div className="mt-1.5 flex items-center gap-2">
            <StatusBadge status={issue.status} />
          </div>
        </div>
        <div className="flex flex-col gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
          <button
            onClick={(event) => {
              event.preventDefault();
              onEdit();
            }}
            className="rounded p-1 text-muted hover:bg-[var(--surface-3)] hover:text-[var(--text-strong)]"
            aria-label={`Edit ${issue.title}`}
          >
            <Icon name="edit" className="size-3" />
          </button>
          <button
            onClick={(event) => {
              event.preventDefault();
              onSelect();
            }}
            className={cx(
              "rounded p-1 hover:bg-[var(--surface-3)]",
              selectable ? "text-[var(--text-strong)]" : "text-muted hover:text-[var(--text-strong)]",
            )}
            aria-label={`Select ${issue.title}`}
          >
            <Icon name={selected ? "check" : "plus"} className="size-3" />
          </button>
        </div>
      </div>
    </div>
  );
}

function LibraryFolders({ onClose }: { onClose: () => void }) {
  const toast = useToast();
  const { data, isLoading, refetch } = useQuery({
    queryKey: ["library-roots"],
    queryFn: api.libraryRoots,
  });

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md rounded-2xl border border-[var(--line)] bg-[var(--panel)] p-5 shadow-2xl animate-pop">
        <h2 className="font-display text-base font-semibold">Library folders</h2>
        <p className="mt-1 text-xs text-muted">
          Archives are indexed in place. Removing a folder does not delete your files.
        </p>
        <div className="mt-4 space-y-2">
          {isLoading && <Spinner />}
          {data?.length === 0 && (
            <p className="rounded-lg border border-dashed border-[var(--line)] px-3 py-6 text-center text-xs text-muted">
              No folders yet. Use “Add folder” in the sidebar.
            </p>
          )}
          {data?.map((root) => (
            <div
              key={root.id}
              className="flex items-center gap-2 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 py-2"
            >
              <Icon name="folder" className="size-4 shrink-0 text-muted" />
              <span className="min-w-0 flex-1 truncate text-xs" title={root.path}>
                {root.path}
              </span>
              <span className="text-[10px] text-muted">{root.fileCount}</span>
              <button
                className="rounded p-1 text-muted hover:bg-[var(--surface-3)] hover:text-rose-300"
                aria-label={`Remove ${root.path}`}
                onClick={async () => {
                  await api.removeLibraryRoot(root.id);
                  await refetch();
                  window.dispatchEvent(new Event("library:changed"));
                  toast.push("Folder removed", "success");
                }}
              >
                <Icon name="trash" className="size-3.5" />
              </button>
            </div>
          ))}
        </div>
        <div className="mt-4 flex justify-end">
          <button className="btn" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

function BulkEditor({
  open,
  issueIds,
  onClose,
  onSaved,
}: {
  open: boolean;
  issueIds: string[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [patch, setPatch] = useState<IssuePatch>({});
  const [busy, setBusy] = useState(false);

  const field = (key: keyof IssuePatch, label: string, placeholder = "") => (
    <label>
      <span className="label">{label}</span>
      <input
        className="input"
        placeholder={placeholder}
        value={(patch[key] as string | undefined) ?? ""}
        onChange={(event) =>
          setPatch((current) => ({ ...current, [key]: event.currentTarget.value }))
        }
      />
    </label>
  );

  return (
    <Modal
      open={open}
      title={`Bulk edit ${issueIds.length} issue${issueIds.length === 1 ? "" : "s"}`}
      onClose={onClose}
      width="max-w-lg"
      footer={
        <>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            disabled={busy}
            onClick={async () => {
              const cleaned = Object.fromEntries(
                Object.entries(patch).filter(([, value]) => String(value ?? "").trim() !== ""),
              ) as IssuePatch;
              if (Object.keys(cleaned).length === 0) {
                toast.push("Nothing to change — fill at least one field", "info");
                return;
              }
              setBusy(true);
              try {
                const updated = await api.bulkUpdateIssues(issueIds, cleaned);
                const skipped = issueIds.length - updated.length;
                setPatch({});
                onSaved();
                toast.push(
                  skipped > 0
                    ? `Updated ${updated.length} · skipped ${skipped} locked`
                    : `Updated ${updated.length} issue${updated.length === 1 ? "" : "s"}`,
                  skipped > 0 ? "info" : "success",
                );
              } catch (error) {
                toast.push(String(error), "error");
              } finally {
                setBusy(false);
              }
            }}
          >
            Apply to selection
          </button>
        </>
      }
    >
      <p className="mb-3 text-[11px] text-muted">
        Empty fields are left untouched. Filled fields are written to every selected issue;
        issues with a metadata lock are skipped. Edited issues become locked automatically.
      </p>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {field("publisher", "Publisher")}
        {field("volume", "Volume")}
        {field("releaseDate", "Release date", "YYYY-MM-DD")}
        {field("writer", "Writer")}
        {field("artist", "Artist")}
        {field("coverArtist", "Cover artist")}
        {field("seriesTitle", "Assign to series (by name)")}
      </div>
    </Modal>
  );
}
