import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, Modal, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { cx, formatDate, progressRatio } from "../lib/utils";
import { useToast } from "../components/Toast";

export default function SearchPage() {
  const [params, setParams] = useSearchParams();
  const initial = params.get("q") ?? "";
  const [query, setQuery] = useState(initial);
  const [debounced, setDebounced] = useState(initial);
  const [saveOpen, setSaveOpen] = useState(false);
  const [saveName, setSaveName] = useState("");
  const toast = useToast();
  const queryClient = useQueryClient();

  useEffect(() => {
    const handle = window.setTimeout(() => setDebounced(query), 220);
    return () => window.clearTimeout(handle);
  }, [query]);

  useEffect(() => {
    if (debounced) setParams({ q: debounced }, { replace: true });
    else setParams({}, { replace: true });
  }, [debounced, setParams]);

  const results = useQuery({
    queryKey: ["search", debounced],
    queryFn: () => api.search(debounced, 120),
    enabled: debounced.trim().length > 0,
  });

  const suggestions = useQuery({
    queryKey: ["suggestions", debounced],
    queryFn: () => api.searchSuggestions(debounced),
    enabled: debounced.trim().length >= 2,
  });

  const saved = useQuery({ queryKey: ["saved-searches"], queryFn: api.savedSearches });

  const issues = results.data?.issues ?? [];
  const series = results.data?.series ?? [];
  const total = issues.length + series.length;

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <h1 className="font-display text-2xl font-semibold tracking-tight">Search</h1>
      <p className="mt-1 text-[13px] text-muted">
        Search titles, series, creators, publishers, dates and descriptions.
      </p>

      <div className="mt-4 flex gap-2">
        <div className="relative flex-1">
          <Icon
            name="search"
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted"
          />
          <input
            autoFocus
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="Search your library…"
            className="input h-10 pl-9"
          />
        </div>
        <button
          className="btn"
          disabled={!debounced.trim()}
          onClick={() => {
            setSaveName(debounced.trim());
            setSaveOpen(true);
          }}
        >
          <Icon name="star" className="size-3.5" /> Save search
        </button>
      </div>

      {suggestions.data && suggestions.data.length > 0 && query !== debounced && (
        <div className="mt-2 flex flex-wrap gap-1.5">
          {suggestions.data.slice(0, 8).map((suggestion) => (
            <button
              key={`${suggestion.kind}-${suggestion.value}`}
              className="chip hover:border-[var(--line-strong)]"
              onClick={() => setQuery(suggestion.value)}
            >
              {suggestion.value}
              <span className="text-[9px] uppercase tracking-wider text-muted">
                {suggestion.kind}
              </span>
            </button>
          ))}
        </div>
      )}

      {saved.data && saved.data.length > 0 && (
        <div className="mt-4">
          <p className="mb-1.5 text-[11px] uppercase tracking-wider text-muted">Saved searches</p>
          <div className="flex flex-wrap gap-1.5">
            {saved.data.map((entry) => (
              <span
                key={entry.id}
                className="group inline-flex items-center rounded-full border border-[var(--line)] bg-[var(--surface-1)] pl-2.5 text-[11px]"
              >
                <button className="py-1" onClick={() => setQuery(entry.query)}>
                  {entry.name}
                </button>
                <button
                  className="ml-1 rounded-full p-1 text-muted opacity-0 hover:text-rose-300 group-hover:opacity-100"
                  aria-label={`Delete saved search ${entry.name}`}
                  onClick={async () => {
                    await api.deleteSavedSearch(entry.id);
                    await queryClient.invalidateQueries({ queryKey: ["saved-searches"] });
                  }}
                >
                  <Icon name="x" className="size-2.5" />
                </button>
              </span>
            ))}
          </div>
        </div>
      )}

      <div className="mt-6">
        {results.isLoading && (
          <div className="grid h-40 place-items-center">
            <Spinner />
          </div>
        )}

        {debounced.trim() && results.isSuccess && total === 0 && (
          <EmptyState
            title="No results"
            body={`Nothing matched “${debounced}”. Try a broader term, a creator name, or an issue number.`}
          />
        )}

        {!debounced.trim() && (
          <EmptyState
            title="Search across your shelf"
            body="Results include issues and series. Character and location search arrives with the universe database."
          />
        )}

        {series.length > 0 && (
          <section className="mb-6">
            <h2 className="mb-2 text-[11px] uppercase tracking-wider text-muted">
              Series ({series.length})
            </h2>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {series.map((entry) => (
                <Link
                  key={entry.id}
                  to={`/series/${entry.id}`}
                  className="flex items-center gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-2.5 hover:border-[var(--line-strong)]"
                >
                  <Cover
                    fileId={entry.coverFileId}
                    page={entry.coverPage ?? 0}
                    title={entry.title}
                    className="h-16 w-11 shrink-0"
                    width={90}
                    rounded="rounded"
                  />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium">{entry.title}</p>
                    <p className="truncate text-[11px] text-muted">
                      {entry.issueCount} issues · {entry.readCount} read
                    </p>
                  </div>
                </Link>
              ))}
            </div>
          </section>
        )}

        {issues.length > 0 && (
          <section>
            <h2 className="mb-2 text-[11px] uppercase tracking-wider text-muted">
              Issues ({issues.length})
            </h2>
            <div className="overflow-hidden rounded-xl border border-[var(--line)]">
              {issues.map((issue) => (
                <Link
                  key={issue.id}
                  to={`/reader/${issue.id}`}
                  className="flex items-center gap-3 border-b border-[var(--line)] px-3 py-2 text-sm last:border-0 hover:bg-[var(--surface-1)]"
                >
                  <Cover
                    fileId={issue.fileId}
                    title={issue.title}
                    className="h-12 w-8 shrink-0"
                    width={80}
                    rounded="rounded"
                  />
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-medium">{issue.title}</p>
                    <p className="truncate text-[11px] text-muted">
                      {[issue.seriesTitle, issue.number ? `#${issue.number}` : null, formatDate(issue.releaseDate)]
                        .filter(Boolean)
                        .join(" · ")}
                    </p>
                  </div>
                  <span
                    className={cx(
                      "hidden text-[10px] text-muted sm:block",
                    )}
                  >
                    {Math.round(progressRatio(issue) * 100)}%
                  </span>
                  <StatusBadge status={issue.status} />
                </Link>
              ))}
            </div>
          </section>
        )}
      </div>

      <Modal
        open={saveOpen}
        title="Save this search"
        onClose={() => setSaveOpen(false)}
        footer={
          <>
            <button className="btn" onClick={() => setSaveOpen(false)}>
              Cancel
            </button>
            <button
              className="btn btn-primary"
              disabled={!saveName.trim()}
              onClick={async () => {
                await api.saveSearch(saveName.trim(), debounced);
                await queryClient.invalidateQueries({ queryKey: ["saved-searches"] });
                setSaveOpen(false);
                toast.push("Search saved", "success");
              }}
            >
              Save
            </button>
          </>
        }
      >
        <label>
          <span className="label">Name</span>
          <input
            className="input"
            value={saveName}
            onChange={(event) => setSaveName(event.currentTarget.value)}
            autoFocus
          />
        </label>
        <p className="mt-2 text-[11px] text-muted">Query: “{debounced}”</p>
      </Modal>
    </div>
  );
}
