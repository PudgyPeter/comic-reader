import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { Cover, EmptyState, Modal, Spinner } from "../components/ui";
import { Icon } from "../components/Icon";
import { pluralize } from "../lib/utils";
import { useToast } from "../components/Toast";

export default function CollectionsPage() {
  const [creating, setCreating] = useState(false);
  const toast = useToast();
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["collections"], queryFn: api.collections });
  const [editing, setEditing] = useState<{
    id: string;
    name: string;
    description: string;
    coverFileId: string | null;
  } | null>(null);

  const coverOptions = useQuery({
    queryKey: ["collection-cover-options"],
    queryFn: () => api.library({ sort: "series" }),
    enabled: Boolean(editing),
  });

  return (
    <div className="mx-auto w-full max-w-[1400px] px-6 py-6">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl font-semibold tracking-tight">Collections</h1>
          <p className="mt-1 text-[13px] text-muted">
            Personal groupings that can overlap. Favourites, arcs, or reading piles.
          </p>
        </div>
        <button className="btn btn-sm btn-primary" onClick={() => setCreating(true)}>
          <Icon name="plus" className="size-3.5" /> New collection
        </button>
      </div>

      {query.isLoading && (
        <div className="grid h-48 place-items-center">
          <Spinner />
        </div>
      )}

      {query.isSuccess && query.data.length === 0 && (
        <EmptyState
          title="No collections yet"
          body="Create a collection, then add issues from the library or a series page."
          action={
            <button className="btn btn-primary" onClick={() => setCreating(true)}>
              Create your first collection
            </button>
          }
        />
      )}

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {query.data?.map((collection) => (
          <div
            key={collection.id}
            className="group flex gap-3 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-3 transition-colors hover:border-[var(--line-strong)]"
          >
            <Link
              to={`/collections/${collection.id}`}
              className="flex flex-1 gap-3 overflow-hidden"
            >
              <div className="grid shrink-0 grid-cols-2 gap-0.5">
                {collection.coverFileId ? (
                  [0, 1, 2, 3].map((slot) => (
                    <Cover
                      key={slot}
                      fileId={collection.coverFileId}
                      title={collection.name}
                      className="h-14 w-10"
                      width={80}
                      rounded="rounded"
                    />
                  ))
                ) : (
                  <Cover
                    fileId={null}
                    title={collection.name}
                    className="col-span-2 row-span-2 h-[7.125rem] w-[5.125rem]"
                    rounded="rounded"
                  />
                )}
              </div>
              <div className="min-w-0 flex-1 py-0.5">
                <p className="truncate text-sm font-medium">{collection.name}</p>
                <p className="mt-0.5 line-clamp-2 text-[11px] leading-relaxed text-muted">
                  {collection.description || "No description"}
                </p>
                <p className="mt-1.5 text-[10px] text-muted">
                  {pluralize(collection.itemCount, "issue")}
                </p>
              </div>
            </Link>
            <div className="flex flex-col gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <button
                className="rounded p-1 text-muted hover:bg-[var(--surface-3)] hover:text-[var(--text-strong)]"
                aria-label="Edit collection"
                onClick={() =>
                  setEditing({
                    id: collection.id,
                    name: collection.name,
                    description: collection.description ?? "",
                    coverFileId: collection.coverFileId,
                  })
                }
              >
                <Icon name="edit" className="size-3.5" />
              </button>
              <button
                className="rounded p-1 text-muted hover:bg-[var(--surface-3)] hover:text-rose-300"
                aria-label="Delete collection"
                onClick={async () => {
                  await api.deleteCollection(collection.id);
                  await queryClient.invalidateQueries({ queryKey: ["collections"] });
                  toast.push("Collection deleted", "success");
                }}
              >
                <Icon name="trash" className="size-3.5" />
              </button>
            </div>
          </div>
        ))}
      </div>

      <CollectionForm
        open={creating}
        title="New collection"
        onClose={() => setCreating(false)}
        onSubmit={async (name, description) => {
          await api.createCollection(name, description || undefined);
          await queryClient.invalidateQueries({ queryKey: ["collections"] });
          setCreating(false);
          toast.push("Collection created", "success");
        }}
      />

      <Modal
        open={Boolean(editing)}
        title="Edit collection"
        onClose={() => setEditing(null)}
        footer={
          <>
            <button className="btn" onClick={() => setEditing(null)}>
              Cancel
            </button>
            <button
              className="btn btn-primary"
              onClick={async () => {
                if (!editing) return;
                await api.updateCollection(
                  editing.id,
                  editing.name,
                  editing.description,
                  editing.coverFileId ?? undefined,
                );
                await queryClient.invalidateQueries({ queryKey: ["collections"] });
                await queryClient.invalidateQueries({ queryKey: ["collection", editing.id] });
                setEditing(null);
                toast.push("Collection updated", "success");
              }}
            >
              Save
            </button>
          </>
        }
      >
        {editing && (
          <div className="space-y-3">
            <label>
              <span className="label">Name</span>
              <input
                className="input"
                value={editing.name}
                onChange={(event) => setEditing({ ...editing, name: event.currentTarget.value })}
              />
            </label>
            <label>
              <span className="label">Description</span>
              <textarea
                className="input min-h-20"
                value={editing.description}
                onChange={(event) =>
                  setEditing({ ...editing, description: event.currentTarget.value })
                }
              />
            </label>
            <label>
              <span className="label">Cover image</span>
              <div className="flex items-center gap-2">
                <Cover
                  fileId={editing.coverFileId ?? null}
                  title={editing.name}
                  className="h-16 w-11 shrink-0"
                  width={90}
                  rounded="rounded"
                />
                <div className="flex-1 space-y-1.5">
                  <select
                    className="input h-8 text-xs"
                    value={editing.coverFileId ?? ""}
                    onChange={(event) =>
                      setEditing({
                        ...editing,
                        coverFileId: event.currentTarget.value || null,
                      })
                    }
                  >
                    <option value="">Use first issue cover</option>
                    {coverOptions.data?.issues.map((issue) => (
                      <option key={issue.id} value={issue.fileId ?? ""}>
                        {issue.title}
                      </option>
                    ))}
                  </select>
                  <p className="text-[10px] text-muted">
                    Pick any issue to use its first page as the collection cover.
                  </p>
                </div>
              </div>
            </label>
          </div>
        )}
      </Modal>
    </div>
  );
}

export function CollectionForm({
  open,
  title,
  onClose,
  onSubmit,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  onSubmit: (name: string, description: string) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  return (
    <Modal
      open={open}
      title={title}
      onClose={onClose}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn btn-primary"
            disabled={!name.trim()}
            onClick={async () => {
              await onSubmit(name.trim(), description.trim());
              setName("");
              setDescription("");
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
            placeholder="Best Shadow appearances"
            autoFocus
          />
        </label>
        <label>
          <span className="label">Description</span>
          <textarea
            className="input min-h-20"
            value={description}
            onChange={(event) => setDescription(event.currentTarget.value)}
            placeholder="Why these issues belong together"
          />
        </label>
      </div>
    </Modal>
  );
}
