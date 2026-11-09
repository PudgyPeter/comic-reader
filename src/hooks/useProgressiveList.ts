import { useEffect, useRef, useState } from "react";

/**
 * Renders a growing slice of a large list instead of mounting every item.
 *
 * The library grid previously mapped all issues, which is fine for dozens and
 * fatal for thousands. This keeps the DOM bounded by page size while preserving
 * normal document flow: a sentinel grows the slice as it scrolls into view.
 */
export function useProgressiveList<T>(items: T[], resetKey: string, pageSize = 120) {
  const [state, setState] = useState({ key: resetKey, count: pageSize });
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  // Adjust state during render when the filters change, rather than in an
  // effect, so the first paint already shows the correct slice.
  if (state.key !== resetKey) {
    setState({ key: resetKey, count: pageSize });
  }

  const total = items.length;
  const count = state.key === resetKey ? state.count : pageSize;
  const visible = count >= total ? items : items.slice(0, count);
  const hasMore = count < total;

  useEffect(() => {
    if (!hasMore) return;
    const node = sentinelRef.current;
    if (!node || typeof IntersectionObserver === "undefined") {
      setState((current) => ({ ...current, count: total }));
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setState((current) => ({
            ...current,
            count: Math.min(total, current.count + pageSize),
          }));
        }
      },
      { rootMargin: "600px 0px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [hasMore, total, pageSize, count]);

  return { visible, sentinelRef, hasMore, total };
}
