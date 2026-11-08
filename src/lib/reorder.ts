import { useState } from "react";

export function moveItem<T>(items: T[], from: number, to: number): T[] {
  if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) {
    return items;
  }
  const next = [...items];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

export interface DragReorderHandlers {
  draggable: true;
  onDragStart: (event: React.DragEvent) => void;
  onDragOver: (event: React.DragEvent) => void;
  onDrop: (event: React.DragEvent) => void;
  onDragEnd: () => void;
}

export function useDragReorder(ids: string[], onChange: (next: string[]) => void) {
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);

  const reset = () => {
    setDragId(null);
    setOverId(null);
  };

  const itemProps = (id: string): DragReorderHandlers => ({
    draggable: true,
    onDragStart: (event) => {
      setDragId(id);
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", id);
    },
    onDragOver: (event) => {
      if (!dragId || dragId === id) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
      if (overId !== id) setOverId(id);
    },
    onDrop: (event) => {
      event.preventDefault();
      if (!dragId || dragId === id) {
        reset();
        return;
      }
      const from = ids.indexOf(dragId);
      const to = ids.indexOf(id);
      if (from >= 0 && to >= 0) onChange(moveItem(ids, from, to));
      reset();
    },
    onDragEnd: reset,
  });

  return { dragId, overId, itemProps };
}
