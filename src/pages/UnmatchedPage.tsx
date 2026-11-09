import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import type { FileProposal, IssueCard, UnmatchedFile } from "../lib/types";
import { cx, formatRelative, pluralize } from "../lib/utils";
import { EmptyState, Modal, ProgressBar, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";

export default function UnmatchedPage() {
  const queryClient = useQueryClient();
  const toast = useToast();
  const [search, setSearch] = useState("");
  const [matching, setMatching] = useState<UnmatchedFile | null>(null);

  const query = useQuery({
    queryKey: ["unmatched-files"],
    queryFn: () => api.unmatchedFiles(),
  });

  const files = (query.data ?? []).filter((file) =>
    [file.relativePath, file.suggestedSeries, file.suggestedTitle]
      .join(" ")
      .toLowerCase()
      .includes(search.trim().toLowerCase()),
  );

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">Unmatched files</h1>
          <p className="mt-1 text-[13px] text-muted">
            Files that are not attached to an issue yet. They stay readable and keep their own
            progress. Matching one moves it to an issue without re-importing the file.
          </p>
        </div>
        <input
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
          placeholder="Filter files"
          className="input w-56"
          aria-label="Filter unmatched files"
        />
      </div>

      {query.isLoading && (
        <div className="grid h-48 place-items-center">
          <Spinner />
        </div>
      )}

      {query.isSuccess && files.length === 0 && (
        <EmptyState
          title={search ? "No files match" : "Everything is matched"}
          body={
            search
              ? "Try a different search term."
              : "Any archive imported with “Keep unmatched” or added outside an issue appears here."
          }
          icon={<Icon name="layers" className="size-5" />}
        />
      )}

      {files.length > 0 && (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          <table className="w-full border-collapse text-left text-[13px]">
            <thead className="bg-[var(--surface-2)] text-[11px] uppercase tracking-wide text-muted">
              <tr>
                <th className="px-4 py-2.5 font-medium">File</th>
                <th className="px-4 py-2.5 font-medium">Suggested</th>
                <th className="px-4 py-2.5 font-medium">Pages</th>
                <th className="px-4 py-2.5 font-medium">Progress</th>
                <th className="px-4 py-2.5 text-right font-medium">Actions</th>
              </tr>
            </thead>
            <tbody>
              {files.map((file) => {
                const ratio = file.pageCount ? (file.page + 1) / file.pageCount : 0;
                return (
                  <tr
                    key={file.id}
                    className="border-t border-[var(--line)] bg-[var(--surface-1)] align-middle transition-colors hover:bg-[var(--surface-2)]"
                  >
                    <td className="max-w-[360px] px-4 py-3">
                      <p className="truncate font-medium" title={file.path}>
                        {file.relativePath}
                      </p>
                      <p className="mt-0.5 truncate text-[11px] text-muted">
                        {file.format.toUpperCase()} · {(file.sizeBytes / 1_048_576).toFixed(1)} MB
                        {file.status !== "present" ? " · missing" : ""}
                        {file.lastReadAt ? ` · read ${formatRelative(file.lastReadAt)}` : ""}
                      </p>
                    </td>
                    <td className="px-4 py-3">
                      <p className="truncate text-[12px]">{file.suggestedSeries || "Unknown"}</p>
                      {file.suggestedNumber && (
                        <p className="text-[11px] text-muted">#{file.suggestedNumber}</p>
                      )}
                    </td>
                    <td className="px-4 py-3 tabular-nums text-muted">{file.pageCount}</td>
                    <td className="min-w-[140px] px-4 py-3">
                      <div className="flex items-center gap-2">
                        <ProgressBar value={ratio} />
                        <StatusBadge status={file.progressStatus} />
                      </div>
                    </td>
                    <td className="whitespace-nowrap px-4 py-3 text-right">
                      <div className="inline-flex items-center gap-1.5">
                        <Link to={`/file/${file.id}`} className="btn btn-sm">
                          <Icon name="book" className="size-3.5" />
                          Read
                        </Link>
                        <button
                          className={cx("btn btn-sm", "btn-primary")}
                          onClick={() => setMatching(file)}
                          disabled={file.status !== "present"}
                        >
                          <Icon name="layers" className="size-3.5" />
                          Match
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {files.length > 0 && (
        <p className="mt-3 text-[11px] text-muted">
          {pluralize(files.length, "file")} unmatched · {pluralize(files.reduce((sum, file) => sum + file.pageCount, 0), "page")}
        </p>
      )}

      <MatchFileDialog
        file={matching}
        onClose={() => setMatching(null)}
        onMatched={async (issue) => {
          setMatching(null);
          await queryClient.invalidateQueries({ queryKey: ["unmatched-files"] });
          await queryClient.invalidateQueries({ queryKey: ["library"] });
          toast.push(`Matched “${issue.title}”`, "success");
        }}
      />
    </div>
  );
}

const EMPTY_PATCH = {
  seriesTitle: "",
  seriesId: "",
  title: "",
  number: "",
  volume: "",
  publisher: "",
  releaseDate: "",
  writer: "",
  artist: "",
  coverArtist: "",
  description: "",
};

function MatchFileDialog({
  file,
  onClose,
  onMatched,
}: {
  file: UnmatchedFile | null;
  onClose: () => void;
  onMatched: (issue: IssueCard) => void;
}) {
  return (
    <Modal
      open={Boolean(file)}
      title={file ? `Match “${file.relativePath.split("/").pop()}”` : "Match file"}
      onClose={onClose}
      width="max-w-2xl"
    >
      {file && <MatchFileForm key={file.id} file={file} onClose={onClose} onMatched={onMatched} />}
    </Modal>
  );
}

function MatchFileForm({
  file,
  onClose,
  onMatched,
}: {
  file: UnmatchedFile;
  onClose: () => void;
  onMatched: (issue: IssueCard) => void;
}) {
  const toast = useToast();
  const [mode, setMode] = useState<"new" | "existing">("new");
  const [patch, setPatch] = useState(EMPTY_PATCH);
  const [issueId, setIssueId] = useState("");
  const [busy, setBusy] = useState(false);

  const proposal = useQuery({
    queryKey: ["file-proposal", file.id],
    queryFn: () => api.fileProposal(file.id),
  });

  const fileless = useQuery({
    queryKey: ["fileless-issues"],
    queryFn: () => api.filelessIssues(),
    enabled: mode === "existing",
  });

  const seed = proposal.data;
  const value = (key: keyof typeof EMPTY_PATCH) =>
    patch[key] || (seed ? String(seed[key as keyof FileProposal] ?? "") : "");
  const set = (key: keyof typeof EMPTY_PATCH, next: string) =>
    setPatch((current) => ({ ...current, [key]: next }));

  const submit = async () => {
    setBusy(true);
    try {
      const payload = {
        seriesTitle: value("seriesTitle") || undefined,
        seriesId: patch.seriesId || undefined,
        title: value("title") || undefined,
        number: value("number") || undefined,
        volume: value("volume") || undefined,
        publisher: value("publisher") || undefined,
        releaseDate: value("releaseDate") || undefined,
        writer: value("writer") || undefined,
        artist: value("artist") || undefined,
        coverArtist: value("coverArtist") || undefined,
        description: value("description") || undefined,
      };
      const issue = await api.applyFileMatch(
        file.id,
        payload,
        mode === "existing" ? issueId : null,
      );
      onMatched(issue);
    } catch (error) {
      toast.push(String(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const field = (key: keyof typeof EMPTY_PATCH, label: string, className?: string) => (
    <label className={className}>
      <span className="label">{label}</span>
      <input
        className="input"
        value={value(key)}
        onChange={(event) => set(key, event.currentTarget.value)}
      />
    </label>
  );

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        {(["new", "existing"] as const).map((entry) => (
          <button
            key={entry}
            className={cx("btn btn-sm", mode === entry && "btn-primary")}
            onClick={() => setMode(entry)}
          >
            {entry === "new" ? "Create new issue" : "Attach to existing issue"}
          </button>
        ))}
      </div>

      {mode === "existing" ? (
        <div className="space-y-2">
          <p className="text-[12px] text-muted">
            Only issues without a file are listed. Matching moves this file and its progress onto
            the selected issue.
          </p>
          {fileless.isLoading && <Spinner />}
          <select
            className="input"
            value={issueId}
            onChange={(event) => setIssueId(event.currentTarget.value)}
          >
            <option value="">— Select an issue —</option>
            {(fileless.data ?? []).map((issue) => (
              <option key={issue.id} value={issue.id}>
                {issue.seriesTitle ? `${issue.seriesTitle} · ` : ""}
                {issue.title} (#{issue.number})
              </option>
            ))}
          </select>
          {fileless.isSuccess && (fileless.data ?? []).length === 0 && (
            <p className="text-[11px] text-muted">
              Every issue already has a file. Create a new issue instead.
            </p>
          )}
        </div>
      ) : (
        <>
          {proposal.isLoading && <Spinner />}
          <p className="text-[11px] text-muted">
            Prefilled from ComicInfo.xml or the filename. Nothing is written until you confirm.
          </p>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            {field("seriesTitle", "Series", "sm:col-span-2")}
            {field("number", "Issue number")}
            {field("title", "Title")}
            {field("publisher", "Publisher")}
            {field("releaseDate", "Release date")}
            {field("volume", "Volume")}
            {field("writer", "Writer")}
            {field("artist", "Artist")}
            {field("coverArtist", "Cover artist")}
          </div>
        </>
      )}

      <div className="flex justify-end gap-2 border-t border-[var(--line)] pt-3">
        <button className="btn" onClick={onClose} disabled={busy}>
          Cancel
        </button>
        <button
          className="btn btn-primary"
          onClick={submit}
          disabled={busy || (mode === "existing" && !issueId)}
        >
          {busy ? <Spinner className="size-3.5" /> : <Icon name="check" className="size-3.5" />}
          {mode === "existing" ? "Attach file" : "Create issue"}
        </button>
      </div>
    </div>
  );
}
