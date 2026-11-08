import { useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, ProgressBar, Spinner, StatusBadge } from "../components/ui";
import { Icon } from "../components/Icon";
import { cx, formatDate, progressRatio } from "../lib/utils";
import { useToast } from "../components/Toast";
import { collectionProgress } from "../lib/collectionProgress";
import { useDragReorder } from "../lib/reorder";

export default function CollectionDetailPage() {
  const { collectionId = "" } = useParams();
  const navigate = useNavigate();
  const toast = useToast();
  const queryClient = useQueryClient();
  const [order, setOrder] = useState<string[] | null>(null);

  const query = useQuery({
    queryKey: ["collection", collectionId],
    queryFn: () => api.collection(collectionId),
    enabled: Boolean(collectionId),
  });

  const items = useMemo(() => query.data?.items ?? [], [query.data]);
  const ids = useMemo(() => order ?? items.map((issue) => issue.id), [order, items]);
  const ordered = useMemo(() => {
    const map = new Map(items.map((issue) => [issue.id, issue]));
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
    await api.reorderCollection(collectionId, order);
    setOrder(null);
    await queryClient.invalidateQueries({ queryKey: ["collection", collectionId] });
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
        <EmptyState title="Collection not found" body="It may have been deleted." />
      </div>
    );
  }

  const collection = query.data;
  const progress = collectionProgress(items);

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <button className="btn btn-ghost btn-sm mb-4" onClick={() => navigate(-1)}>
        <Icon name="arrowLeft" className="size-3.5" /> Back
      </button>

      <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-[11px] uppercase tracking-[0.18em] text-muted">Collection</p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">{collection.name}</h1>
          <p className="mt-2 max-w-xl text-sm text-muted">
            {collection.description || "No description."}
          </p>
          <div className="mt-4 flex items-center gap-3">
            <ProgressBar value={progress} className="w-48" />
            <span className="text-[11px] text-muted">
              {items.length} issues · {Math.round(progress * 100)}%
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2">
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
            items.length > 1 && (
              <button className="btn btn-sm" onClick={() => setOrder([...ids])}>
                <Icon name="list" className="size-3.5" /> Reorder
              </button>
            )
          )}
        </div>
      </div>

      {items.length === 0 ? (
        <EmptyState
          title="Nothing here yet"
          body="Add issues from the library (select issues → Add to collection) or from a series page."
          action={
            <Link to="/" className="btn btn-primary">
              Browse library
            </Link>
          }
        />
      ) : (
        <div className="overflow-hidden rounded-xl border border-[var(--line)]">
          {ordered.map((issue, index) => (
            <div
              key={issue.id}
              {...(order ? drag.itemProps(issue.id) : {})}
              className={cx(
                "flex items-center gap-2 border-b border-[var(--line)] px-3 py-2 text-sm last:border-0 hover:bg-[var(--surface-1)]",
                order && drag.dragId === issue.id && "opacity-50",
                order && drag.overId === issue.id && "border-t-2 border-t-[var(--accent)]",
              )}
            >
              {order && (
                <span
                  className="cursor-grab text-muted active:cursor-grabbing"
                  aria-hidden
                  title="Drag to reorder"
                >
                  <Icon name="list" className="size-3.5" />
                </span>
              )}
              {order && (
                <div className="flex flex-col">
                  <button
                    className="text-muted hover:text-[var(--text-strong)] disabled:opacity-30"
                    onClick={() => move(index, -1)}
                    disabled={index === 0}
                    aria-label="Move up"
                  >
                    <Icon name="chevronDown" className="size-3 rotate-180" />
                  </button>
                  <button
                    className="text-muted hover:text-[var(--text-strong)] disabled:opacity-30"
                    onClick={() => move(index, 1)}
                    disabled={index === ordered.length - 1}
                    aria-label="Move down"
                  >
                    <Icon name="chevronDown" className="size-3" />
                  </button>
                </div>
              )}
              <span className="w-6 text-right font-mono text-[11px] text-muted">{index + 1}</span>
              <Link to={`/reader/${issue.id}`} className="flex min-w-0 flex-1 items-center gap-3">
                <Cover
                  fileId={issue.fileId}
                  title={issue.title}
                  className="h-12 w-8 shrink-0"
                  width={80}
                  rounded="rounded"
                />
                <div className="min-w-0">
                  <p className="truncate font-medium">{issue.title}</p>
                  <p className="truncate text-[11px] text-muted">
                    {[issue.seriesTitle, issue.number ? `#${issue.number}` : null, formatDate(issue.releaseDate)]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                </div>
              </Link>
              <ProgressBar value={progressRatio(issue)} className="hidden w-20 sm:block" />
              <StatusBadge status={issue.status} />
              <button
                className="rounded p-1 text-muted hover:bg-[var(--surface-3)] hover:text-rose-300"
                aria-label="Remove from collection"
                onClick={async () => {
                  await api.removeFromCollection(collectionId, [issue.id]);
                  await queryClient.invalidateQueries({ queryKey: ["collection", collectionId] });
                  setOrder(null);
                }}
              >
                <Icon name="x" className="size-3.5" />
              </button>
            </div>
          ))}
        </div>
      )}

      {items.length > 1 && !order && (
        <p className={cx("mt-3 text-[11px] text-muted")}>
          Tip: drag any row to reorder it, or use the arrow controls. Both update the saved order.
        </p>
      )}
    </div>
  );
}
