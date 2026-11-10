export interface LibraryRoot {
  id: string;
  path: string;
  addedAt: string;
  fileCount: number;
}

export interface SeriesSummary {
  id: string;
  title: string;
  publisher: string | null;
  universe: string | null;
  description: string | null;
  issueCount: number;
  readCount: number;
  coverFileId: string | null;
  coverPage: number | null;
  year: string | null;
  readingDirection: "ltr" | "rtl" | null;
  missingCount: number;
  metadataLocked: boolean;
}

export interface MissingIssue {
  number: string;
  reason: "missing" | "not-required";
}

export interface SeriesDetail extends SeriesSummary {
  issues: IssueCard[];
  missing: MissingIssue[];
}

export interface IssueCard {
  id: string;
  seriesId: string | null;
  seriesTitle: string | null;
  title: string;
  number: string;
  sortNumber: number;
  volume: string | null;
  publisher: string | null;
  universe: string | null;
  releaseDate: string | null;
  writer: string | null;
  artist: string | null;
  coverArtist: string | null;
  description: string | null;
  fileId: string | null;
  filePath: string | null;
  pageCount: number;
  lastPage: number;
  status: "unread" | "in-progress" | "completed";
  lastReadAt: string | null;
  completedAt: string | null;
  spoilerProtected: boolean;
  missing: boolean;
  variantOf: string | null;
  edition: string | null;
  variantCount: number;
  metadataLocked: boolean;
  /** First page of this issue inside a shared archive (multi-issue files). */
  pageOffset: number;
}

export interface LibraryTotals {
  issues: number;
  series: number;
  completed: number;
  inProgress: number;
  unread: number;
  pagesRead: number;
}

export interface LibrarySnapshot {
  issues: IssueCard[];
  series: SeriesSummary[];
  roots: LibraryRoot[];
  totals: LibraryTotals;
}

export interface ImportWarning {
  path: string;
  message: string;
  kind: string;
}

export interface ImportProgress {
  stage: string;
  processed: number;
  total: number;
  current: string;
}

export interface ImportResult {
  root: LibraryRoot;
  filesImported: number;
  filesSkipped: number;
  seriesCreated: number;
  issuesCreated: number;
  warnings: ImportWarning[];
}

export interface ImportCandidate {
  path: string;
  relativePath: string;
  format: string;
  pageCount: number;
  seriesTitle: string;
  title: string;
  number: string;
  volume: string | null;
  publisher: string | null;
  releaseDate: string | null;
  writer: string | null;
  artist: string | null;
  coverArtist: string | null;
  description: string | null;
  duplicate: boolean;
  warning: string | null;
}

export interface ImportOverride {
  relativePath: string;
  seriesTitle?: string;
  title?: string;
  number?: string;
  volume?: string;
  publisher?: string;
  releaseDate?: string;
  writer?: string;
  artist?: string;
  coverArtist?: string;
  description?: string;
  keepUnmatched?: boolean;
}

export interface UnmatchedFile {
  id: string;
  rootId: string | null;
  path: string;
  relativePath: string;
  format: string;
  sizeBytes: number;
  pageCount: number;
  status: string;
  addedAt: string;
  page: number;
  progressStatus: "unread" | "in-progress" | "completed";
  lastReadAt: string | null;
  suggestedSeries: string;
  suggestedNumber: string;
  suggestedTitle: string;
}

export interface FileProposal {
  fileId: string;
  filePath: string;
  format: string;
  pageCount: number;
  seriesTitle: string;
  title: string;
  number: string;
  volume: string | null;
  publisher: string | null;
  releaseDate: string | null;
  writer: string | null;
  artist: string | null;
  coverArtist: string | null;
  description: string | null;
}

export interface MatchFilePatch {
  seriesTitle?: string;
  seriesId?: string;
  title?: string;
  number?: string;
  volume?: string;
  publisher?: string;
  releaseDate?: string;
  writer?: string;
  artist?: string;
  coverArtist?: string;
  description?: string;
}

export interface TocEntry {
  title: string;
  printedPage: number;
}

export interface SplitEntry {
  title: string;
  number: string;
  printedPage: number;
  startPage: number;
  endPage: number;
}

export interface SplitProposal {
  fileId: string;
  tocPage: number;
  pageCount: number;
  offset: number;
  entries: SplitEntry[];
}

export interface SplitRange {
  title: string;
  number: string;
  startPage: number;
  endPage: number;
}

export interface FileProgressState {
  page: number;
  total: number;
  status: "unread" | "in-progress" | "completed";
  startedAt: string | null;
  completedAt: string | null;
  updatedAt: string | null;
  scrollOffset: number;
}

export interface PageInfo {
  index: number;
  name: string;
  width: number;
  height: number;
}

export interface ProgressState {
  page: number;
  total: number;
  status: string;
  startedAt: string | null;
  completedAt: string | null;
  updatedAt: string | null;
  scrollOffset: number;
}

export interface ReaderPayload {
  fileId: string;
  issueId: string | null;
  title: string;
  seriesTitle: string | null;
  number: string;
  pages: PageInfo[];
  progress: ProgressState;
  background: string;
  mode: ReaderMode;
  readingDirection: "ltr" | "rtl";
  fit: FitMode;
  transition: TransitionMode;
  transitionDuration: number;
  showInterface: boolean;
  nextIssueId: string | null;
  previousIssueId: string | null;
  spreadOffset: number;
}

