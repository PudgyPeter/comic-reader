/**
 * Development-only mock bridge.
 *
 * The Linux CI/sandbox environment cannot render the Tauri webview reliably, so this
 * module lets the UI run in a plain browser against a JSON snapshot exported from the
 * real Rust repository. It is never bundled into the desktop app: `main.tsx` only
 * installs it when running under Vite dev *and* the URL asks for it.
 */

import { setMockResolver } from "./api";

type Json = Record<string, any>;

// The fixture is generated and git-ignored, so it must not be a static import:
// a clean clone has no `src/mock/data.json` and `tsc` would fail before the
// first export. `import.meta.glob` resolves to `{}` when the file is absent,
// which keeps typecheck/build green and lets the harness show an empty library.
const fixtures = import.meta.glob<{ default: Json }>("../mock/data.json", { eager: true });
const emptyFixture: Json = {
  library: { issues: [], series: [], roots: [], totals: {} },
  settings: {},
  readers: {},
};
const data = (Object.values(fixtures)[0]?.default ?? emptyFixture) as Json;

const store: Json = JSON.parse(JSON.stringify(data.library));
const settings = JSON.parse(JSON.stringify(data.settings));
const state = {
  bookmarks: {} as Record<string, Json[]>,
  progress: {} as Record<string, Json>,
  reveals: new Set<string>(),
  fileBookmarks: {} as Record<string, Json[]>,
  fileProgress: {} as Record<string, Json>,
};

// The exported fixture only contains matched issues, so synthesise one unmatched
// file from the first reader payload. That keeps the dev harness showing a real
// file (readable pages, progress) instead of an empty state.
const mockFile = (() => {
  const exported = (data.unmatchedFiles as Json[] | undefined)?.[0];
  if (exported) {
    const reader = Object.values(data.readers as Json).find(
      (payload) => payload.fileId === exported.id,
    );
    return reader ? { ...exported, payload: reader } : null;
  }
  const entry = Object.entries(data.readers as Json)[0];
  if (!entry) return null;
  const [issueId, payload] = entry;
  const issue = findIssue(issueId);
  const path = String(issue?.filePath ?? payload.fileId);
  const stem = path.split(/[\\/]/).pop() ?? "Unmatched file";
  return {
    id: String(payload.fileId),
    rootId: (store.roots as Json[])[0]?.id ?? null,
    path,
    relativePath: stem,
    format: String(issue?.format ?? "cbz"),
    sizeBytes: 12_400_000,
    pageCount: (payload.pages as Json[]).length,
    status: "present",
    addedAt: new Date().toISOString(),
    page: 0,
    progressStatus: "unread",
    lastReadAt: null,
    suggestedSeries: String(stem).replace(/\.[^.]+$/, "").replace(/[_-]+/g, " "),
    suggestedNumber: "1",
    suggestedTitle: String(stem).replace(/\.[^.]+$/, ""),
    payload,
  };
})();
const mockSessions = new Set<string>();

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
    metadataLocked: issue.metadataLocked ?? false,
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

function smartRuleMatches(issue: Json, rule: Json): boolean {
  const valueOf = (field: string): string => {
    switch (field) {
      case "series":
        return String(issue.seriesTitle ?? "");
      case "status":
        return String(issue.status ?? "unread");
      case "format":
        return String(issue.format ?? "");
      case "year":
        return String(issue.releaseDate ?? "").slice(0, 4);
      case "pages":
        return String(issue.pageCount ?? 0);
      case "lastPage":
        return String(issue.lastPage ?? 0);
      default:
        return String(issue[field] ?? "");
    }
  };
  const actual = valueOf(rule.field);
  const expected = String(rule.value ?? "");
  const numeric = ["year", "pages", "lastPage"].includes(rule.field);
  const actualNumber = Number(actual);
  const expectedNumber = Number(expected);
  switch (rule.op) {
    case "contains":
      return actual.toLowerCase().includes(expected.toLowerCase());
    case "startsWith":
      return actual.toLowerCase().startsWith(expected.toLowerCase());
    case "eq":
      return numeric ? actualNumber === expectedNumber : actual === expected;
    case "ne":
      return actual !== expected;
    case "gt":
      return numeric ? actualNumber > expectedNumber : actual > expected;
    case "gte":
      return numeric ? actualNumber >= expectedNumber : actual >= expected;
    case "lt":
      return numeric ? actualNumber < expectedNumber : actual < expected;
    case "lte":
      return numeric ? actualNumber <= expectedNumber : actual <= expected;
    case "isSet":
      return actual.trim() !== "";
    case "isEmpty":
      return actual.trim() === "";
    default:
      return false;
  }
}

