export function formatDate(value: string | null): string {
  if (!value) return "Unknown";
  const date = new Date(value.length === 4 ? `${value}-01-01` : value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: value.length === 4 ? undefined : "numeric",
  });
}

export function formatRelative(value: string | null): string {
  if (!value) return "Never";
  const date = new Date(value);
  const diff = Date.now() - date.getTime();
  const minutes = Math.round(diff / 60000);
  if (minutes < 1) return "Just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days}d ago`;
  return date.toLocaleDateString();
}

export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}

export function progressRatio(issue: { status: string; lastPage: number; pageCount: number }) {
  if (issue.status === "completed") return 1;
  if (issue.pageCount <= 0) return 0;
  return Math.min(1, Math.max(0, issue.lastPage / issue.pageCount));
}

export function statusLabel(status: string): string {
  switch (status) {
    case "completed":
      return "Completed";
    case "in-progress":
      return "In progress";
    default:
      return "Unread";
  }
}

export function naturalCompare(a: string, b: string) {
  return a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });
}

export function cx(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}

interface KeyEventLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export function eventMatches(event: KeyEventLike, binding: string | undefined): boolean {
  if (!binding) return false;
  const combos = binding
    .split(",")
    .map((combo) => combo.trim())
    .filter(Boolean);
  return combos.some((combo) => {
    const parts = combo.split("+").map((part) => part.trim().toLowerCase());
    let ctrl = false;
    let shift = false;
    let alt = false;
    let key = "";
    for (const part of parts) {
      if (part === "ctrl" || part === "cmd" || part === "meta") ctrl = true;
      else if (part === "shift") shift = true;
      else if (part === "alt") alt = true;
      else key = part;
    }
    if (ctrl !== (event.ctrlKey || event.metaKey)) return false;
    if (shift !== event.shiftKey) return false;
    if (alt !== event.altKey) return false;
    const normalized = key === "space" ? " " : key === "esc" ? "escape" : key;
    return event.key.toLowerCase() === normalized;
  });
}

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  return (
    tag === "input" ||
    tag === "textarea" ||
    tag === "select" ||
    Boolean(target.isContentEditable)
  );
}

export function truncate(value: string, length: number) {
  if (value.length <= length) return value;
  return `${value.slice(0, length - 1)}…`;
}

export function pluralize(count: number, singular: string, plural?: string) {
  return `${count} ${count === 1 ? singular : plural ?? `${singular}s`}`;
}

export function readingDirectionLabel(value: string) {
  return value === "rtl" ? "Right to left" : "Left to right";
}

export function fitLabel(fit: string) {
  switch (fit) {
    case "width":
      return "Fit width";
    case "height":
      return "Fit height";
    case "original":
      return "Original size";
    default:
      return "Fit page";
  }
}

export function modeLabel(mode: string) {
  switch (mode) {
    case "double":
      return "Double page";
    case "vertical":
      return "Vertical scroll";
    case "horizontal":
      return "Horizontal strip";
    default:
      return "Single page";
  }
}
