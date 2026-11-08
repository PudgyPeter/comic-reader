import { useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, Modal, ProgressBar, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { cx, formatDate, progressRatio } from "../lib/utils";
import { useToast } from "../components/Toast";
import { useDragReorder } from "../lib/reorder";

export default function OrderDetailPage() {
  const { orderId = "" } = useParams();
  const navigate = useNavigate();
  const toast = useToast();
  const queryClient = useQueryClient();
  const [adding, setAdding] = useState(false);
  const [order, setOrder] = useState<string[] | null>(null);
  const [search, setSearch] = useState("");

  const query = useQuery({
    queryKey: ["order", orderId],
    queryFn: () => api.readingOrder(orderId),
    enabled: Boolean(orderId),
  });

  const library = useQuery({
    queryKey: ["order-library-picker"],
    queryFn: () => api.library({ sort: "series" }),
    enabled: adding,
  });

  const items = useMemo(
    () => (query.data?.items ?? []).filter((item) => item.issue),
    [query.data],
  );
  const ids = useMemo(
    () => order ?? items.map((item) => item.issueId),
    [order, items],
  );
  const orderedItems = useMemo(() => {
    const map = new Map(items.map((item) => [item.issueId, item]));
    return ids.map((id) => map.get(id)).filter(Boolean) as typeof items;
  }, [ids, items]);

  const drag = useDragReorder(ids, setOrder);

  const move = (index: number, direction: -1 | 1) => {
    const next = [...ids];
    const target = index + direction;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    setOrder(next);
  };

  const saveOrder = async () => {
    if (!order) return;
    await api.reorderItems(orderId, order);
    setOrder(null);
    await queryClient.invalidateQueries({ queryKey: ["order", orderId] });
    toast.push("Order saved", "success");
  };

  if (query.isLoading) {
    return (
      <div className="grid h-64 place-items-center">
        <Spinner />
      </div>
    );
  }
  if (query.isError || !query.data) {
    return (
      <div className="p-6">
        <EmptyState title="Reading order not found" body="It may have been deleted." />
      </div>
    );
  }

  const detail = query.data;
  const completed = items.filter((item) => item.issue?.status === "completed").length;
  const upNext = orderedItems.find((item, index) => {
    if (!item.issue || item.issue.status === "completed") return false;
    return orderedItems.slice(0, index).every((entry) => entry.issue?.status === "completed");
  });
  const resume = upNext?.issue ?? orderedItems[0]?.issue ?? null;
  const candidates = (library.data?.issues ?? []).filter((issue) => {
    if (ids.includes(issue.id)) return false;
    if (!search.trim()) return true;
    const needle = search.trim().toLowerCase();
    return (
      issue.title.toLowerCase().includes(needle) ||
      (issue.seriesTitle ?? "").toLowerCase().includes(needle)
    );
  });

  return (
    <div className="mx-auto w-full max-w-[1100px] px-6 py-6">
      <button className="btn btn-ghost btn-sm mb-4" onClick={() => navigate(-1)}>
        <Icon name="arrowLeft" className="size-3.5" /> Back
      </button>

      <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-[11px] uppercase tracking-[0.18em] text-muted">
            {detail.kind === "publication" ? "Publication order" : "Chronological order"}
          </p>
          <h1 className="mt-1 font-display text-2xl font-semibold tracking-tight">{detail.name}</h1>
          {detail.description && (
            <p className="mt-1 max-w-xl text-sm text-muted">{detail.description}</p>
          )}
          <div className="mt-3 flex items-center gap-3">
            <ProgressBar
              value={items.length ? completed / items.length : 0}
              className="w-44"
            />
            <span className="text-[11px] text-muted">
              {completed}/{items.length} complete
            </span>
          </div>
        </div>
        <div className="flex gap-2">
          {resume && (
            <Link
              to={`/reader/${resume.id}?order=${detail.id}`}
              className="btn btn-sm btn-primary"
            >
              <Icon name="play" className="size-3.5" />
              {upNext ? "Continue in order" : "Start order"}
            </Link>
          )}
          {order ? (
            <>
              <button className="btn btn-sm" onClick={() => setOrder(null)}>
                Cancel
              </button>
              <button className="btn btn-sm btn-primary" onClick={saveOrder}>
                Save order
              </button>
            </>
          ) : (
            <>
              {orderedItems.length > 1 && (
                <button className="btn btn-sm" onClick={() => setOrder([...ids])}>
                  <Icon name="list" className="size-3.5" /> Reorder
                </button>
              )}
              <button className="btn btn-sm" onClick={() => setAdding(true)}>
                <Icon name="plus" className="size-3.5" /> Add issues
              </button>
            </>
          )}
        </div>
      </div>

      {orderedItems.length === 0 ? (
        <EmptyState
          title="This order is empty"
          body="Add issues in the sequence you want to read them. You can reorder at any time."
          action={
            <button className="btn btn-primary" onClick={() => setAdding(true)}>
              Add issues
            </button>
          }
        />
      ) : (
        <div className="relative">
          <div className="absolute bottom-0 left-[27px] top-0 w-px bg-[var(--surface-3)]" aria-hidden />
          <div className="space-y-1">
            {orderedItems.map((item, index) => {
              const issue = item.issue!;
              const nextUnread =
                issue.status !== "completed" &&
                orderedItems.slice(0, index).every((entry) => entry.issue?.status === "completed");
              return (
                <div
                  key={item.issueId}
                  {...(order ? drag.itemProps(item.issueId) : {})}
                  className={cx(
                    "relative flex items-center gap-3 rounded-xl border px-3 py-2 transition-colors",
                    nextUnread
                      ? "border-[var(--accent)]/35 bg-[var(--accent)]/6"
                      : "border-transparent hover:border-[var(--line)] hover:bg-[var(--surface-1)]",
                    order && drag.dragId === item.issueId && "opacity-50",
                    order && drag.overId === item.issueId && "border-t-2 border-t-[var(--accent)]",
                  )}
                >
                  {order && (
                    <span
                      className="z-10 cursor-grab text-muted active:cursor-grabbing"
                      aria-hidden
                      title="Drag to reorder"
                    >
                      <Icon name="list" className="size-3.5" />
                    </span>
                  )}
                  {order ? (
                    <div className="z-10 flex flex-col">
                      <button
                        className="text-muted hover:text-[var(--text-strong)] disabled:opacity-30"
                        onClick={() => move(index, -1)}
                        disabled={index === 0}
                        aria-label="Move earlier"
                      >
                        <Icon name="chevronDown" className="size-3 rotate-180" />
                      </button>
                      <button
                        className="text-muted hover:text-[var(--text-strong)] disabled:opacity-30"
                        onClick={() => move(index, 1)}
                        disabled={index === orderedItems.length - 1}
                        aria-label="Move later"
                      >
                        <Icon name="chevronDown" className="size-3" />
                      </button>
                    </div>
                  ) : (
                    <span
                      className={cx(
                        "z-10 grid size-7 shrink-0 place-items-center rounded-full border text-[10px] font-semibold",
                        issue.status === "completed"
                          ? "border-emerald-400/40 bg-emerald-500/20 text-emerald-300"
                          : "border-[var(--line-strong)] bg-[var(--panel)] text-muted",
                      )}
                    >
                      {index + 1}
                    </span>
                  )}
                  <Link
                    to={`/reader/${issue.id}?order=${detail.id}`}
                    className="flex min-w-0 flex-1 items-center gap-3"
                  >
                    <Cover
                      fileId={issue.fileId}
                      title={issue.title}
                      className="h-12 w-8 shrink-0"
                      width={80}
                      rounded="rounded"
                    />
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">{issue.title}</p>
                      <p className="truncate text-[11px] text-muted">
                        {[issue.seriesTitle, issue.number ? `#${issue.number}` : null, formatDate(issue.releaseDate)]
                          .filter(Boolean)
                          .join(" · ")}
                      </p>
                    </div>
                  </Link>
                  {nextUnread && <span className="chip chip-accent">Up next</span>}
                  <ProgressBar value={progressRatio(issue)} className="hidden w-20 sm:block" />
                  <StatusBadge status={issue.status} />
                </div>
              );
            })}
          </div>
        </div>
      )}

      <Modal
        open={adding}
        title="Add issues to order"
        onClose={() => setAdding(false)}
        width="max-w-2xl"
        footer={
          <button className="btn" onClick={() => setAdding(false)}>
            Done
          </button>
        }
      >
        <input
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
          placeholder="Filter issues…"
          className="input mb-3"
        />
        <div className="max-h-80 overflow-y-auto rounded-lg border border-[var(--line)]">
          {candidates.length === 0 && (
            <p className="px-3 py-6 text-center text-xs text-muted">No issues available.</p>
          )}
          {candidates.map((issue) => (
            <button
              key={issue.id}
              className="flex w-full items-center gap-2 border-b border-[var(--line)] px-3 py-2 text-left text-xs last:border-0 hover:bg-[var(--surface-2)]"
              onClick={async () => {
                await api.addOrderItems(orderId, [issue.id]);
                await queryClient.invalidateQueries({ queryKey: ["order", orderId] });
                await queryClient.invalidateQueries({ queryKey: ["order-library-picker"] });
                toast.push(`Added ${issue.title}`, "success");
              }}
            >
              <Cover
                fileId={issue.fileId}
                title={issue.title}
                className="h-9 w-6 shrink-0"
                width={60}
                rounded="rounded"
              />
              <span className="min-w-0 flex-1 truncate">{issue.title}</span>
              <span className="text-[10px] text-muted">{issue.seriesTitle}</span>
            </button>
          ))}
        </div>
      </Modal>
    </div>
  );
}
