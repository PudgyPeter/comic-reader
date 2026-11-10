import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import { EmptyState, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { CharacterDetailModal } from "../components/Characters";
import { pluralize } from "../lib/utils";

const SOURCE_LABEL = {
  comic: "From the comics",
  web: "From the web",
  manual: "Written by you",
} as const;

export default function CharactersPage() {
  const [search, setSearch] = useState("");
  const [openId, setOpenId] = useState<string | null>(null);

  const query = useQuery({
    queryKey: ["characters"],
    queryFn: () => api.allCharacters(),
  });

  const characters = useMemo(() => {
    const needle = search.trim().toLowerCase();
    const all = query.data ?? [];
    if (!needle) return all;
    return all.filter((character) =>
      `${character.name} ${character.seriesTitle ?? ""}`.toLowerCase().includes(needle),
    );
  }, [query.data, search]);

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">Characters</h1>
          <p className="mt-1 max-w-2xl text-[13px] text-muted">
            Everyone the library has recorded, grouped by series. Descriptions come from the
            comics first; if an issue set does not say enough, look the character up on the web
            from their card. Nothing is fetched until you ask.
          </p>
        </div>
        <input
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
          placeholder="Filter characters"
          className="input w-56"
          aria-label="Filter characters"
        />
      </div>

      {query.isLoading && (
        <div className="grid h-48 place-items-center">
          <Spinner />
        </div>
      )}

      {query.isError && (
        <EmptyState
          title="Characters could not load"
          body={(query.error as Error).message}
          icon={<Icon name="info" className="size-5" />}
        />
      )}

      {query.isSuccess && characters.length === 0 && (
        <EmptyState
          title={search ? "No characters match" : "No characters recorded yet"}
          body={
            search
              ? "Try a different name or series."
              : "Open a series and use “Rebuild roster” to read the <Characters> tags in its ComicInfo.xml files."
          }
          icon={<Icon name="book" className="size-5" />}
        />
      )}

      {characters.length > 0 && (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          <table className="w-full border-collapse text-left text-[13px]">
            <thead className="bg-[var(--surface-2)] text-[11px] uppercase tracking-wide text-muted">
              <tr>
                <th className="px-4 py-2.5 font-medium">Character</th>
                <th className="px-4 py-2.5 font-medium">Series</th>
                <th className="px-4 py-2.5 font-medium">Description</th>
                <th className="px-4 py-2.5 text-right font-medium">Appearances</th>
              </tr>
            </thead>
            <tbody>
              {characters.map((character) => (
                <tr
                  key={character.id}
                  className="cursor-pointer border-t border-[var(--line)] bg-[var(--surface-1)] align-top transition-colors hover:bg-[var(--surface-2)]"
                  onClick={() => setOpenId(character.id)}
                >
                  <td className="px-4 py-3 font-medium">{character.name}</td>
                  <td className="px-4 py-3 text-muted">
                    {character.seriesId ? (
                      <Link
                        to={`/series/${character.seriesId}`}
                        className="hover:text-[var(--text)]"
                        onClick={(event) => event.stopPropagation()}
                      >
                        {character.seriesTitle ?? "Series"}
                      </Link>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td className="max-w-[420px] px-4 py-3">
                    <p className="line-clamp-2 text-[12px] text-muted">
                      {character.description || "No description yet"}
                    </p>
                    <span className="text-[10px] uppercase tracking-wide text-muted">
                      {SOURCE_LABEL[character.descriptionSource]}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-right tabular-nums text-muted">
                    {pluralize(character.appearanceCount, "issue")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <CharacterDetailModal characterId={openId} onClose={() => setOpenId(null)} />
    </div>
  );
}
