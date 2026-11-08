/**
 * Development-only mock bridge.
 *
 * The Linux CI/sandbox environment cannot render the Tauri webview reliably, so this
 * module lets the UI run in a plain browser against a JSON snapshot exported from the
 * real Rust repository. It is never bundled into the desktop app: `main.tsx` only
 * installs it when running under Vite dev *and* the URL asks for it.
 */

import mockData from "../mock/data.json";
import { setMockResolver } from "./api";

type Json = Record<string, any>;

const data = mockData as unknown as Json;

const store: Json = JSON.parse(JSON.stringify(data.library));
const settings = JSON.parse(JSON.stringify(data.settings));
const state = {
  bookmarks: {} as Record<string, Json[]>,
  progress: {} as Record<string, Json>,
  reveals: new Set<string>(),
};

function findIssue(id: string) {
  return (store.issues as Json[]).find((issue) => issue.id === id);
}

function isProtected(issue: Json) {
  if (!settings.spoilerProtection) return false;
  if (issue.status === "completed") return false;
  if (state.reveals.has(issue.id)) return false;
  const completed = (store.issues as Json[])
    .filter((entry) => entry.status === "completed" && entry.seriesId === issue.seriesId)
    .map((entry) => entry.sortNumber);
  const frontier = completed.length ? Math.max(...completed) : -1;
  return issue.sortNumber > frontier;
}

function decorate(issue: Json): Json {
  const base = {
    variantOf: issue.variantOf ?? null,
    edition: issue.edition ?? null,
    variantCount: (store.issues as Json[]).filter((entry) => entry.variantOf === issue.id).length,
  };
  if (isProtected(issue)) {
    return { ...issue, ...base, description: null, spoilerProtected: true };
  }
  return { ...issue, ...base, spoilerProtected: false };
}

