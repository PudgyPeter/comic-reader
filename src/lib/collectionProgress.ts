export interface ProgressItem {
  status: string;
  lastPage: number;
  pageCount: number;
}

export function collectionProgress(items: ProgressItem[]): number {
  if (items.length === 0) return 0;
  const total = items.reduce((sum, item) => {
    if (item.status === "completed") return sum + 1;
    if (item.pageCount <= 0) return sum;
    return sum + Math.min(1, item.lastPage / item.pageCount);
  }, 0);
  return total / items.length;
}