export type ReaderMode = "single" | "double" | "vertical" | "horizontal";
export type FitMode = "width" | "height" | "page" | "original";
export type TransitionMode = "instant" | "slide" | "fade" | "page-turn";

export interface ReadingOrder {
  id: string;
  name: string;
  description: string | null;
  kind: string;
  itemCount: number;
  createdAt: string;
}

export interface OrderItem {
  issueId: string;
  position: number;
  issue: IssueCard | null;
}

export interface ReadingOrderDetail extends ReadingOrder {
  items: OrderItem[];
}

export interface NextIssue {
  id: string;
  title: string;
  number: string;
  seriesTitle: string | null;
  fileId: string | null;
}

export type SmartRuleField =
  | "title"
  | "number"
  | "series"
  | "publisher"
  | "universe"
  | "writer"
  | "artist"
  | "coverArtist"
  | "edition"
  | "releaseDate"
  | "year"
  | "status"
  | "format"
  | "pages"
  | "lastPage";

export type SmartRuleOp =
  | "contains"
  | "eq"
  | "ne"
  | "startsWith"
  | "gt"
  | "gte"
  | "lt"
  | "lte"
  | "isSet"
  | "isEmpty";

export interface SmartRule {
  field: SmartRuleField;
  op: SmartRuleOp;
  value: string;
}

export interface SmartRules {
  all: SmartRule[];
  any: SmartRule[];
}

export type CollectionSortMode =
  | "manual"
  | "title"
  | "series"
  | "added_desc"
  | "added_asc"
  | "release_date";

export interface CollectionPatch {
  name?: string;
  description?: string;
  coverFileId?: string;
  isSmart?: boolean;
  smartRules?: SmartRules;
  sortMode?: CollectionSortMode;
}

export interface Collection {
  id: string;
  name: string;
  description: string | null;
  coverFileId: string | null;
  itemCount: number;
  createdAt: string;
  isSmart: boolean;
  smartRules: SmartRules | null;
  sortMode: CollectionSortMode;
}

export interface CollectionDetail extends Collection {
  items: IssueCard[];
}

export interface SearchResult {
  issues: IssueCard[];
  series: SeriesSummary[];
  total: number;
  snippets: Record<string, string>;
  nextCursor: string | null;
}

export interface SearchFilter {
  query: string;
  publisher?: string | null;
  seriesId?: string | null;
  universe?: string | null;
  status?: string | null;
  format?: string | null;
  yearFrom?: number | null;
  yearTo?: number | null;
  collectionId?: string | null;
  sort?: string;
  cursor?: string | null;
  limit?: number;
}

export interface RecentSearch {
  query: string;
  searchedAt: string;
}

export interface SearchSuggestion {
  value: string;
  kind: string;
}

export interface SavedSearch {
  id: string;
  name: string;
  query: string;
  createdAt: string;
}

export interface Settings {
  background: string;
  pageSpacing: number;
  brightness: number;
  gamma: number;
  readingDirection: "ltr" | "rtl";
  defaultMode: ReaderMode;
  defaultFit: FitMode;
  defaultZoomBehavior: "cursor" | "center" | "page";
  transition: TransitionMode;
  transitionDuration: number;
  animationsEnabled: boolean;
  showInterface: boolean;
  fullscreen: boolean;
  completionThreshold: number;
  spoilerProtection: boolean;
  spoilerOrderId: string | null;
  theme: "dark" | "light" | "midnight";
  accent: string;
  density: "compact" | "comfortable" | "spacious";
  libraryLayout: "grid" | "list";
  librarySort: string;
  shortcuts: Record<string, string>;
}

export interface HistorySession {
  id: string;
  issueId: string;
  title: string;
  seriesTitle: string | null;
  number: string;
  fileId: string | null;
  startedAt: string;
  endedAt: string;
  pages: number;
  lastPage: number;
  status: string;
  resumedAt: string | null;
}

export interface HistoryPayload {
  sessions: HistorySession[];
  interrupted: HistorySession[];
  recent: IssueCard[];
}

export interface Stats {
  totalIssues: number;
  completed: number;
  inProgress: number;
  totalPagesRead: number;
  readingSeconds: number;
  completedThisMonth: number;
  streakDays: number;
  topSeries: { name: string; read: number; total: number }[];
  byStatus: { label: string; count: number }[];
}

export interface LibraryFilter {
  search?: string;
  universe?: string | null;
  publisher?: string | null;
  seriesId?: string | null;
  status?: string | null;
  sort?: string;
  descending?: boolean;
}

export interface Bookmark {
  id: string;
  page: number;
  label: string | null;
  note: string | null;
  createdAt: string;
}

export interface IssuePatch {
  title?: string;
  number?: string;
  volume?: string;
  publisher?: string;
  releaseDate?: string;
  writer?: string;
  artist?: string;
  coverArtist?: string;
  description?: string;
  seriesId?: string;
  seriesTitle?: string;
  variantOf?: string | null;
  edition?: string | null;
  metadataLocked?: boolean;
}

export interface SeriesPatch {
  title?: string;
  publisher?: string;
  universe?: string;
  description?: string;
  volume?: string;
  coverFileId?: string;
  readingDirection?: "ltr" | "rtl" | "";
  metadataLocked?: boolean;
}
