import { useEffect, useMemo, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { useInfiniteQuery, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, Modal, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { cx, formatDate, progressRatio } from "../lib/utils";
import { useToast } from "../components/Toast";
import type { SearchFilter, SearchResult } from "../lib/types";

export default function SearchPage() {
  const [params, setParams] = useSearchParams();
  const initial = params.get("q") ?? "";
  const [query, setQuery] = useState(initial);
  const [debounced, setDebounced] = useState(initial);
  const [saveOpen, setSaveOpen] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [publisher, setPublisher] = useState("");
  const [seriesId, setSeriesId] = useState("");
  const [status, setStatus] = useState("");
  const [format, setFormat] = useState("");
  const [yearFrom, setYearFrom] = useState("");
  const [yearTo, setYearTo] = useState("");
  const [sort, setSort] = useState("relevance");
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

  const options = useQuery({ queryKey: ["filter-options"], queryFn: api.filterOptions });
  const recent = useQuery({ queryKey: ["recent-searches"], queryFn: api.recentSearches });

  const activeFilters = useMemo(() => {
    const filter: SearchFilter = { query: debounced.trim(), sort, limit: 40 };
    if (publisher) filter.publisher = publisher;
    if (seriesId) filter.seriesId = seriesId;
    if (status) filter.status = status;
    if (format) filter.format = format;
    if (yearFrom) filter.yearFrom = Number(yearFrom);
    if (yearTo) filter.yearTo = Number(yearTo);
    return filter;
  }, [debounced, publisher, seriesId, status, format, yearFrom, yearTo, sort]);

  const filterCount = [publisher, seriesId, status, format, yearFrom || yearTo].filter(
    Boolean,
  ).length;

  const results = useInfiniteQuery({
    queryKey: ["search", activeFilters],
    queryFn: ({ pageParam }) =>
      api.searchFiltered({ ...activeFilters, cursor: pageParam ?? null }),
    initialPageParam: null as string | null,
    getNextPageParam: (last: SearchResult) => last.nextCursor,
    enabled: debounced.trim().length > 0,
  });

  const suggestions = useQuery({
    queryKey: ["suggestions", debounced],
    queryFn: () => api.searchSuggestions(debounced),
    enabled: debounced.trim().length >= 2,
  });

  const saved = useQuery({ queryKey: ["saved-searches"], queryFn: api.savedSearches });

  const issues = useMemo(
    () => results.data?.pages.flatMap((page) => page.issues) ?? [],
    [results.data],
  );
  const series = results.data?.pages[0]?.series ?? [];
  const snippets = useMemo(() => {
    const merged: Record<string, string> = {};
    for (const page of results.data?.pages ?? []) {
      Object.assign(merged, page.snippets);
    }
    return merged;
  }, [results.data]);
  const total = issues.length + series.length;

  useEffect(() => {
    if (!results.isSuccess) return;
    void queryClient.invalidateQueries({ queryKey: ["recent-searches"] });
  }, [results.isSuccess, results.data?.pages.length, queryClient]);

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
          className={cx("btn", filterCount > 0 && "border-[var(--accent)]/50 text-[var(--accent)]")}
          onClick={() => setFiltersOpen((current) => !current)}
          aria-expanded={filtersOpen}
        >
          <Icon name="filter" className="size-3.5" /> Filters
          {filterCount > 0 && (
            <span className="rounded-full bg-[var(--accent)] px-1.5 text-[10px] font-semibold text-black">
              {filterCount}
            </span>
          )}
        </button>
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

      {filtersOpen && (
        <div className="mt-3 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3">
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
            <label>
              <span className="label">Publisher</span>
              <select
                className="input h-8 text-xs"
                value={publisher}
                onChange={(event) => setPublisher(event.currentTarget.value)}
              >
                <option value="">Any</option>
                {(options.data?.publishers ?? []).map((entry) => (
                  <option key={entry} value={entry}>
                    {entry}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span className="label">Series</span>
              <select
                className="input h-8 text-xs"
                value={seriesId}
                onChange={(event) => setSeriesId(event.currentTarget.value)}
              >
                <option value="">Any</option>
                {(options.data?.series ?? []).map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.title}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span className="label">Status</span>
              <select
                className="input h-8 text-xs"
                value={status}
                onChange={(event) => setStatus(event.currentTarget.value)}
              >
                <option value="">Any</option>
                <option value="unread">Unread</option>
                <option value="in-progress">In progress</option>
                <option value="completed">Completed</option>
              </select>
            </label>
            <label>
              <span className="label">Format</span>
              <select
                className="input h-8 text-xs"
                value={format}
                onChange={(event) => setFormat(event.currentTarget.value)}
              >
                <option value="">Any</option>
                {(options.data?.formats ?? []).map((entry) => (
                  <option key={entry} value={entry}>
                    {entry.toUpperCase()}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span className="label">Year from</span>
              <input
                className="input h-8 text-xs"
                inputMode="numeric"
                placeholder={options.data?.yearMin?.toString() ?? ""}
                value={yearFrom}
                onChange={(event) =>
                  setYearFrom(event.currentTarget.value.replace(/[^0-9]/g, ""))
                }
              />
            </label>
            <label>
              <span className="label">Year to</span>
              <input
                className="input h-8 text-xs"
                inputMode="numeric"
                placeholder={options.data?.yearMax?.toString() ?? ""}
                value={yearTo}
                onChange={(event) =>
                  setYearTo(event.currentTarget.value.replace(/[^0-9]/g, ""))
                }
              />
            </label>
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <label className="flex items-center gap-2 text-[11px] text-muted">
              Sort
              <select
                className="input h-7 w-auto text-xs"
                value={sort}
                onChange={(event) => setSort(event.currentTarget.value)}
              >
                <option value="relevance">Relevance</option>
                <option value="series">Series and number</option>
                <option value="title">Title</option>
                <option value="recent">Recently read</option>
              </select>
            </label>
            {filterCount > 0 && (
              <button
                className="btn btn-sm btn-ghost"
                onClick={() => {
                  setPublisher("");
                  setSeriesId("");
                  setStatus("");
                  setFormat("");
                  setYearFrom("");
                  setYearTo("");
                }}
              >
                Clear filters
              </button>
            )}
            <span className="ml-auto text-[10px] text-muted">
              Filters combine with the text query. Snippets are hidden for spoiler-protected
              issues.
            </span>
          </div>
        </div>
      )}

      {recent.data && recent.data.length > 0 && !debounced.trim() && (
        <div className="mt-4 flex flex-wrap items-center gap-1.5">
          <span className="text-[11px] uppercase tracking-wider text-muted">Recent</span>
          {recent.data.map((entry) => (
            <button
              key={entry.query}
              className="chip hover:border-[var(--line-strong)]"
              onClick={() => setQuery(entry.query)}
            >
              {entry.query}
            </button>
          ))}
          <button
            className="btn btn-ghost btn-sm ml-auto"
            onClick={async () => {
              await api.clearSearchHistory();
              await queryClient.invalidateQueries({ queryKey: ["recent-searches"] });
            }}
          >
            Clear
          </button>
        </div>
      )}

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

        {results.isError && (
          <EmptyState
            title="Search failed"
            body={String(results.error)}
          />
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
                    {snippets[issue.id] && (
                      <p className="mt-0.5 line-clamp-2 text-[11px] text-[var(--text)]/80">
                        {snippets[issue.id]}
                      </p>
                    )}
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

        {results.hasNextPage && (
          <div className="mt-4 flex justify-center">
            <button
              className="btn"
              onClick={() => void results.fetchNextPage()}
              disabled={results.isFetchingNextPage}
            >
              {results.isFetchingNextPage ? <Spinner className="size-3.5" /> : null}
              Load more results
            </button>
          </div>
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
