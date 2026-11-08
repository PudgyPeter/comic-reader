import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { EmptyState, Modal, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { formatDate, pluralize } from "../lib/utils";
import { useToast } from "../components/Toast";

export default function OrdersPage() {
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const toast = useToast();
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["orders"], queryFn: api.readingOrders });

  const generate = async () => {
    const stamp = new Date().toLocaleDateString();
    const detail = await api.generatePublicationOrder(`Publication order · ${stamp}`);
    await queryClient.invalidateQueries({ queryKey: ["orders"] });
    toast.push(`Generated ${pluralize(detail.items.length, "issue")}`, "success");
  };

  return (
    <div className="mx-auto w-full max-w-[1200px] px-6 py-6">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">Reading orders</h1>
          <p className="mt-1 text-[13px] text-muted">
            Ordered sequences across series. Publication orders are generated; story orders are yours.
          </p>
        </div>
        <div className="flex gap-2">
          <button className="btn btn-sm" onClick={generate}>
            <Icon name="refresh" className="size-3.5" /> Generate publication order
          </button>
          <button className="btn btn-sm btn-primary" onClick={() => setCreating(true)}>
            <Icon name="plus" className="size-3.5" /> New order
          </button>
        </div>
      </div>

      {query.isLoading && (
        <div className="grid h-48 place-items-center">
          <Spinner />
        </div>
      )}

      {query.isSuccess && query.data.length === 0 && (
        <EmptyState
          title="No reading orders"
          body="Generate a publication order from release dates, or create an empty order and add issues manually."
        />
      )}

      <div className="space-y-2">
        {query.data?.map((order) => (
          <div
            key={order.id}
            className="group flex items-center gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] px-4 py-3 transition-colors hover:border-[var(--line-strong)]"
          >
            <div className="grid size-9 place-items-center rounded-lg border border-[var(--line)] bg-[var(--surface-1)] text-[var(--accent)]">
              <Icon name="list" className="size-4" />
            </div>
            <Link to={`/orders/${order.id}`} className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{order.name}</p>
              <p className="truncate text-[11px] text-muted">
                {[
                  order.kind === "publication" ? "Publication order" : "Chronological order",
                  pluralize(order.itemCount, "issue"),
                  formatDate(order.createdAt),
                ].join(" · ")}
              </p>
            </Link>
            <button
              className="rounded p-1.5 text-muted opacity-0 hover:bg-[var(--surface-3)] hover:text-rose-300 group-hover:opacity-100"
              aria-label="Delete order"
              onClick={async () => {
                await api.deleteReadingOrder(order.id);
                await queryClient.invalidateQueries({ queryKey: ["orders"] });
                toast.push("Reading order deleted", "success");
              }}
            >
              <Icon name="trash" className="size-3.5" />
            </button>
          </div>
        ))}
      </div>

      <Modal
        open={creating}
        title="New reading order"
        onClose={() => setCreating(false)}
        footer={
          <>
            <button className="btn" onClick={() => setCreating(false)}>
              Cancel
            </button>
            <button
              className="btn btn-primary"
              disabled={!name.trim()}
              onClick={async () => {
                const created = await api.createReadingOrder(name.trim(), description.trim() || undefined);
                setName("");
                setDescription("");
                setCreating(false);
                await queryClient.invalidateQueries({ queryKey: ["orders"] });
                toast.push(`Created “${created.name}”`, "success");
              }}
            >
              Create
            </button>
          </>
        }
      >
        <div className="space-y-3">
          <label>
            <span className="label">Name</span>
            <input
              className="input"
              value={name}
              onChange={(event) => setName(event.currentTarget.value)}
              placeholder="Sonic IDW — story order"
              autoFocus
            />
          </label>
          <label>
            <span className="label">Description</span>
            <textarea
              className="input min-h-20"
              value={description}
              onChange={(event) => setDescription(event.currentTarget.value)}
              placeholder="What this order represents, and why it differs from publication order"
            />
          </label>
        </div>
      </Modal>
    </div>
  );
}
