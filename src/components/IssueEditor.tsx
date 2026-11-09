import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";
import type { IssueCard, IssuePatch } from "../lib/types";
import { Modal } from "./ui";
import { useToast } from "./Toast";

export function IssueEditor(props: {
  issue: IssueCard | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  // Remount per issue so form state always starts from the selected issue.
  return <IssueEditorForm key={props.issue?.id ?? "empty"} {...props} />;
}

function IssueEditorForm({
  issue,
  onClose,
  onSaved,
}: {
  issue: IssueCard | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const toast = useToast();
  const [form, setForm] = useState<IssuePatch>({});
  const [saving, setSaving] = useState(false);
  const [reveal, setReveal] = useState(false);
  const [markPage, setMarkPage] = useState("");
  const [locked, setLocked] = useState(issue?.metadataLocked ?? false);
  const [lockDirty, setLockDirty] = useState(false);

  const series = useQuery({
    queryKey: ["editor-series"],
    queryFn: () => api.library({ sort: "series" }),
    enabled: Boolean(issue),
  });

  const [lastIssueId, setLastIssueId] = useState(issue?.id ?? null);
  if ((issue?.id ?? null) !== lastIssueId) {
    setLastIssueId(issue?.id ?? null);
    setForm({
      title: issue?.title ?? "",
      number: issue?.number ?? "",
      volume: issue?.volume ?? "",
      publisher: issue?.publisher ?? "",
      releaseDate: issue?.releaseDate ?? "",
      writer: issue?.writer ?? "",
      artist: issue?.artist ?? "",
      coverArtist: issue?.coverArtist ?? "",
      description: issue?.description ?? "",
      seriesId: issue?.seriesId ?? "",
      edition: issue?.edition ?? "",
      variantOf: issue?.variantOf ?? null,
    });
    setReveal(false);
    setLocked(issue?.metadataLocked ?? false);
    setLockDirty(false);
  }

  const field = (key: keyof IssuePatch) => ({
    value: (form[key] as string | undefined) ?? "",
    onChange: (event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) =>
      setForm((current) => ({ ...current, [key]: event.currentTarget.value })),
  });

  const save = async () => {
    if (!issue) return;
    setSaving(true);
    try {
      const patch: IssuePatch = { ...form };
      if (!patch.seriesId) delete patch.seriesId;
      if (patch.variantOf === null) patch.variantOf = "";
      if (patch.edition === null) patch.edition = "";
      // Only send the lock when the user changed it; otherwise saving an edit
      // implicitly locks the issue on the backend.
      if (lockDirty) patch.metadataLocked = locked;
      await api.updateIssue(issue.id, patch);
      onSaved();
    } catch (error) {
      toast.push(String(error), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={Boolean(issue)}
      title={issue ? `Edit metadata · ${issue.title}` : "Edit metadata"}
      onClose={onClose}
      width="max-w-2xl"
      footer={
        <>
          {issue?.spoilerProtected && (
            <button
              className="btn btn-sm mr-auto"
              onClick={async () => {
                await api.revealSpoiler(issue.id);
                const fresh = await api.issueDetail(issue.id);
                setForm((current) => ({ ...current, description: fresh.description ?? "" }));
                setReveal(true);
                toast.push("Spoiler revealed for this issue", "info");
              }}
            >
              Reveal protected description
            </button>
          )}
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={save} disabled={saving}>
            {saving ? "Saving…" : "Save changes"}
          </button>
        </>
      }
    >
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <label className="sm:col-span-2">
          <span className="label">Title</span>
          <input className="input" {...field("title")} />
        </label>
        <label>
          <span className="label">Series</span>
          <select className="input" {...field("seriesId")}>
            <option value="">— Unassigned —</option>
            {(series.data?.series ?? []).map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.title}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span className="label">Issue number</span>
          <input className="input" {...field("number")} />
        </label>
        <label>
          <span className="label">Volume</span>
          <input className="input" {...field("volume")} />
        </label>
        <label>
          <span className="label">Publisher</span>
          <input className="input" {...field("publisher")} />
        </label>
        <label>
          <span className="label">Release date</span>
          <input className="input" placeholder="YYYY-MM-DD or YYYY" {...field("releaseDate")} />
        </label>
        <label>
          <span className="label">Writer</span>
          <input className="input" {...field("writer")} />
        </label>
        <label>
          <span className="label">Artist</span>
          <input className="input" {...field("artist")} />
        </label>
        <label>
          <span className="label">Cover artist</span>
          <input className="input" {...field("coverArtist")} />
        </label>
        <label>
          <span className="label">Edition</span>
          <input
            className="input"
            {...field("edition")}
            placeholder="e.g. Variant cover B, Director's cut"
          />
        </label>
        <label>
          <span className="label">Alternate of</span>
          <select
            className="input"
            value={(form.variantOf as string | null) ?? ""}
            onChange={(event) =>
              setForm((current) => ({
                ...current,
                variantOf: event.currentTarget.value || null,
              }))
            }
          >
            <option value="">Standalone issue</option>
            {(series.data?.issues ?? [])
              .filter(
                (entry) =>
                  entry.id !== issue?.id &&
                  entry.seriesId === (form.seriesId || issue?.seriesId) &&
                  !entry.variantOf,
              )
              .map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.seriesTitle ? `${entry.seriesTitle} ` : ""}#{entry.number} · {entry.title}
                </option>
              ))}
          </select>
          <p className="mt-1 text-[10px] text-muted">
            Links this file to a base issue as an alternate cover or special edition.
          </p>
        </label>
        <label className="sm:col-span-2">
          <span className="label">Description</span>
          <textarea
            className="input min-h-24 resize-y"
            {...field("description")}
            placeholder={issue?.spoilerProtected && !reveal ? "Hidden by spoiler protection" : ""}
          />
        </label>
      </div>
      {issue && issue.pageCount > 0 && (
        <div className="mt-3 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 py-2">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[11px] font-medium">Reading position</span>
            <span className="text-[10px] text-muted">
              page {issue.lastPage + 1} of {issue.pageCount}
            </span>
            <form
              className="ml-auto flex items-center gap-1"
              onSubmit={async (event) => {
                event.preventDefault();
                const value = Number(markPage);
                if (!Number.isFinite(value)) return;
                const target = Math.max(1, Math.min(issue.pageCount, Math.round(value)));
                setSaving(true);
                try {
                  await api.markIssueToPage(issue.id, target - 1);
                  setMarkPage("");
                  toast.push(`Marked read up to page ${target}`, "success");
                  onSaved();
                } catch (error) {
                  toast.push(String(error), "error");
                } finally {
                  setSaving(false);
                }
              }}
            >
              <input
                value={markPage}
                onChange={(event) =>
                  setMarkPage(event.currentTarget.value.replace(/[^0-9]/g, ""))
                }
                inputMode="numeric"
                placeholder="Page"
                aria-label="Mark read up to page"
                title="Mark every page up to this number as read"
                className="input h-7 w-16 text-center text-xs"
              />
              <button className="btn btn-sm" disabled={saving || !markPage} type="submit">
                Mark read to page
              </button>
            </form>
          </div>
          <p className="mt-1 text-[10px] text-muted">
            Pages up to this number count as read. The final page completes the issue.
          </p>
        </div>
      )}
      <div className="mt-3 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 py-2">
        <label className="flex items-start gap-2">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={locked}
            onChange={(event) => {
              setLocked(event.currentTarget.checked);
              setLockDirty(true);
            }}
          />
          <span>
            <span className="text-[11px] font-medium">Protect manual edits (metadata lock)</span>
            <span className="mt-0.5 block text-[10px] text-muted">
              {locked
                ? "Locked: rescans and metadata refreshes will not overwrite these fields."
                : "Unlocked: the next metadata refresh may replace these fields. Saving an edit locks them automatically."}
            </span>
          </span>
        </label>
        {locked && (
          <button
            type="button"
            className="btn btn-sm mt-2"
            disabled={saving}
            onClick={async () => {
              if (!issue) return;
              setSaving(true);
              try {
                const refreshed = await api.refreshIssueMetadata(issue.id, true);
                toast.push(
                  refreshed
                    ? "Metadata restored from the archive"
                    : "Refresh skipped — unlock the issue first",
                  refreshed ? "success" : "info",
                );
                onSaved();
              } catch (error) {
                toast.push(String(error), "error");
              } finally {
                setSaving(false);
              }
            }}
          >
            Refresh from file
          </button>
        )}
      </div>
      <p className="mt-3 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 py-2 text-[11px] leading-relaxed text-muted">
        Metadata is stored in the local database. The original archive is never modified.
      </p>
    </Modal>
  );
}
