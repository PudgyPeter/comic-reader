import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Modal, Cover, EmptyState, ProgressBar, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { pluralize } from "../lib/utils";
import { useToast } from "../components/Toast";

export default function SeriesPage() {
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const toast = useToast();
  const query = useQuery({
    queryKey: ["series-list"],
    queryFn: () => api.library({ sort: "series" }),
  });

  const series = (query.data?.series ?? []).filter((entry) =>
    entry.title.toLowerCase().includes(search.trim().toLowerCase()),
  );

  return (
    <div className="mx-auto w-full max-w-[1500px] px-6 py-6">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">Series</h1>
          <p className="mt-1 text-[13px] text-muted">
            Organise issues into runs, volumes and universes.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <input
            value={search}
            onChange={(event) => setSearch(event.currentTarget.value)}
            placeholder="Filter series"
            className="input w-56"
            aria-label="Filter series"
          />
          <button className="btn btn-sm btn-primary" onClick={() => setCreating(true)}>
            <Icon name="plus" className="size-3.5" />
            New series
          </button>
        </div>
      </div>

      {query.isLoading && (
        <div className="grid h-48 place-items-center">
          <Spinner />
        </div>
      )}

      {query.isSuccess && series.length === 0 && (
        <EmptyState
          title={search ? "No series match" : "No series yet"}
          body={
            search
              ? "Try a different search term."
              : "Import archives, or create a series manually and assign issues to it."
          }
        />
      )}

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
        {series.map((entry) => (
          <Link
            key={entry.id}
            to={`/series/${entry.id}`}
            className="group rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3 transition-colors hover:border-[var(--line-strong)]"
          >
            <div className="relative">
              <Cover
                fileId={entry.coverFileId}
                page={entry.coverPage ?? 0}
                title={entry.title}
                className="aspect-[2/3] w-full"
              />
              {entry.missingCount > 0 && (
                <span
                  className="chip chip-warn absolute left-1.5 top-1.5"
                  title={`${entry.missingCount} missing ${entry.missingCount === 1 ? "issue" : "issues"}`}
                >
                  <Icon name="info" className="size-3" />
                  {entry.missingCount} missing
                </span>
              )}
            </div>
            <div className="mt-3">
              <p className="truncate text-sm font-medium" title={entry.title}>
                {entry.title}
              </p>
              <p className="mt-0.5 truncate text-[11px] text-muted">
                {[entry.publisher, entry.universe, entry.year].filter(Boolean).join(" · ") ||
                  "No publisher"}
              </p>
              <div className="mt-2">
                <ProgressBar value={entry.issueCount ? entry.readCount / entry.issueCount : 0} />
              </div>
              <p className="mt-1 text-[10px] text-muted">
                {pluralize(entry.issueCount, "issue")} · {entry.readCount} read
              </p>
            </div>
          </Link>
        ))}
      </div>

      <CreateSeries
        open={creating}
        onClose={() => setCreating(false)}
        onCreated={async () => {
          setCreating(false);
          await query.refetch();
          toast.push("Series created", "success");
        }}
      />
    </div>
  );
}

export function CreateSeries({
  open,
  onClose,
  onCreated,
}: {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const [title, setTitle] = useState("");
  const [publisher, setPublisher] = useState("");
  const [universe, setUniverse] = useState("");
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const submit = async () => {
    if (!title.trim()) return;
    setBusy(true);
    try {
      await api.createSeries(title.trim(), publisher.trim() || undefined, universe.trim() || undefined);
      setTitle("");
      setPublisher("");
      setUniverse("");
      onCreated();
    } catch (error) {
      toast.push(String(error), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Create series"
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={submit} disabled={busy || !title.trim()}>
            Create
          </button>
        </>
      }
    >
      <div className="space-y-3">
        <label>
          <span className="label">Title</span>
          <input
            className="input"
            value={title}
            onChange={(event) => setTitle(event.currentTarget.value)}
            placeholder="Sonic the Hedgehog (IDW)"
            autoFocus
          />
        </label>
        <label>
          <span className="label">Publisher</span>
          <input
            className="input"
            value={publisher}
            onChange={(event) => setPublisher(event.currentTarget.value)}
            placeholder="IDW Publishing"
          />
        </label>
        <label>
          <span className="label">Universe</span>
          <input
            className="input"
            value={universe}
            onChange={(event) => setUniverse(event.currentTarget.value)}
            placeholder="Sonic — IDW continuity"
          />
        </label>
      </div>
    </Modal>
  );
}
