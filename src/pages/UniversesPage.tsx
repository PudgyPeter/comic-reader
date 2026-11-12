import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, ProgressBar, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { pluralize } from "../lib/utils";

function yearRange(first: string | null, last: string | null) {
  const start = first?.slice(0, 4) ?? "";
  const end = last?.slice(0, 4) ?? "";
  if (!start && !end) return "Year unknown";
  if (start === end || !end) return start;
  if (!start) return end;
  return `${start}–${end}`;
}

export default function UniversesPage() {
  const query = useQuery({ queryKey: ["universes"], queryFn: api.universes });
  const universes = query.data ?? [];

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <div className="mb-5">
        <h1 className="font-display text-2xl font-semibold tracking-tight">Universes</h1>
        <p className="mt-1 max-w-2xl text-[13px] text-muted">
          Groupings built from the universe field on series. Set it while editing a series and
          the shared characters, issues and publishers collect here; nothing is inferred from
          folder names.
        </p>
      </div>

      {query.isLoading && (
        <div className="grid h-48 place-items-center">
          <Spinner />
        </div>
      )}

      {query.isError && (
        <EmptyState
          title="Universes could not load"
          body={(query.error as Error).message}
          icon={<Icon name="info" className="size-5" />}
        />
      )}

      {query.isSuccess && universes.length === 0 && (
        <EmptyState
          title="No universes yet"
          body="Open a series, edit it, and give it a universe such as “Sonic the Hedgehog” or “Marvel 616”. Series that share the label are grouped here."
          icon={<Icon name="layers" className="size-5" />}
        />
      )}

      {universes.length > 0 && (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {universes.map((universe) => (
            <Link
              key={universe.name}
              to={`/universes/${encodeURIComponent(universe.name)}`}
              className="group flex gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3 transition-colors hover:border-[var(--line-strong)]"
            >
              <Cover
                fileId={universe.coverFileId}
                page={universe.coverPage ?? 0}
                title={universe.name}
                className="aspect-[2/3] w-20 shrink-0"
              />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium" title={universe.name}>
                  {universe.name}
                </p>
                <p className="mt-0.5 truncate text-[11px] text-muted">
                  {universe.publishers.join(" · ") || "Publisher unknown"}
                </p>
                <p className="mt-1 text-[11px] text-muted">{yearRange(universe.yearFirst, universe.yearLast)}</p>
                <div className="mt-2">
                  <ProgressBar
                    value={universe.issueCount ? universe.readCount / universe.issueCount : 0}
                  />
                </div>
                <p className="mt-1 text-[10px] text-muted">
                  {pluralize(universe.seriesCount, "series")} ·{" "}
                  {pluralize(universe.issueCount, "issue")} ·{" "}
                  {pluralize(universe.characterCount, "character")}
                </p>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