function applyFilter(filter: Json = {}): Json[] {
  let issues: Json[] = [...(store.issues as Json[])];
  if (filter.search) {
    const needle = String(filter.search).toLowerCase();
    issues = issues.filter((issue: Json) =>
      [
        issue.title,
        issue.seriesTitle,
        issue.number,
        issue.writer,
        issue.artist,
        issue.publisher,
        issue.description,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }
  if (filter.status) issues = issues.filter((issue) => issue.status === filter.status);
  if (filter.universe) issues = issues.filter((issue) => issue.universe === filter.universe);
  if (filter.publisher) issues = issues.filter((issue) => issue.publisher === filter.publisher);
  if (filter.seriesId) issues = issues.filter((issue) => issue.seriesId === filter.seriesId);

  const sort = filter.sort ?? "recent";
  issues.sort((a: Json, b: Json) => {
    switch (sort) {
      case "series":
        return String(a.seriesTitle ?? "").localeCompare(String(b.seriesTitle ?? ""), undefined, {
          numeric: true,
        });
      case "title":
        return String(a.title).localeCompare(String(b.title));
      case "number":
        return a.sortNumber - b.sortNumber;
      case "release":
        return String(a.releaseDate ?? "9999").localeCompare(String(b.releaseDate ?? "9999"));
      case "added":
        return String(a.id).localeCompare(String(b.id));
      default: {
        const aRead = a.lastReadAt ? Date.parse(a.lastReadAt) : 0;
        const bRead = b.lastReadAt ? Date.parse(b.lastReadAt) : 0;
        return bRead - aRead;
      }
    }
  });
  return issues.map(decorate);
}

function recomputeSeries(): Json[] {
  const series = store.series as Json[];
  return series.map((entry) => {
    const issues = (store.issues as Json[]).filter((issue: Json) => issue.seriesId === entry.id);
    const overrides = ((entry.missingOverrides ?? {}) as Record<string, string>);
    return {
      ...entry,
      issueCount: issues.length,
      readCount: issues.filter((issue: Json) => issue.status === "completed").length,
      missingCount: computeMissing(issues, overrides).length,
    };
  });
}

function computeMissing(
  issues: Json[],
  overrides: Record<string, string>,
): { number: string; reason: string }[] {
  const numbers = issues
    .map((issue: Json) => Number(issue.number))
    .filter((value: number) => Number.isInteger(value) && value > 0)
    .sort((a: number, b: number) => a - b);
  const missing: { number: string; reason: string }[] = [];
  if (numbers.length > 1) {
    const present = new Set(numbers);
    for (let value = numbers[0]; value <= numbers[numbers.length - 1] && value - numbers[0] <= 500; value += 1) {
      if (present.has(value)) continue;
      if (overrides[String(value)] === "not-required") continue;
      missing.push({ number: String(value), reason: "missing" });
    }
  }
  for (const [number, reason] of Object.entries(overrides)) {
    if (reason === "not-required" || missing.some((entry) => entry.number === number)) continue;
    if (numbers.includes(Number(number))) continue;
    missing.push({ number, reason });
  }
  return missing;
}

function snapshot(filter: Json = {}) {
  const issues = applyFilter(filter);
  const totals = {
    issues: (store.issues as Json[]).length,
    series: (store.series as Json[]).length,
    completed: (store.issues as Json[]).filter((issue) => issue.status === "completed").length,
    inProgress: (store.issues as Json[]).filter((issue) => issue.status === "in-progress").length,
    unread: (store.issues as Json[]).filter((issue) => issue.status === "unread").length,
    pagesRead: (store.issues as Json[]).reduce((sum, issue) => sum + (issue.lastPage ?? 0), 0),
  };
  return {
    issues,
    series: recomputeSeries(),
    roots: store.roots,
    totals,
  };
}

function seriesDetail(id: string) {
  const series = recomputeSeries().find((entry) => entry.id === id);
  if (!series) throw new Error("Series not found");
  const issues = applyFilter({ seriesId: id }).sort((a, b) => a.sortNumber - b.sortNumber);
  const overrides = ((series.missingOverrides ?? {}) as Record<string, string>);
  const missing = computeMissing(issues, overrides);
  return { ...series, issues, missing };
}

function setProgress(issueId: string, page: number) {
  const issue = findIssue(issueId);
  if (!issue) return { page, total: 0, status: "unread" };
  const total = issue.pageCount;
  const ratio = total > 0 ? (page + 1) / total : 0;
  issue.lastPage = page;
  issue.lastReadAt = new Date().toISOString();
  if (ratio >= settings.completionThreshold || issue.status === "completed") {
    issue.status = "completed";
    issue.completedAt = issue.completedAt ?? new Date().toISOString();
  } else if (page > 0) {
    issue.status = "in-progress";
  }
  return { page, total, status: issue.status };
}

function findSeriesDirection(seriesId: string | null): string | null {
  if (!seriesId) return null;
  const series = (store.series as Json[]).find((entry: Json) => entry.id === seriesId);
  return (series?.readingDirection as string | null) ?? null;
}

function readerPayload(issueId: string) {
  const issue = findIssue(issueId);
  if (!issue) throw new Error("Issue not found");
  const base = data.readers[issueId];
  if (!base) throw new Error("Issue has no readable file");
  const pages = (base.pages as Json[]).map((page: Json, index: number) => ({
    ...page,
    name: `${index}.${String(page.name).split(".").pop()}`,
  }));
  const ordered = applyFilter({ sort: "number", seriesId: issue.seriesId }).filter(
    (entry: Json) => entry.seriesId === issue.seriesId,
  );
  // Dev affordance: let screenshots/test runs force a reader mode or direction.
  const params = new URLSearchParams(window.location.search);
  const position = ordered.findIndex((entry) => entry.id === issueId);
  return {
    ...base,
    pages,
    title: issue.title,
    number: issue.number,
    seriesTitle: issue.seriesTitle,
    mode: params.get("mode") ?? settings.defaultMode,
    fit: params.get("fit") ?? settings.defaultFit,
    transition: settings.transition,
    transitionDuration: settings.transitionDuration,
    background: settings.background,
    readingDirection:
      params.get("dir") ??
      (base.directionOverride as string | null) ??
      (findSeriesDirection(issue.seriesId) ?? settings.readingDirection),
    spreadOffset: Number(params.get("offset") ?? 0),
    showInterface: settings.showInterface,
    progress: {
      ...base.progress,
      page: issue.lastPage,
      total: issue.pageCount,
      status: issue.status,
      completedAt: issue.completedAt,
    },
    nextIssueId: ordered[position + 1]?.id ?? null,
    previousIssueId: ordered[position - 1]?.id ?? null,
  };
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value));
}