function applySmartRules(rules: Json): Json[] {
  const all = (rules.all ?? []) as Json[];
  const any = (rules.any ?? []) as Json[];
  if (all.length === 0 && any.length === 0) return [];
  return (store.issues as Json[])
    .filter(
      (issue: Json) =>
        all.every((rule: Json) => smartRuleMatches(issue, rule)) &&
        (any.length === 0 || any.some((rule: Json) => smartRuleMatches(issue, rule))),
    )
    .map(decorate);
}

function recomputeSeries(): Json[] {
  const series = store.series as Json[];
  return series.map((entry) => {
    const issues = (store.issues as Json[]).filter((issue: Json) => issue.seriesId === entry.id);
    const overrides = ((entry.missingOverrides ?? {}) as Record<string, string>);
    return {
      ...entry,
      metadataLocked: entry.metadataLocked ?? false,
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
  if (!issue) return { page, total: 0, status: "unread", scrollOffset: 0 };
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
  return { page, total, status: issue.status, scrollOffset: 0 };
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

// The exported fixture has no character rows, so synthesise a small roster from
// the mock series. This keeps the roster screens non-empty in the dev harness
// without implying the real importer invents characters.
const mockCharacters: Json[] = (store.series as Json[]).slice(0, 6).map((series, index) => ({
  id: `mock-character-${index}`,
  seriesId: series.id,
  seriesTitle: series.title,
  name: ["Sonic the Hedgehog", "Miles \"Tails\" Prower", "Knuckles the Echidna", "Amy Rose", "Dr. Eggman", "Shadow the Hedgehog"][index] ?? `Character ${index + 1}`,
  description: index % 2 === 0 ? null : "Mock roster entry used to render the character screens.",
  descriptionSource: index % 2 === 0 ? "comic" : "manual",
  sourceUrl: null,
  appearanceCount: (index + 1) * 3,
  readCount: index,
  descriptionUpdatedAt: null,
}));

const handlers: Record<string, (args: Json) => unknown> = {
  library: (args) => snapshot(args.filter),
  library_roots: () => store.roots,
  filter_options: () => {
    const universes = [...new Set((store.issues as Json[]).map((issue) => issue.universe).filter(Boolean))];
    const publishers = [
      ...new Set((store.issues as Json[]).map((issue) => issue.publisher).filter(Boolean)),
    ];
    const formats = [...new Set((store.issues as Json[]).map((issue) => issue.format).filter(Boolean))];
    const series = (store.series as Json[]).map((entry) => ({
      id: entry.id,
      title: entry.title,
    }));
    const years = (store.issues as Json[])
      .map((issue) => Number(String(issue.releaseDate ?? "").slice(0, 4)))
      .filter((year) => Number.isFinite(year) && year > 0)
      .sort((a, b) => a - b);
    return {
      universes,
      publishers,
      formats,
      series,
      yearMin: years[0] ?? null,
      yearMax: years[years.length - 1] ?? null,
    };
  },
  series_detail: (args) => seriesDetail(args.seriesId),
  series_characters: (args) =>
    mockCharacters.filter((entry) => entry.seriesId === args.seriesId),
  all_characters: () => mockCharacters,
  sync_series_characters: () => ({
    seriesId: "",
    characters: mockCharacters.length,
    appearances: mockCharacters.reduce((sum, c) => sum + c.appearanceCount, 0),
    issuesScanned: 0,
    issuesWithoutMetadata: 0,
  }),
  character_detail: (args) => {
    const card = mockCharacters.find((entry) => entry.id === args.characterId);
    if (!card) throw new Error("Character not found");
    return { ...card, appearances: [] };
  },
  update_character: (args) => {
    const card = mockCharacters.find((entry) => entry.id === args.characterId);
    if (!card) throw new Error("Character not found");
    card.description = args.patch?.description ?? card.description;
    card.descriptionSource = args.patch?.descriptionSource ?? "manual";
    return card;
  },
  delete_character: () => null,
  lookup_character: () => ({
    title: "Example",
    extract: "Mock summaries are not fetched from the web.",
    url: "https://example.invalid",
    source: "Mock",
  }),
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
  set_series_direction: (args) => {
    const issues = (store.issues as Json[]).filter(
      (issue: Json) => issue.seriesId === args.seriesId,
    );
    for (const issue of issues) {
      const reader = data.readers[issue.id];
      if (reader) reader.directionOverride = args.direction;
    }
    return issues.length;
  },
  set_series_status: (args) => {
    const issues = (store.issues as Json[]).filter(
      (issue: Json) => issue.seriesId === args.seriesId,
    );
    for (const issue of issues) {
      issue.status = args.status;
      issue.lastPage = args.status === "completed" ? issue.pageCount - 1 : 0;
      issue.completedAt = args.status === "completed" ? new Date().toISOString() : null;
    }
    return issues.length;
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
    if (args.patch?.metadataLocked === undefined) series.metadataLocked = true;
    return seriesDetail(args.seriesId);
  },
  save_progress: (args) => {
    const result = setProgress(args.issueId, args.page);
    if (typeof args.scrollOffset === "number") result.scrollOffset = args.scrollOffset;
    state.progress[args.issueId] = result;
    return result;
  },
  mark_issue_to_page: (args) => {
    const issue = findIssue(args.issueId);
    if (!issue) throw new Error("Issue not found");
    const total = issue.pageCount as number;
    const page = Math.max(0, Math.min(total - 1, Number(args.page)));
    issue.lastPage = page;
    issue.status =
      issue.status === "completed" || page >= total - 1 ? "completed" : "in-progress";
    issue.completedAt =
      issue.status === "completed"
        ? (issue.completedAt ?? new Date().toISOString())
        : null;
    return { page, total, status: issue.status, scrollOffset: 0 };
  },
  start_reading_session: (args) => {
    const id = `mock-session-${Date.now()}`;
    const issue = findIssue(args.issueId);
    if (issue) {
      const sessions = ((data.history ??= {}).sessions ??= []) as Json[];
      sessions.unshift({
        id,
        issueId: args.issueId,
        title: issue.title,
        seriesTitle: issue.seriesTitle ?? null,
        number: issue.number ?? "",
        fileId: issue.fileId ?? null,
        startedAt: new Date().toISOString(),
        endedAt: new Date().toISOString(),
        pages: 0,
        lastPage: Number(args.page ?? issue.lastPage ?? 0),
        status: "open",
        resumedAt: null,
      });
    }
    mockSessions.add(id);
    return id;
  },
  touch_reading_session: (args) => {
    if (mockSessions.has(args.sessionId)) {
      const sessions = (data.history?.sessions ?? []) as Json[];
      const session = sessions.find((entry) => entry.id === args.sessionId);
      if (session) {
        session.pages = args.pages;
        session.lastPage = args.lastPage;
        session.endedAt = new Date().toISOString();
      }
    }
    return null;
  },
  close_reading_session: (args) => {
    const sessions = (data.history?.sessions ?? []) as Json[];
    const session = sessions.find((entry) => entry.id === args.sessionId);
    if (session) {
      session.pages = args.pages;
      session.lastPage = args.lastPage;
      session.endedAt = new Date().toISOString();
      session.status = "closed";
    }
    mockSessions.delete(args.sessionId);
    return null;
  },
  resume_reading_session: (args) => {
    const sessions = (data.history?.sessions ?? []) as Json[];
    const session = sessions.find((entry) => entry.id === args.sessionId);
    if (session) {
      session.status = "closed";
      session.resumedAt = new Date().toISOString();
    }
    return null;
  },
  issue_variants: (args) => {
    const issue = findIssue(args.issueId);
    if (!issue) return [];
    const base = (issue.variantOf as string | null) ?? issue.id;
    return (store.issues as Json[])
      .filter((entry) => entry.id === base || entry.variantOf === base)
      .map(decorate);
  },
  preview_import: () => [
    {
      path: "/mock/Sonic the Hedgehog 021.cbz",
      relativePath: "Sonic the Hedgehog 021.cbz",
      format: "cbz",
      pageCount: 20,
      seriesTitle: "Sonic the Hedgehog",
      title: "Sonic the Hedgehog #21",
      number: "21",
      volume: null,
      publisher: "IDW Publishing",
      releaseDate: "2019",
      writer: null,
      artist: null,
      coverArtist: null,
      description: null,
      duplicate: false,
      warning: null,
    },
    {
      path: "/mock/Sonic the Hedgehog 022.cbz",
      relativePath: "Sonic the Hedgehog 022.cbz",
      format: "cbz",
      pageCount: 1,
      seriesTitle: "Sonic the Hedgehog",
      title: "Sonic the Hedgehog #22",
      number: "22",
      volume: null,
      publisher: "IDW Publishing",
      releaseDate: "2019",
      writer: null,
      artist: null,
      coverArtist: null,
      description: null,
      duplicate: true,
      warning: "Archive contains a single page",
    },
  ],
  commit_import: () => ({
    root: { id: "mock", path: "/mock", addedAt: new Date().toISOString(), fileCount: 0 },
    filesImported: 0,
    filesSkipped: 0,
    seriesCreated: 0,
    issuesCreated: 0,
    warnings: [],
  }),
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
    if (args.patch.metadataLocked === undefined) issue.metadataLocked = true;
    return decorate(issue);
  },
  bulk_update_issues: (args) =>
    args.issueIds
      .filter((id: string) => args.patch.metadataLocked !== undefined || !findIssue(id)?.metadataLocked)
      .map((id: string) => handlers.update_issue({ issueId: id, patch: args.patch })),
  refresh_issue_metadata: (args) => {
    const issue = findIssue(args.issueId);
    if (!issue) throw new Error("Issue not found");
    return args.force === true || !issue.metadataLocked;
  },
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
  split_available: () => true,
  propose_split: (args) => {
    if (!mockFile || mockFile.id !== args.fileId) throw new Error("File not found");
    const total = Number(mockFile.pageCount);
    const step = Math.max(1, Math.floor(total / 4));
    const titles = ["Fallout", "The Chase", "Metal Virus", "Out of the Blue"];
    const entries = titles.map((title, index) => ({
      title: `Chapter ${index + 1} — ${title}`,
      number: String(index + 1),
      printedPage: index * step + 1,
      startPage: index * step,
      endPage: index === titles.length - 1 ? total : (index + 1) * step,
    }));
    return {
      fileId: mockFile.id,
      tocPage: 4,
      pageCount: total,
      offset: 0,
      entries,
    };
  },
  recombine_split: (args) => {
    if (!mockFile) throw new Error("No file");
    const total = Number(mockFile.pageCount);
    const step = Math.max(1, Math.floor(total / 4));
    return {
      fileId: args.fileId,
      tocPage: 4,
      pageCount: total,
      offset: args.offset,
      entries: (args.entries as Json[]).map((entry, index) => ({
        ...entry,
        number: String(index + 1),
        startPage: index * step,
        endPage: index === args.entries.length - 1 ? total : (index + 1) * step,
      })),
    };
  },
  apply_split: (args) => {
    const ranges = args.request?.ranges ?? [];
    return (ranges as Json[]).map((range, index) => ({
      id: `split-${Date.now()}-${index}`,
      seriesId: null,
      seriesTitle: args.request?.seriesTitle ?? null,
      title: range.title,
      number: range.number,
      sortNumber: index + 1,
      pageCount: range.endPage - range.startPage,
      pageOffset: range.startPage,
      fileId: mockFile?.id ?? null,
      filePath: mockFile?.path ?? null,
      status: "unread",
      lastPage: 0,
      metadataLocked: false,
      variantCount: 0,
      missing: false,
      spoilerProtected: false,
    }));
  },
  unmatched_files: () => {
    if (!mockFile) return [];
    const progress = state.fileProgress[String(mockFile.id)];
    return [
      {
        ...mockFile,
        page: progress?.page ?? 0,
        progressStatus: progress?.status ?? "unread",
        lastReadAt: progress?.updatedAt ?? null,
      },
    ];
  },
  file_proposal: (args) => {
    if (!mockFile || mockFile.id !== args.fileId) throw new Error("File not found");
    return {
      fileId: mockFile.id,
      filePath: mockFile.path,
      format: mockFile.format,
      pageCount: mockFile.pageCount,
      seriesTitle: mockFile.suggestedSeries,
      title: mockFile.suggestedTitle,
      number: mockFile.suggestedNumber,
      volume: null,
      publisher: null,
      releaseDate: null,
      writer: null,
      artist: null,
      coverArtist: null,
      description: null,
    };
  },
  fileless_issues: () => [],
  apply_file_match: (args) => {
    if (!mockFile) throw new Error("No unmatched file");
    const series = (store.series as Json[]).find(
      (entry) => entry.id === args.patch?.seriesId,
    );
    const issue = {
      ...mockFile.payload,
      id: `matched-${Date.now()}`,
      seriesId: series?.id ?? (store.series as Json[])[0]?.id ?? null,
      seriesTitle: args.patch?.seriesTitle ?? series?.title ?? mockFile.suggestedSeries,
      title: args.patch?.title ?? mockFile.suggestedTitle,
      number: args.patch?.number ?? mockFile.suggestedNumber,
      status: "unread",
      lastPage: 0,
      pageCount: mockFile.pageCount,
      fileId: mockFile.id,
      filePath: mockFile.path,
    };
    (store.issues as Json[]).push(issue);
    return decorate(issue);
  },
  file_reader_payload: (args) => {
    if (!mockFile || mockFile.id !== args.fileId) throw new Error("File not found");
    const progress = state.fileProgress[String(mockFile.id)];
    return {
      ...mockFile.payload,
      issueId: null,
      title: mockFile.suggestedTitle,
      seriesTitle: null,
      number: "",
      nextIssueId: null,
      previousIssueId: null,
      progress: {
        page: progress?.page ?? 0,
        total: mockFile.pageCount,
        status: progress?.status ?? "unread",
        startedAt: null,
        completedAt: null,
        updatedAt: progress?.updatedAt ?? null,
        scrollOffset: progress?.scrollOffset ?? 0,
      },
    };
  },
  save_file_progress: (args) => {
    const total = mockFile?.pageCount ?? 0;
    const ratio = total > 0 ? (args.page + 1) / total : 0;
    const previous = state.fileProgress[args.fileId];
    const result = {
      page: args.page,
      total,
      status:
        previous?.status === "completed" || ratio >= settings.completionThreshold
          ? "completed"
          : args.page > 0
            ? "in-progress"
            : "unread",
      startedAt: previous?.startedAt ?? new Date().toISOString(),
      completedAt: null,
      updatedAt: new Date().toISOString(),
      scrollOffset:
        typeof args.scrollOffset === "number"
          ? args.scrollOffset
          : (previous?.scrollOffset ?? 0),
    };
    state.fileProgress[args.fileId] = result;
    return result;
  },
  file_bookmarks: (args) => state.fileBookmarks[args.fileId] ?? [],
  add_file_bookmark: (args) => {
    const list = state.fileBookmarks[args.fileId] ?? [];
    const bookmark = {
      id: `fbm-${list.length + 1}-${Date.now()}`,
      page: args.page,
      label: args.label ?? null,
      note: args.note ?? null,
      createdAt: new Date().toISOString(),
    };
    state.fileBookmarks[args.fileId] = [...list, bookmark];
    return bookmark;
  },
  delete_file_bookmark: (args) => {
    for (const key of Object.keys(state.fileBookmarks)) {
      state.fileBookmarks[key] = state.fileBookmarks[key].filter(
        (mark) => mark.id !== args.id,
      );
    }
    return null;
  },
  update_file_bookmark_note: (args) => {
    for (const key of Object.keys(state.fileBookmarks)) {
      const mark = state.fileBookmarks[key].find((entry) => entry.id === args.id);
      if (mark) {
        mark.note = args.note ?? null;
        return mark;
      }
    }
    throw new Error("Bookmark not found");
  },
  set_file_spread_offset: (args) => {
    if (mockFile) mockFile.payload.spreadOffset = args.offset;
    return args.offset;
  },
  set_file_direction_override: (args) => {
    if (mockFile) mockFile.payload.directionOverride = args.direction;
    return args.direction;
  },
  search_library: (args) => {
    const needle = String(args.query).toLowerCase();
    const issues = applyFilter({ search: args.query });
    const series = recomputeSeries().filter((entry: Json) =>
      String(entry.title).toLowerCase().includes(needle),
    );
    return {
      issues,
      series,
      total: issues.length + series.length,
      snippets: {},
      nextCursor: null,
    };
  },
  search_filtered: (args) => {
    const filter = args.filter ?? {};
    const needle = String(filter.query ?? "").toLowerCase();
    let issues = applyFilter({ search: filter.query ?? "" });
    if (filter.publisher) {
      issues = issues.filter((issue: Json) => issue.publisher === filter.publisher);
    }
    if (filter.seriesId) {
      issues = issues.filter((issue: Json) => issue.seriesId === filter.seriesId);
    }
    if (filter.status) {
      issues = issues.filter((issue: Json) => issue.status === filter.status);
    }
    if (filter.yearFrom) {
      issues = issues.filter(
        (issue: Json) => Number(String(issue.releaseDate ?? "").slice(0, 4)) >= filter.yearFrom,
      );
    }
    if (filter.yearTo) {
      issues = issues.filter(
        (issue: Json) => Number(String(issue.releaseDate ?? "").slice(0, 4)) <= filter.yearTo,
      );
    }
    if (filter.sort === "title") {
      issues = [...issues].sort((a: Json, b: Json) =>
        String(a.title).localeCompare(String(b.title)),
      );
    }
    const offset = filter.cursor ? Number(filter.cursor) : 0;
    const limit = Number(filter.limit ?? 40);
    const page = issues.slice(offset, offset + limit);
    const nextCursor = offset + limit < issues.length ? String(offset + limit) : null;
    const snippets: Record<string, string> = {};
    if (needle) {
      for (const issue of page) {
        const haystack = `${issue.title} ${issue.description ?? ""}`;
        if (haystack.toLowerCase().includes(needle)) {
          const at = haystack.toLowerCase().indexOf(needle);
          const start = Math.max(0, at - 24);
          snippets[issue.id] = `…${haystack.slice(start, at + needle.length + 40)}…`;
        }
      }
    }
    const series = recomputeSeries().filter((entry: Json) =>
      String(entry.title).toLowerCase().includes(needle),
    );
    if (needle) {
      const searches = (data.recentSearches ??= []) as Json[];
      const existing = searches.find((entry) => entry.query === filter.query);
      if (existing) existing.searchedAt = new Date().toISOString();
      else searches.unshift({ query: filter.query, searchedAt: new Date().toISOString() });
    }
    return {
      issues: page,
      series: offset === 0 ? series.slice(0, 8) : [],
      total: page.length + (offset === 0 ? series.length : 0),
      snippets,
      nextCursor,
    };
  },
  recent_searches: () => (data.recentSearches ?? []) as Json[],
  clear_search_history: () => {
    data.recentSearches = [];
    return null;
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
    isSmart: false,
    smartRules: null,
    sortMode: "manual",
  }),
  create_smart_collection: (args) => {
    const id = `smart-${Date.now()}`;
    const issues = applySmartRules(args.rules ?? { all: [], any: [] });
    const collection = {
      id,
      name: args.name,
      description: args.description ?? null,
      coverFileId: null,
      itemCount: issues.length,
      createdAt: new Date().toISOString(),
      isSmart: true,
      smartRules: args.rules,
      sortMode: args.sortMode ?? "series",
    };
    data.collections = [collection, ...((data.collections as Json[]) ?? [])];
    data.collectionDetails = {
      ...(data.collectionDetails as Json),
      [id]: { ...collection, items: issues },
    };
    return collection;
  },
  update_collection: (args) => {
    const existing = (data.collections as Json[]).find(
      (entry) => entry.id === args.collectionId,
    );
    const patch = args.patch ?? {};
    const merged = {
      id: args.collectionId,
      name: patch.name ?? existing?.name ?? "Collection",
      description: patch.description ?? existing?.description ?? null,
      coverFileId: patch.coverFileId ?? existing?.coverFileId ?? null,
      itemCount: existing?.itemCount ?? 0,
      createdAt: existing?.createdAt ?? new Date().toISOString(),
      isSmart: patch.isSmart ?? existing?.isSmart ?? false,
      smartRules: patch.smartRules ?? existing?.smartRules ?? null,
      sortMode: patch.sortMode ?? existing?.sortMode ?? "manual",
    };
    if (existing && merged.isSmart && merged.smartRules) {
      merged.itemCount = applySmartRules(merged.smartRules).length;
    }
    if (existing) Object.assign(existing, merged);
    return merged;
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
    const all = (data.history?.sessions ?? []).map((session: Json) => {
      const issue = findIssue(session.issueId);
      return {
        ...session,
        lastPage: session.lastPage ?? issue?.lastPage ?? 0,
        status: session.status ?? "closed",
        resumedAt: session.resumedAt ?? null,
      };
    });
    const sessions = all.filter((session: Json) => session.status !== "open");
    const interrupted = all.filter((session: Json) => session.status === "open");
    // The exported fixture has no interrupted sessions; synthesise one from a
    // part-read issue so the resume panel is previewable in the mock harness.
    if (interrupted.length === 0) {
      const candidate = (store.issues as Json[]).find(
        (issue) => issue.status === "in-progress" && issue.lastPage > 0,
      );
      if (candidate) {
        interrupted.push({
          id: "mock-interrupted",
          issueId: candidate.id,
          title: candidate.title,
          seriesTitle: candidate.seriesTitle ?? null,
          number: candidate.number ?? "",
          fileId: candidate.fileId ?? null,
          startedAt: candidate.lastReadAt ?? new Date().toISOString(),
          endedAt: candidate.lastReadAt ?? new Date().toISOString(),
          pages: 4,
          lastPage: candidate.lastPage,
          status: "open",
          resumedAt: null,
        });
      }
    }
    return { sessions, interrupted, recent };
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


