import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api";
import { useToast } from "./Toast";

export function AddToCollectionMenu({
  issueIds,
  label = "Add to collection",
  seriesId,
}: {
  issueIds?: string[];
  label?: string;
  seriesId?: string;
}) {
  const [open, setOpen] = useState(false);
  const [creating, setCreating] = useState("");
  const toast = useToast();
  const queryClient = useQueryClient();
  const collections = useQuery({
    queryKey: ["collections"],
    queryFn: api.collections,
    enabled: open,
  });

  const add = async (collectionId: string) => {
    try {
      if (seriesId) {
        await api.addSeriesToCollection(collectionId, seriesId);
      } else if (issueIds?.length) {
        await api.addToCollection(collectionId, issueIds);
      }
      await queryClient.invalidateQueries({ queryKey: ["collections"] });
      await queryClient.invalidateQueries({ queryKey: ["collection"] });
      toast.push("Added to collection", "success");
      setOpen(false);
    } catch (error) {
      toast.push(String(error), "error");
    }
  };

  return (
    <div className="relative">
      <button className="btn btn-sm" onClick={() => setOpen((current) => !current)}>
        {label}
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-30" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full z-40 mt-1 w-64 rounded-xl border border-[var(--line)] bg-[var(--panel)] p-2 shadow-2xl animate-pop">
            <div className="max-h-56 overflow-y-auto">
              {collections.data?.length === 0 && (
                <p className="px-2 py-2 text-[11px] text-muted">No collections yet.</p>
              )}
              {collections.data?.map((collection) => (
                <button
                  key={collection.id}
                  onClick={() => add(collection.id)}
                  disabled={collection.isSmart}
                  title={
                    collection.isSmart
                      ? "Smart collections fill themselves from rules"
                      : `Add to ${collection.name}`
                  }
                  className="flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-left text-xs hover:bg-[var(--surface-3)] disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <span className="truncate">
                    {collection.name}
                    {collection.isSmart && (
                      <span className="ml-1 text-[9px] uppercase tracking-wide text-muted">smart</span>
                    )}
                  </span>
                  <span className="text-[10px] text-muted">{collection.itemCount}</span>
                </button>
              ))}
            </div>
            <div className="mt-1 flex gap-1 border-t border-[var(--line)] pt-2">
              <input
                value={creating}
                onChange={(event) => setCreating(event.currentTarget.value)}
                placeholder="New collection"
                className="input h-7 flex-1 px-2 py-1 text-[11px]"
                onKeyDown={async (event) => {
                  if (event.key !== "Enter" || !creating.trim()) return;
                  const created = await api.createCollection(creating.trim());
                  setCreating("");
                  await add(created.id);
                }}
              />
            </div>
          </div>
        </>
      )}
    </div>
  );
}
