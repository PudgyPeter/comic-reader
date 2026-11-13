import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { useToast } from "./Toast";
import { Icon } from "./Icon";

/**
 * Optional ComicVine metadata.
 *
 * Two rules shape this UI: the key is user-supplied and stored locally only, and
 * nothing the provider says reaches the roster until the user accepts it. Every
 * lookup is explicit — the app stays fully usable with no key and no network.
 */
export function ProviderSettings() {
  const toast = useToast();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState("");
  const status = useQuery({ queryKey: ["provider-status"], queryFn: api.providerStatus });

  const save = useMutation({
    mutationFn: () => api.setProviderKey(draft),
    onSuccess: async () => {
      setDraft("");
      await queryClient.invalidateQueries({ queryKey: ["provider-status"] });
      toast.push("ComicVine key saved locally", "info");
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const clear = useMutation({
    mutationFn: () => api.setProviderKey(""),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["provider-status"] });
      toast.push("ComicVine key removed", "info");
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-xs font-medium">ComicVine</p>
          <p className="text-[11px] text-muted">
            {status.data?.keySet
              ? "Key set. Lookups run only when you ask for them."
              : "No key set. Everything else in the app works without it."}
          </p>
        </div>
        {status.data?.keySet && (
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => clear.mutate()}
            disabled={clear.isPending}
          >
            Remove key
          </button>
        )}
      </div>

      <div className="flex gap-2">
        <input
          className="input flex-1"
          type="password"
          autoComplete="off"
          placeholder={status.data?.keySet ? "Replace key…" : "Paste your ComicVine API key"}
          value={draft}
          onChange={(event) => setDraft(event.currentTarget.value)}
          aria-label="ComicVine API key"
        />
        <button
          className="btn btn-primary btn-sm"
          onClick={() => save.mutate()}
          disabled={!draft.trim() || save.isPending}
        >
          {save.isPending ? "Saving…" : "Save"}
        </button>
      </div>

      <p className="text-[11px] leading-relaxed text-muted">
        The key is stored in this library's database and sent only to
        comicvine.gamespot.com. Responses are cached locally, so a rescan costs no
        requests. ComicVine allows 200 requests per resource per hour; the app counts
        locally and refuses rather than being throttled mid-import.
      </p>

      <div className="flex flex-wrap gap-3 text-[11px] text-muted">
        <span className="chip">{status.data?.cachedResponses ?? 0} cached responses</span>
        <span className="chip">{status.data?.pendingProposals ?? 0} pending proposals</span>
      </div>
    </div>
  );
}

/** A series-level provider result with an explicit accept step for credits. */
export function ProviderSeriesPanel({
  seriesId,
  onApplied,
}: {
  seriesId: string;
  onApplied?: () => void;
}) {
  const toast = useToast();
  const queryClient = useQueryClient();
  const [proposal, setProposal] = useState<import("../lib/types").ProviderProposal | null>(null);
  const [edges, setEdges] = useState<import("../lib/types").ProposedEdge[] | null>(null);

  const lookup = useMutation({
    mutationFn: () => api.lookupSeriesProvider(seriesId),
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const propose = useMutation({
    mutationFn: () => api.proposeSeriesCharacters(seriesId),
    onSuccess: async (result) => {
      setProposal(result);
      await queryClient.invalidateQueries({ queryKey: ["provider-status"] });
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const enrich = useMutation({
    mutationFn: () => api.enrichSeriesCharacters(seriesId),
    onSuccess: async (count) => {
      await queryClient.invalidateQueries({ queryKey: ["characters", seriesId] });
      toast.push(`Added bios to ${count} characters`, "info");
      onApplied?.();
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const proposeRelationships = useMutation({
    mutationFn: () => api.proposeSeriesRelationships(seriesId),
    onSuccess: async (found) => {
      setEdges(found);
      await queryClient.invalidateQueries({ queryKey: ["provider-status"] });
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const decideEdges = useMutation({
    mutationFn: async ({ accept }: { accept: boolean }) => {
      const pending = await api.providerProposals();
      const row = pending.find(
        (entry) => entry.entity === "series" && entry.localId === seriesId,
      );
      if (!row) throw new Error("no pending relationship proposal");
      return api.decideProviderProposal(row.id, accept);
    },
    onSuccess: async (applied, { accept }) => {
      setEdges(null);
      await queryClient.invalidateQueries({ queryKey: ["provider-status"] });
      await queryClient.invalidateQueries({ queryKey: ["character"] });
      await queryClient.invalidateQueries({ queryKey: ["characters"] });
      toast.push(
        accept ? `Linked ${applied} relationships` : "Relationship proposal rejected",
        "info",
      );
      onApplied?.();
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const decide = useMutation({
    mutationFn: ({ accept }: { accept: boolean }) =>
      api.decideProviderProposal(proposal!.id, accept),
    onSuccess: async (applied, { accept }) => {
      setProposal(null);
      await queryClient.invalidateQueries({ queryKey: ["provider-status"] });
      await queryClient.invalidateQueries({ queryKey: ["series", seriesId] });
      await queryClient.invalidateQueries({ queryKey: ["characters"] });
      toast.push(accept ? `Added ${applied} characters` : "Proposal rejected", "info");
      onApplied?.();
    },
    onError: (error: Error) => toast.push(error.message, "error"),
  });

  const found = lookup.data;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs font-medium">ComicVine</p>
        <button
          className="btn btn-ghost btn-sm"
          onClick={() => lookup.mutate()}
          disabled={lookup.isPending}
          title="Look this series up by title, publisher and issue numbers"
        >
          <Icon name="search" className="size-3.5" />
          {lookup.isPending ? "Looking up…" : "Look up series"}
        </button>
      </div>

      {found && (
        <div className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] p-2.5 text-[11px]">
          <p className="font-medium">
            {found.name}
            {found.startYear ? ` (${found.startYear})` : ""}
          </p>
          <p className="text-muted">
            {found.publisher ?? "Publisher unknown"}
            {found.issueCount ? ` · ${found.issueCount} issues` : ""}
          </p>
          {found.descriptor && <p className="mt-1.5 leading-relaxed">{found.descriptor}</p>}
          <a
            href={found.sourceUrl}
            className="mt-1 inline-block text-[10px] text-[var(--accent)] hover:underline"
            target="_blank"
            rel="noreferrer"
          >
            Source
          </a>
        </div>
      )}

      {found && !proposal && (
        <div className="flex flex-wrap gap-2">
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => propose.mutate()}
            disabled={propose.isPending}
          >
            {propose.isPending ? "Fetching credits…" : "Propose characters from issues"}
          </button>
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => enrich.mutate()}
            disabled={enrich.isPending}
            title="One-line bios and aliases for characters already linked"
          >
            {enrich.isPending ? "Fetching bios…" : "Fetch bios & aliases"}
          </button>
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => proposeRelationships.mutate()}
            disabled={proposeRelationships.isPending}
            title="Edges are filtered to characters already in the library"
          >
            {proposeRelationships.isPending ? "Building graph…" : "Propose relationships"}
          </button>
        </div>
      )}

      {edges && (
        <div className="max-h-64 overflow-y-auto rounded-lg border border-[var(--accent)]/40 bg-[var(--surface-2)] p-2.5">
          <p className="text-[11px] text-muted">
            {edges.length} edges between characters in this library. Provider lists span
            every continuity; anything outside your library is dropped.
          </p>
          <ul className="mt-1.5 space-y-0.5">
            {edges.slice(0, 40).map((edge, index) => (
              <li key={`${edge.characterId}-${edge.otherId}-${index}`} className="text-[11px]">
                <span className="font-medium">{edge.characterName}</span>
                <span className="text-muted"> — {edge.relationshipType.replace(/_/g, " ")} — </span>
                <span className="font-medium">{edge.otherName}</span>
              </li>
            ))}
          </ul>
          {edges.length > 40 && (
            <p className="mt-1 text-[10px] text-muted">…and {edges.length - 40} more</p>
          )}
          <div className="mt-2 flex justify-end gap-2">
            <button
              className="btn btn-ghost btn-sm"
              onClick={() => decideEdges.mutate({ accept: false })}
              disabled={decideEdges.isPending}
            >
              Reject
            </button>
            <button
              className="btn btn-primary btn-sm"
              onClick={() => decideEdges.mutate({ accept: true })}
              disabled={decideEdges.isPending || edges.length === 0}
            >
              Accept {edges.length}
            </button>
          </div>
        </div>
      )}

      {proposal && (
        <div className="rounded-lg border border-[var(--accent)]/40 bg-[var(--surface-2)] p-2.5">
          <p className="text-[11px] text-muted">
            Matched {proposal.matchedIssues} of {proposal.localIssues} issues ·{" "}
            {proposal.credits.length} characters proposed. Nothing is saved until you accept.
          </p>
          <p className="mt-1.5 text-[11px] leading-relaxed">
            {proposal.credits.map((credit) => credit.name).join(", ")}
          </p>
          <div className="mt-2 flex justify-end gap-2">
            <button
              className="btn btn-ghost btn-sm"
              onClick={() => decide.mutate({ accept: false })}
              disabled={decide.isPending}
            >
              Reject
            </button>
            <button
              className="btn btn-primary btn-sm"
              onClick={() => decide.mutate({ accept: true })}
              disabled={decide.isPending}
            >
              Accept {proposal.credits.length}
            </button>
          </div>
        </div>
      )}

      <p className="text-[10px] leading-relaxed text-muted">
        Credits come from ComicVine's per-issue records, not from the pages, and are a
        suggestion you review — automated matching can pick the wrong run.
      </p>
    </div>
  );
}
