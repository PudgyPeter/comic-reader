import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import type { CharacterCard } from "../lib/types";
import { EmptyState, Modal, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";
import { RelationshipsPanel } from "../components/Relationships";
import { cx, pluralize } from "../lib/utils";

const SOURCE_LABEL: Record<CharacterCard["descriptionSource"], string> = {
  comic: "From the comics",
  web: "From the web",
  manual: "Written by you",
};

export function CharacterDetailModal({
  characterId,
  onClose,
  onOpenCharacter,
}: {
  characterId: string | null;
  onClose: () => void;
  onOpenCharacter?: (characterId: string) => void;
}) {
  const queryClient = useQueryClient();
  const toast = useToast();
  const [description, setDescription] = useState("");
  const [webBusy, setWebBusy] = useState(false);

  const query = useQuery({
    queryKey: ["character", characterId],
    queryFn: () => api.characterDetail(characterId!),
    enabled: Boolean(characterId),
  });

  const save = useMutation({
    mutationFn: (patch: Parameters<typeof api.updateCharacter>[1]) =>
      api.updateCharacter(characterId!, patch),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["character", characterId] });
      await queryClient.invalidateQueries({ queryKey: ["characters"] });
      toast.push("Character saved", "info");
      onClose();
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const detail = query.data;
  const draft = description || detail?.description || "";

  const fetchFromWeb = async () => {
    if (!detail) return;
    setWebBusy(true);
    try {
      const summary = await api.lookupCharacter(detail.name);
      setDescription(summary.extract);
      await api.updateCharacter(detail.id, {
        description: summary.extract,
        descriptionSource: "web",
        sourceUrl: summary.url,
      });
      await queryClient.invalidateQueries({ queryKey: ["character", detail.id] });
      await queryClient.invalidateQueries({ queryKey: ["characters"] });
      toast.push(`Pulled a summary from ${summary.source}`, "info");
    } catch (error) {
      toast.push((error as Error).message, "error");
    } finally {
      setWebBusy(false);
    }
  };

  return (
    <Modal
      open={Boolean(characterId)}
      title={detail ? detail.name : "Character"}
      onClose={onClose}
      width="max-w-xl"
      footer={
        <>
          <button
            className="btn btn-sm mr-auto"
            onClick={() => void fetchFromWeb()}
            disabled={webBusy || !detail}
            title="Fetch a public summary for this character (needs the network)"
          >
            <Icon name="search" className="size-3.5" />
            {webBusy ? "Looking up…" : "Look up on the web"}
          </button>
          <button className="btn" onClick={onClose}>
            Close
          </button>
          <button
            className="btn btn-primary"
            disabled={save.isPending || !detail}
            onClick={() => save.mutate({ description: draft, descriptionSource: "manual" })}
          >
            {save.isPending ? "Saving…" : "Save description"}
          </button>
        </>
      }
    >
      {query.isLoading && (
        <div className="grid h-32 place-items-center">
          <Spinner />
        </div>
      )}
      {query.isError && (
        <p className="text-sm text-red-400">{(query.error as Error).message}</p>
      )}
      {detail && (
        <div className="space-y-4">
          <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted">
            <span className="chip">{SOURCE_LABEL[detail.descriptionSource]}</span>
            <span>{pluralize(detail.appearanceCount, "appearance")}</span>
            <span>·</span>
            <span>{detail.readCount} read</span>
            <span>·</span>
            <span>{pluralize(detail.relationships.length, "relationship")}</span>
            {detail.descriptionUpdatedAt && (
              <>
                <span>·</span>
                <span>updated {detail.descriptionUpdatedAt.slice(0, 10)}</span>
              </>
            )}
          </div>

          {detail.sourceUrl && (
            <p className="text-[11px] text-muted">
              Source:{" "}
              <a className="underline hover:text-[var(--text)]" href={detail.sourceUrl}>
                {detail.sourceUrl}
              </a>
            </p>
          )}

          <label className="block">
            <span className="label">Description</span>
            <textarea
              className="input min-h-32 resize-y"
              value={draft}
              placeholder="Describe the character. “Look up on the web” fills this in when the comics do not carry enough text."
              onChange={(event) => setDescription(event.currentTarget.value)}
            />
          </label>

          <div>
            <p className="label mb-1">Appearances</p>
            {detail.appearances.length === 0 ? (
              <p className="text-[12px] text-muted">No recorded appearances.</p>
            ) : (
              <ul className="max-h-56 space-y-1 overflow-y-auto pr-1">
                {detail.appearances.map((appearance) => (
                  <li
                    key={appearance.issueId}
                    className="flex items-center gap-2 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-2.5 py-1.5 text-[12px]"
                  >
                    {appearance.spoilerProtected ? (
                      <span className="flex-1 text-muted">
                        <Icon name="eyeOff" className="mr-1 inline size-3" />
                        #{appearance.number} — hidden until you read this far
                      </span>
                    ) : (
                      <Link
                        to={`/reader/${appearance.issueId}`}
                        className="flex-1 truncate hover:text-[var(--accent)]"
                      >
                        #{appearance.number} {appearance.title}
                      </Link>
                    )}
                    <span className="text-[10px] uppercase tracking-wide text-muted">
                      {appearance.status}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <RelationshipsPanel
            characterId={detail.id}
            relationships={detail.relationships}
            onOpenCharacter={onOpenCharacter}
          />
        </div>
      )}
    </Modal>
  );
}

export function CharactersPanel({
  seriesId,
  characters,
  loading,
}: {
  seriesId: string;
  characters: CharacterCard[];
  loading: boolean;
}) {
  const queryClient = useQueryClient();
  const toast = useToast();
  const [openId, setOpenId] = useState<string | null>(null);

  const sync = useMutation({
    mutationFn: () => api.syncSeriesCharacters(seriesId),
    onSuccess: async (report) => {
      await queryClient.invalidateQueries({ queryKey: ["characters", seriesId] });
      await queryClient.invalidateQueries({ queryKey: ["characters"] });
      toast.push(
        report.characters === 0
          ? "No character tags found. Add a <Characters> entry to ComicInfo.xml and rescan."
          : `Found ${pluralize(report.characters, "character")} across ${pluralize(
              report.issuesScanned,
              "issue",
            )}`,
        report.characters === 0 ? "error" : "info",
      );
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  return (
    <section className="mt-8">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="font-display text-sm font-semibold tracking-tight">Characters</h2>
          <p className="mt-0.5 text-[11px] text-muted">
            Built from the <code>&lt;Characters&gt;</code> tag in each issue's ComicInfo.xml. No
            network is needed until you ask for a web summary.
          </p>
        </div>
        <button
          className="btn btn-sm"
          onClick={() => sync.mutate()}
          disabled={sync.isPending}
          title="Read every issue's character tags again"
        >
          <Icon name="refresh" className="size-3.5" />
          {sync.isPending ? "Scanning…" : "Rebuild roster"}
        </button>
      </div>

      {loading && (
        <div className="grid h-24 place-items-center">
          <Spinner />
        </div>
      )}

      {!loading && characters.length === 0 && (
        <EmptyState
          title="No characters recorded yet"
          body="Issues imported with a <Characters> tag in ComicInfo.xml appear here. Rebuild the roster after adding tags, then fill any gaps with a web summary."
          icon={<Icon name="book" className="size-5" />}
        />
      )}

      {characters.length > 0 && (
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {characters.map((character) => (
            <button
              key={character.id}
              onClick={() => setOpenId(character.id)}
              className={cx(
                "rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3 text-left",
                "transition-colors hover:border-[var(--accent)]/40 hover:bg-[var(--surface-2)]",
              )}
            >
              <div className="flex items-baseline justify-between gap-2">
                <span className="truncate text-[13px] font-medium">{character.name}</span>
                <span className="shrink-0 text-[10px] text-muted">
                  {character.appearanceCount}×
                </span>
              </div>
              <p className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-muted">
                {character.description || "No description yet — open to write or look one up."}
              </p>
              <span className="mt-2 inline-flex items-center gap-1 text-[10px] text-muted">
                <Icon
                  name={character.descriptionSource === "web" ? "search" : "edit"}
                  className="size-3"
                />
                {SOURCE_LABEL[character.descriptionSource]}
              </span>
            </button>
          ))}
        </div>
      )}

      <CharacterDetailModal
        characterId={openId}
        onClose={() => setOpenId(null)}
        onOpenCharacter={(id) => setOpenId(id)}
      />
    </section>
  );
}