const handlers: Record<string, (args: Json) => unknown> = {
  library: (args) => snapshot(args.filter),
  library_roots: () => store.roots,
  filter_options: () => {
    const universes = [...new Set((store.issues as Json[]).map((issue) => issue.universe).filter(Boolean))];
    const publishers = [
      ...new Set((store.issues as Json[]).map((issue) => issue.publisher).filter(Boolean)),
    ];
    return { universes, publishers };
  },
  series_detail: (args) => seriesDetail(args.seriesId),
  issue_detail: (args) => decorate(findIssue(args.issueId) as Json),
  reader_payload: (args) => readerPayload(args.issueId),
  set_spread_offset: (args) => {
    const reader = data.readers[args.issueId];
    if (reader) reader.spreadOffset = args.offset;
    return args.offset;
  },
  set_direction_override: (args) => {
    const reader = data.readers[args.issueId];
    if (reader) reader.directionOverride = args.direction;
    return args.direction;
  },
  set_missing_reason: (args) => {
    const series = (store.series as Json[]).find((entry: Json) => entry.id === args.seriesId);
    if (!series) throw new Error("Series not found");
    const overrides = (series.missingOverrides ??= {});
    if (args.reason === null) delete overrides[args.number];
    else overrides[args.number] = args.reason;
    return null;
  },
  update_series: (args) => {
    const series = (store.series as Json[]).find((entry: Json) => entry.id === args.seriesId);
    if (!series) throw new Error("Series not found");
    for (const [key, value] of Object.entries(args.patch ?? {})) {
      if (value === undefined || value === "") continue;
      if (key === "coverFileId") {
        series.coverFileId = value;
        continue;
      }
      series[key] = value;
    }
    return seriesDetail(args.seriesId);
  },
  save_progress: (args) => {
    const result = setProgress(args.issueId, args.page);
    state.progress[args.issueId] = result;
    return result;
  },
  set_issue_status: (args) => {
    const issue = findIssue(args.issueId);
    if (!issue) throw new Error("Issue not found");
    issue.status = args.status;
    issue.lastPage = args.status === "completed" ? issue.pageCount - 1 : 0;
    issue.completedAt = args.status === "completed" ? new Date().toISOString() : null;
    return decorate(issue);
  },
  update_issue: (args) => {
    const issue = findIssue(args.issueId);
    if (!issue) throw new Error("Issue not found");
    Object.assign(issue, Object.fromEntries(Object.entries(args.patch).filter(([, v]) => v !== undefined && v !== "")));
    return decorate(issue);
  },
  bulk_update_issues: (args) => args.issueIds.map((id: string) => handlers.update_issue({ issueId: id, patch: args.patch })),
  reveal_spoiler: (args) => {
    state.reveals.add(args.issueId);
    return null;
  },
  bookmarks: (args) => state.bookmarks[args.issueId] ?? [],
  add_bookmark: (args) => {
    const list = state.bookmarks[args.issueId] ?? [];
    const bookmark = {
      id: `bm-${list.length + 1}-${Date.now()}`,
      page: args.page,
      label: args.label ?? null,
      note: args.note ?? null,
      createdAt: new Date().toISOString(),
    };
    state.bookmarks[args.issueId] = [...list, bookmark];
    return bookmark;
  },
  delete_bookmark: (args) => {
    for (const key of Object.keys(state.bookmarks)) {
      state.bookmarks[key] = state.bookmarks[key].filter((mark) => mark.id !== args.id);
    }
    return null;
  },
  update_bookmark_note: (args) => {
    for (const key of Object.keys(state.bookmarks)) {
      const mark = state.bookmarks[key].find((entry) => entry.id === args.id);
      if (mark) {
        mark.note = args.note ?? null;
        return mark;
      }
    }
    throw new Error("Bookmark not found");
  },
  search_library: (args) => {
    const needle = String(args.query).toLowerCase();
    const issues = applyFilter({ search: args.query });
    const series = recomputeSeries().filter((entry: Json) =>
      String(entry.title).toLowerCase().includes(needle),
    );
    return { issues, series, total: issues.length + series.length };
  },
  search_suggestions: (args) => {
    const needle = String(args.query).toLowerCase();
    return [...new Set((store.series as Json[]).map((entry: Json) => entry.title as string))]
      .filter((title: string) => title.toLowerCase().startsWith(needle))
      .slice(0, 5)
      .map((value: string) => ({ value, kind: "series" }));
  },
  saved_searches: () => [],
  save_search: (args) => ({
    id: `ss-${Date.now()}`,
    name: args.name,
    query: args.query,
    createdAt: new Date().toISOString(),
  }),
  delete_saved_search: () => null,
  reading_orders: () => clone(data.orders),
  reading_order: (args) => {
    const detail = data.orderDetails[args.orderId];
    if (detail) return clone(detail);
    throw new Error("Order not found");
  },
  create_reading_order: (args) => ({
    id: `order-${Date.now()}`,
    name: args.name,
    description: args.description ?? null,
    kind: args.kind ?? "chronological",
    itemCount: 0,
    createdAt: new Date().toISOString(),
  }),
  generate_publication_order: (args) => {
    const issues = [...(store.issues as Json[])].sort((a: Json, b: Json) => {
      const dateA = a.releaseDate ?? "9999";
      const dateB = b.releaseDate ?? "9999";
      return String(dateA).localeCompare(String(dateB)) || a.sortNumber - b.sortNumber;
    });
    return {
      id: `order-${Date.now()}`,
      name: args.name,
      description: "Generated from release dates",
      kind: "publication",
      itemCount: issues.length,
      createdAt: new Date().toISOString(),
      items: issues.map((issue: Json, index: number) => ({
        issueId: issue.id,
        position: index,
        issue: decorate(issue),
      })),
    };
  },
  delete_reading_order: () => null,
  add_order_items: () => null,
  reorder_items: () => null,
  next_in_order: (args) => {
    const detail = data.orderDetails[args.orderId];
    if (!detail) return null;
    const index = detail.items.findIndex((item: Json) => item.issueId === args.issueId);
    const next = detail.items[index + 1];
    if (!next?.issue) return null;
    return {
      id: next.issue.id,
      title: next.issue.title,
      number: next.issue.number,
      seriesTitle: next.issue.seriesTitle,
      fileId: next.issue.fileId,
    };
  },
  collections: () => clone(data.collections),
  collection: (args) => {
    const detail = data.collectionDetails[args.collectionId];
    if (detail) return clone(detail);
    throw new Error("Collection not found");
  },
  create_collection: (args) => ({
    id: `col-${Date.now()}`,
    name: args.name,
    description: args.description ?? null,
    coverFileId: null,
    itemCount: 0,
    createdAt: new Date().toISOString(),
  }),
  update_collection: (args) => {
    const existing = (data.collections as Json[]).find(
      (entry) => entry.id === args.collectionId,
    );
    return {
      id: args.collectionId,
      name: args.name ?? existing?.name ?? "Collection",
      description: args.description ?? existing?.description ?? null,
      coverFileId: args.coverFileId ?? existing?.coverFileId ?? null,
      itemCount: existing?.itemCount ?? 0,
      createdAt: existing?.createdAt ?? new Date().toISOString(),
    };
  },
  delete_collection: () => null,
  add_to_collection: () => null,
  add_series_to_collection: () => null,
  remove_from_collection: () => null,
  reorder_collection: () => null,
  get_settings: () => {
    // Dev affordance for theme screenshots: ?mock&theme=light|midnight.
    // Keep the stored settings and the returned view in sync so the Settings
    // theme buttons highlight the theme that is actually rendered.
    const params = new URLSearchParams(window.location.search);
    const theme = params.get("theme");
    if (theme && theme !== settings.theme) settings.theme = theme;
    return settings;
  },
  save_settings: (args) => {
    Object.assign(settings, args.settings);
    return settings;
  },
  stats: () => {
    const issues = store.issues as Json[];
    return {
      ...data.stats,
      totalIssues: issues.length,
      completed: issues.filter((issue) => issue.status === "completed").length,
      inProgress: issues.filter((issue) => issue.status === "in-progress").length,
      totalPagesRead: issues.reduce((sum, issue) => sum + (issue.lastPage ?? 0), 0),
    };
  },
  reading_history: () => {
    const recent = (store.issues as Json[])
      .filter((issue) => issue.lastReadAt)
      .sort((a, b) => String(b.lastReadAt).localeCompare(String(a.lastReadAt)))
      .slice(0, 24)
      .map(decorate);
    const sessions = (data.history?.sessions ?? []).map((session: Json) => ({ ...session }));
    return { sessions, recent };
  },
  export_library: () => ({ version: 1, issues: store.issues }),
  export_library_to: () => null,
  record_reading_session: () => null,
};

export function installMockBridge() {
  setMockResolver(mockAsset);
  const internals = ((window as unknown as Json).__TAURI_INTERNALS__ ??= {});
  internals.invoke = async (command: string, args: Json = {}) => {
    const handler = handlers[command];
    if (!handler) {
      console.warn(`[mock bridge] unhandled command: ${command}`);
      return null;
    }
    await new Promise((resolve) => setTimeout(resolve, 60));
    return handler(args);
  };
  internals.convertFileSrc = (path: string) => path;
  internals.metadata = {
    currentWindow: { label: "main" },
    currentWebview: { label: "main", windowLabel: "main" },
  };
}

export function mockAsset(url: string): string {
  const match = url.match(/\/(?:page|cover)\/([^/]+)\/(\d+)/);
  if (!match) return url;
  const reader = data.readers as Json;
  const entry = Object.entries(reader).find(([, payload]) => payload.fileId === match[1]);
  if (!entry) return url;
  const page = Number(match[2]);
  const pageInfo = entry[1].pages[page];
  if (!pageInfo) return url;
  const ext = String(pageInfo.name).split(".").pop();
  return `/mock-pages/${match[1]}/${page}.${ext}`;
}


