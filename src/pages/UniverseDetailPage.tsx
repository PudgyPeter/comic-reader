import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, ProgressBar, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { CharacterDetailModal } from "../components/Characters";
import { pluralize } from "../lib/utils";

export default function UniverseDetailPage() {
  const { universeName } = useParams();
  const name = decodeURIComponent(universeName ?? "");
  const [openCharacter, setOpenCharacter] = useState<string | null>(null);

  const query = useQuery({
    queryKey: ["universe", name],
    queryFn: () => api.universeDetail(name),
    enabled: Boolean(name),
  });
  const universe = query.data;

  if (query.isLoading) {
    return (
      <div className="grid h-64 place-items-center">
        <Spinner />
      </div>
    );
  }

  if (query.isError || !universe) {
    return (
      <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
        <EmptyState
          title="Universe not found"
          body="The universe label may have been renamed on its series. Open Series and check the universe field."
          icon={<Icon name="info" className="size-5" />}
        />
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <Link
        to="/universes"
        className="mb-4 inline-flex items-center gap-1 text-[12px] text-muted hover:text-[var(--text)]"
      >
        <Icon name="arrowLeft" className="size-3.5" />
        Universes
      </Link>

      <header className="mb-6 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">{universe.name}</h1>
          <p className="mt-1 text-[13px] text-muted">
            {universe.publishers.join(" · ") || "Publisher unknown"}
          </p>
        </div>
        <div className="flex flex-wrap gap-2 text-[11px]">
          <span className="chip">{pluralize(universe.seriesCount, "series")}</span>
          <span className="chip">{pluralize(universe.issueCount, "issue")}</span>
          <span className="chip">{pluralize(universe.characterCount, "character")}</span>
          <span className="chip">{universe.readCount} read</span>
        </div>
      </header>

      <div className="mb-6 max-w-md">
        <ProgressBar value={universe.issueCount ? universe.readCount / universe.issueCount : 0} />
      </div>

      <section className="mb-8">
        <h2 className="mb-3 font-display text-sm font-semibold tracking-tight">Series</h2>
        {universe.series.length === 0 ? (
          <p className="text-[12px] text-muted">No series in this universe.</p>
        ) : (
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
            {universe.series.map((entry) => (
              <Link
                key={entry.id}
                to={`/series/${entry.id}`}
                className="group rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3 transition-colors hover:border-[var(--line-strong)]"
              >
                <Cover
                  fileId={entry.coverFileId}
                  page={entry.coverPage ?? 0}
                  title={entry.title}
                  className="aspect-[2/3] w-full"
                />
                <p className="mt-3 truncate text-sm font-medium" title={entry.title}>
                  {entry.title}
                </p>
                <div className="mt-2">
                  <ProgressBar value={entry.issueCount ? entry.readCount / entry.issueCount : 0} />
                </div>
                <p className="mt-1 text-[10px] text-muted">
                  {pluralize(entry.issueCount, "issue")} · {entry.readCount} read
                </p>
              </Link>
            ))}
          </div>
        )}
      </section>

      <section>
        <h2 className="mb-3 font-display text-sm font-semibold tracking-tight">Characters</h2>
        {universe.characters.length === 0 ? (
          <p className="text-[12px] text-muted">
            No characters recorded. Open a series in this universe and use “Rebuild roster”.
          </p>
        ) : (
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {universe.characters.map((character) => (
              <button
                key={character.id}
                onClick={() => setOpenCharacter(character.id)}
                className="rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3 text-left transition-colors hover:border-[var(--accent)]/40 hover:bg-[var(--surface-2)]"
              >
                <div className="flex items-baseline justify-between gap-2">
                  <span className="truncate text-[13px] font-medium">{character.name}</span>
                  <span className="shrink-0 text-[10px] text-muted">
                    {character.appearanceCount}×
                  </span>
                </div>
                <p className="mt-1 truncate text-[11px] text-muted">
                  {character.seriesTitle ?? "No series"}
                </p>
                <span className="mt-1 inline-flex items-center gap-1 text-[10px] text-muted">
                  <Icon name="link" className="size-3" />
                  {pluralize(character.relationshipCount, "relationship")}
                </span>
              </button>
            ))}
          </div>
        )}
      </section>

      <CharacterDetailModal
        characterId={openCharacter}
        onClose={() => setOpenCharacter(null)}
        onOpenCharacter={(id) => setOpenCharacter(id)}
      />
    </div>
  );
}
