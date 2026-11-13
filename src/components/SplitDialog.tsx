import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import type { IssueCard, SplitEntry, SplitRange, UnmatchedFile } from "../lib/types";
import { pluralize } from "../lib/utils";
import { Modal, Spinner } from "./ui";
import { Icon } from "./Icon";
import { useToast } from "./Toast";

/**
 * Splits one archive into several issues using the contents page.
 *
 * OCR never writes anything by itself: the proposed ranges are shown, the user
 * can shift the archive offset (printed page 1 → which archive page) and edit
 * each range, and only "Split file" commits.
 */
export function SplitDialog({
  file,
  onClose,
  onSplit,
}: {
  file: UnmatchedFile | null;
  onClose: () => void;
  onSplit: (issues: IssueCard[]) => void;
}) {
  if (!file) return null;
  return (
    <Modal
      open
      title={`Split “${file.relativePath.split("/").pop()}”`}
      onClose={onClose}
      width="max-w-3xl"
    >
      <SplitForm file={file} onClose={onClose} onSplit={onSplit} />
    </Modal>
  );
}

function SplitForm({
  file,
  onClose,
  onSplit,
}: {
  file: UnmatchedFile;
  onClose: () => void;
  onSplit: (issues: IssueCard[]) => void;
}) {
  const toast = useToast();
  const queryClient = useQueryClient();
  const [offset, setOffset] = useState<number | null>(null);
  const [rows, setRows] = useState<SplitEntry[] | null>(null);
  const [busy, setBusy] = useState(false);

  const available = useQuery({
    queryKey: ["split-available"],
    queryFn: () => api.splitAvailable(),
    staleTime: Infinity,
  });

  const proposal = useQuery({
    queryKey: ["split-proposal", file.id],
    queryFn: () => api.proposeSplit(file.id),
    enabled: available.data === true,
    retry: false,
  });

  const seed = proposal.data;
  const effectiveOffset = offset ?? seed?.offset ?? 0;
  const entries = rows ?? seed?.entries ?? [];
  const total = seed?.pageCount ?? file.pageCount;

  const shift = async (next: number) => {
    if (!seed) return;
    setOffset(next);
    try {
      const updated = await api.recombineSplit(
        file.id,
        entries.map((entry) => ({
          title: entry.title,
          series: entry.series,
          number: entry.number,
          printedPage: entry.printedPage,
        })),
        next,
      );
      setRows(updated.entries);
    } catch (error) {
      toast.push(String(error), "error");
    }
  };

  const edit = (index: number, patch: Partial<SplitEntry>) => {
    setRows((current) =>
      (current ?? []).map((row, rowIndex) =>
        rowIndex === index ? { ...row, ...patch } : row,
      ),
    );
  };

  const submit = async () => {
    if (entries.length === 0) return;
    setBusy(true);
    try {
      const ranges: SplitRange[] = entries.map((entry, index) => ({
        title: entry.title,
        series: entry.series,
        number: entry.number || String(index + 1),
        startPage: entry.startPage,
        endPage: entry.endPage,
      }));
      const issues = await api.applySplit(
        file.id,
        file.suggestedSeries || undefined,
        ranges,
      );
      await queryClient.invalidateQueries({ queryKey: ["unmatched-files"] });
      await queryClient.invalidateQueries({ queryKey: ["library"] });
      onSplit(issues);
    } catch (error) {
      toast.push(String(error), "error");
    } finally {
      setBusy(false);
    }
  };

  if (available.isLoading || proposal.isLoading) {
    return (
      <div className="grid h-40 place-items-center gap-2 text-[12px] text-muted">
        <Spinner />
        <span>Reading the contents page…</span>
      </div>
    );
  }

  if (available.data === false) {
    return (
      <p className="text-[13px] text-muted">
        OCR models are missing from this build, so the contents page cannot be read. Reinstall the
        app, or split manually by editing the file outside Panel.
      </p>
    );
  }

  if (proposal.isError) {
    return (
      <div className="space-y-3 text-[13px]">
        <p className="text-muted">
          No contents page was found in the first pages of this file. Splitting needs a page that
          lists chapters with page numbers.
        </p>
        <p className="rounded-lg border border-[var(--line)] bg-[var(--surface-2)] p-3 text-[11px] text-muted">
          {String(proposal.error)}
        </p>
        <p className="text-[11px] text-muted">
          If the contents page is further in, this cannot find it yet.
        </p>
      </div>
    );
  }

  const invalid = entries.some(
    (entry, index) =>
      entry.endPage <= entry.startPage ||
      entry.startPage < (index === 0 ? 0 : entries[index - 1].endPage) ||
      entry.endPage > total,
  );

  return (
    <div className="space-y-3">
      <p className="text-[12px] text-muted">
        Found {pluralize(entries.length, "chapter")} on archive page {(seed?.tocPage ?? 0) + 1}.
        Adjust the offset so the first chapter starts on the right page, then check the ranges.
      </p>

      <label className="flex flex-wrap items-center gap-2 text-[12px]">
        <span className="text-muted">Printed page 1 is archive page</span>
        <input
          type="number"
          className="input w-20"
          value={effectiveOffset}
          min={0}
          max={Math.max(0, total - 1)}
          onChange={(event) => void shift(Number(event.currentTarget.value) || 0)}
        />
        <span className="text-muted">of {total}</span>
      </label>

      <div className="overflow-hidden rounded-lg border border-[var(--line)]">
        <table className="w-full border-collapse text-left text-[12px]">
          <thead className="bg-[var(--surface-2)] text-[10px] uppercase tracking-wide text-muted">
            <tr>
              <th className="px-3 py-2 font-medium">#</th>
              <th className="px-3 py-2 font-medium">Title</th>
              <th className="px-3 py-2 font-medium">Pages (archive)</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry, index) => (
              <tr key={`${entry.printedPage}-${index}`} className="border-t border-[var(--line)]">
                <td className="px-3 py-2 tabular-nums text-muted">{entry.number}</td>
                <td className="px-3 py-2">
                  <input
                    className="input text-[12px]"
                    value={entry.title}
                    onChange={(event) => edit(index, { title: event.currentTarget.value })}
                  />
                </td>
                <td className="whitespace-nowrap px-3 py-2">
                  <div className="flex items-center gap-1">
                    <input
                      type="number"
                      className="input w-16 text-[12px]"
                      value={entry.startPage}
                      onChange={(event) =>
                        edit(index, { startPage: Number(event.currentTarget.value) || 0 })
                      }
                    />
                    <span className="text-muted">→</span>
                    <input
                      type="number"
                      className="input w-16 text-[12px]"
                      value={entry.endPage}
                      onChange={(event) =>
                        edit(index, { endPage: Number(event.currentTarget.value) || 0 })
                      }
                    />
                    <span className="text-[10px] text-muted">
                      {Math.max(0, entry.endPage - entry.startPage)} pp
                    </span>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {invalid && (
        <p className="text-[11px] text-[var(--warning,#f59e0b)]">
          Ranges must be in order, non-empty, and inside the file.
        </p>
      )}

      <div className="flex justify-end gap-2 border-t border-[var(--line)] pt-3">
        <button className="btn" onClick={onClose} disabled={busy}>
          Cancel
        </button>
        <button
          className="btn btn-primary"
          onClick={submit}
          disabled={busy || invalid || entries.length === 0}
        >
          {busy ? <Spinner className="size-3.5" /> : <Icon name="check" className="size-3.5" />}
          Split into {pluralize(entries.length, "issue")}
        </button>
      </div>
    </div>
  );
}
