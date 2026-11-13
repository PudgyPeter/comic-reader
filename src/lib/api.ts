import { invoke } from "@tauri-apps/api/core";
import type {
  Bookmark,
  Collection,
  CollectionDetail,
  CollectionPatch,
  CollectionSortMode,
  ImportCandidate,
  ImportOverride,
  ImportResult,
  HistoryPayload,
  IssueCard,
  FileProgressState,
  FileProposal,
  IssuePatch,
  LibraryFilter,
  MatchFilePatch,
  SplitProposal,
  SplitRange,
  CharacterCard,
  CharacterDetail,
  CharacterPatch,
  CharacterSummary,
  CharacterSync,
  CharacterRelationship,
  RelationshipInput,
  RelationshipPatch,
  RelationshipType,
  UniverseDetail,
  UniverseSummary,
  ProviderLookup,
  ProviderProposal,
  ProviderProposalRow,
  ProviderStatus,
  TocEntry,
  LibraryRoot,
  LibrarySnapshot,
  NextIssue,
  OrderItem,
  ReaderPayload,
  ReadingOrder,
  ReadingOrderDetail,
  RecentSearch,
  SavedSearch,
  SearchFilter,
  SearchResult,
  SearchSuggestion,
  SeriesDetail,
  SeriesPatch,
  Settings,
  SmartRules,
  Stats,
  UnmatchedFile,
} from "./types";

interface CommandError {
  kind?: string;
  message?: string;
}

function normalizeError(error: unknown): Error {
  if (error instanceof Error) return error;
  if (error && typeof error === "object") {
    const payload = error as CommandError;
    if (payload.message) {
      const wrapped = new Error(payload.message);
      wrapped.name = payload.kind ?? "error";
      return wrapped;
    }
  }
  return new Error(String(error));
}

function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(name, args).catch((error) => {
    throw normalizeError(error);
  });
}

export const api = {
  addLibraryRoot: (path: string) => command<ImportResult>("add_library_root", { path }),
  previewImport: (path: string) => command<ImportCandidate[]>("preview_import", { path }),
  commitImport: (path: string, overrides: ImportOverride[]) =>
    command<ImportResult>("commit_import", { path, overrides }),
  rescanLibrary: () => command<ImportResult[]>("rescan_library"),
  cancelImport: () => command<boolean>("cancel_import"),
  removeLibraryRoot: (rootId: string) => command<void>("remove_library_root", { rootId }),
  libraryRoots: () => command<LibraryRoot[]>("library_roots"),
  library: (filter: LibraryFilter) => command<LibrarySnapshot>("library", { filter }),
  filterOptions: () =>
    command<{
      universes: string[];
      publishers: string[];
      formats: string[];
      series: { id: string; title: string }[];
      yearMin: number | null;
      yearMax: number | null;
    }>("filter_options"),
  seriesDetail: (seriesId: string) => command<SeriesDetail>("series_detail", { seriesId }),
  createSeries: (title: string, publisher?: string, universe?: string) =>
    command<SeriesDetail>("create_series", { title, publisher, universe }),
  updateSeries: (seriesId: string, patch: SeriesPatch) =>
    command<SeriesDetail>("update_series", { seriesId, patch }),
  deleteSeries: (seriesId: string) => command<void>("delete_series", { seriesId }),
  issueDetail: (issueId: string) => command<IssueCard>("issue_detail", { issueId }),
  updateIssue: (issueId: string, patch: IssuePatch) =>
    command<IssueCard>("update_issue", { issueId, patch }),
  bulkUpdateIssues: (issueIds: string[], patch: IssuePatch) =>
    command<IssueCard[]>("bulk_update_issues", { issueIds, patch }),
  refreshIssueMetadata: (issueId: string, force?: boolean) =>
    command<boolean>("refresh_issue_metadata", { issueId, force }),
  deleteIssue: (issueId: string) => command<void>("delete_issue", { issueId }),
  readerPayload: (issueId: string) => command<ReaderPayload>("reader_payload", { issueId }),
  issueVariants: (issueId: string) => command<IssueCard[]>("issue_variants", { issueId }),
  setSpreadOffset: (issueId: string, offset: number) =>
    command<number>("set_spread_offset", { issueId, offset }),
  setDirectionOverride: (issueId: string, direction: "ltr" | "rtl" | null) =>
    command<"ltr" | "rtl" | null>("set_direction_override", { issueId, direction }),
  setMissingReason: (seriesId: string, number: string, reason: "missing" | "not-required" | null) =>
    command<void>("set_missing_reason", { seriesId, number, reason }),
  saveProgress: (
    issueId: string,
    page: number,
    scrollOffset?: number | null,
    completionThreshold?: number,
  ) =>
    command<{ page: number; total: number; status: string; scrollOffset: number }>(
      "save_progress",
      { issueId, page, scrollOffset, completionThreshold },
    ),
  markIssueToPage: (issueId: string, page: number) =>
    command<{ page: number; total: number; status: string; scrollOffset: number }>(
      "mark_issue_to_page",
      { issueId, page },
    ),
  startReadingSession: (issueId: string, page: number) =>
    command<string>("start_reading_session", { issueId, page }),
  touchReadingSession: (sessionId: string, pages: number, lastPage: number) =>
    command<void>("touch_reading_session", { sessionId, pages, lastPage }),
  closeReadingSession: (sessionId: string, pages: number, lastPage: number) =>
    command<void>("close_reading_session", { sessionId, pages, lastPage }),
  resumeReadingSession: (sessionId: string) =>
    command<void>("resume_reading_session", { sessionId }),
  setIssueStatus: (issueId: string, status: string) =>
    command<IssueCard>("set_issue_status", { issueId, status }),
  setSeriesDirection: (seriesId: string, direction: "ltr" | "rtl" | null) =>
    command<number>("set_series_direction", { seriesId, direction }),
  setSeriesStatus: (seriesId: string, status: string) =>
    command<number>("set_series_status", { seriesId, status }),
  recordReadingSession: (issueId: string, pages: number, seconds: number) =>
    command<void>("record_reading_session", { issueId, pages, seconds }),
  search: (query: string, limit?: number) =>
    command<SearchResult>("search_library", { query, limit }),
  searchFiltered: (filter: SearchFilter) => command<SearchResult>("search_filtered", { filter }),
  recentSearches: () => command<RecentSearch[]>("recent_searches"),
  clearSearchHistory: () => command<void>("clear_search_history"),
  searchSuggestions: (query: string) =>
    command<SearchSuggestion[]>("search_suggestions", { query }),
  savedSearches: () => command<SavedSearch[]>("saved_searches"),
  saveSearch: (name: string, query: string) =>
    command<SavedSearch>("save_search", { name, query }),
  deleteSavedSearch: (id: string) => command<void>("delete_saved_search", { id }),
  readingOrders: () => command<ReadingOrder[]>("reading_orders"),
  readingOrder: (orderId: string) => command<ReadingOrderDetail>("reading_order", { orderId }),
  createReadingOrder: (name: string, description?: string, kind?: string) =>
    command<ReadingOrder>("create_reading_order", { name, description, kind }),
  generatePublicationOrder: (name: string) =>
    command<ReadingOrderDetail>("generate_publication_order", { name }),
  deleteReadingOrder: (orderId: string) => command<void>("delete_reading_order", { orderId }),
  addOrderItems: (orderId: string, issueIds: string[]) =>
    command<void>("add_order_items", { orderId, issueIds }),
  reorderItems: (orderId: string, issueIds: string[]) =>
    command<void>("reorder_items", { orderId, issueIds }),
  nextInOrder: (orderId: string, issueId: string) =>
    command<NextIssue | null>("next_in_order", { orderId, issueId }),
  collections: () => command<Collection[]>("collections"),
  collection: (collectionId: string) =>
    command<CollectionDetail>("collection", { collectionId }),
  createCollection: (name: string, description?: string) =>
    command<Collection>("create_collection", { name, description }),
  createSmartCollection: (
    name: string,
    description: string | undefined,
    rules: SmartRules,
    sortMode?: CollectionSortMode,
  ) => command<Collection>("create_smart_collection", { name, description, rules, sortMode }),
  updateCollection: (collectionId: string, patch: CollectionPatch) =>
    command<Collection>("update_collection", { collectionId, patch }),
  deleteCollection: (collectionId: string) =>
    command<void>("delete_collection", { collectionId }),
  addToCollection: (collectionId: string, issueIds: string[]) =>
    command<void>("add_to_collection", { collectionId, issueIds }),
  addSeriesToCollection: (collectionId: string, seriesId: string) =>
    command<void>("add_series_to_collection", { collectionId, seriesId }),
  removeFromCollection: (collectionId: string, issueIds: string[]) =>
    command<void>("remove_from_collection", { collectionId, issueIds }),
  reorderCollection: (collectionId: string, issueIds: string[]) =>
    command<void>("reorder_collection", { collectionId, issueIds }),
  getSettings: () => command<Settings>("get_settings"),
  saveSettings: (settings: Settings) => command<Settings>("save_settings", { settings }),
  unmatchedFiles: () => command<UnmatchedFile[]>("unmatched_files"),
  fileProposal: (fileId: string) => command<FileProposal>("file_proposal", { fileId }),
  filelessIssues: () => command<IssueCard[]>("fileless_issues"),
  applyFileMatch: (fileId: string, patch: MatchFilePatch, issueId?: string | null) =>
    command<IssueCard>("apply_file_match", { fileId, patch, issueId: issueId ?? null }),
  fileReaderPayload: (fileId: string) =>
    command<ReaderPayload>("file_reader_payload", { fileId }),
  saveFileProgress: (fileId: string, page: number, scrollOffset?: number | null) =>
    command<FileProgressState>("save_file_progress", { fileId, page, scrollOffset }),
  fileBookmarks: (fileId: string) => command<Bookmark[]>("file_bookmarks", { fileId }),
  addFileBookmark: (fileId: string, page: number, label?: string, note?: string) =>
    command<Bookmark>("add_file_bookmark", { fileId, page, label, note }),
  deleteFileBookmark: (id: string) => command<void>("delete_file_bookmark", { id }),
  updateFileBookmarkNote: (id: string, note: string | null) =>
    command<Bookmark>("update_file_bookmark_note", { id, note }),
  setFileSpreadOffset: (fileId: string, offset: number) =>
    command<number>("set_file_spread_offset", { fileId, offset }),
  setFileDirectionOverride: (fileId: string, direction: "ltr" | "rtl" | null) =>
    command<"ltr" | "rtl" | null>("set_file_direction_override", { fileId, direction }),
  seriesCharacters: (seriesId: string) =>
    command<CharacterCard[]>("series_characters", { seriesId }),
  allCharacters: () => command<CharacterCard[]>("all_characters"),
  syncSeriesCharacters: (seriesId: string) =>
    command<CharacterSync>("sync_series_characters", { seriesId }),
  characterDetail: (characterId: string) =>
    command<CharacterDetail>("character_detail", { characterId }),
  updateCharacter: (characterId: string, patch: CharacterPatch) =>
    command<CharacterCard>("update_character", { characterId, patch }),
  deleteCharacter: (characterId: string) => command<void>("delete_character", { characterId }),
  lookupCharacter: (name: string) => command<CharacterSummary>("lookup_character", { name }),
  characterRelationships: (characterId: string) =>
    command<CharacterRelationship[]>("character_relationships", { characterId }),
  relationshipTypes: () => command<RelationshipType[]>("relationship_types"),
  createRelationship: (characterId: string, input: RelationshipInput) =>
    command<CharacterRelationship>("create_relationship", { characterId, input }),
  updateRelationship: (relationshipId: string, patch: RelationshipPatch) =>
    command<void>("update_relationship", { relationshipId, patch }),
  deleteRelationship: (relationshipId: string) =>
    command<void>("delete_relationship", { relationshipId }),
  providerStatus: () => command<ProviderStatus>("provider_status"),
  setProviderKey: (key: string) => command<void>("set_provider_key", { key }),
  lookupSeriesProvider: (seriesId: string) =>
    command<ProviderLookup>("lookup_series_provider", { seriesId }),
  proposeSeriesCharacters: (seriesId: string) =>
    command<ProviderProposal>("propose_series_characters", { seriesId }),
  providerProposals: () => command<ProviderProposalRow[]>("provider_proposals"),
  decideProviderProposal: (proposalId: string, accept: boolean) =>
    command<number>("decide_provider_proposal", { proposalId, accept }),
  universes: () => command<UniverseSummary[]>("universes"),
  universeDetail: (name: string) => command<UniverseDetail>("universe_detail", { name }),
  splitAvailable: () => command<boolean>("split_available"),
  proposeSplit: (fileId: string) => command<SplitProposal>("propose_split", { fileId }),
  recombineSplit: (fileId: string, entries: TocEntry[], offset: number) =>
    command<SplitProposal>("recombine_split", { fileId, entries, offset }),
  applySplit: (fileId: string, seriesTitle: string | undefined, ranges: SplitRange[]) =>
    command<IssueCard[]>("apply_split", {
      request: { fileId, seriesTitle, ranges },
    }),
  bookmarks: (issueId: string) => command<Bookmark[]>("bookmarks", { issueId }),
  addBookmark: (issueId: string, page: number, label?: string, note?: string) =>
    command<Bookmark>("add_bookmark", { issueId, page, label, note }),
  deleteBookmark: (id: string) => command<void>("delete_bookmark", { id }),
  updateBookmarkNote: (id: string, note: string | null) =>
    command<Bookmark>("update_bookmark_note", { id, note }),
  revealSpoiler: (issueId: string) => command<void>("reveal_spoiler", { issueId }),
  stats: () => command<Stats>("stats"),
  readingHistory: (limit?: number) => command<HistoryPayload>("reading_history", { limit }),
  exportLibrary: () => command<Record<string, unknown>>("export_library"),
  exportLibraryTo: (path: string) => command<void>("export_library_to", { path }),
};

export type OrderItemPayload = OrderItem;

const isWindowsHost =
  typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent);

export const assetOrigin = isWindowsHost ? "http://comic.localhost" : "comic://localhost";

export const MOCK_MODE =
  import.meta.env.DEV &&
  typeof window !== "undefined" &&
  new URLSearchParams(window.location.search).has("mock");

let mockResolver: ((url: string) => string) | null = null;

export function setMockResolver(resolver: ((url: string) => string) | null) {
  mockResolver = resolver;
}

function resolve(url: string): string {
  if (MOCK_MODE && mockResolver) return mockResolver(url);
  return url;
}

export function coverUrl(fileId: string | null, page = 0, width = 320) {
  if (!fileId) return null;
  return resolve(`${assetOrigin}/cover/${fileId}/${page}/${width}`);
}

export function pageUrl(fileId: string, page: number, width?: number) {
  const suffix = width ? `/${Math.round(width)}` : "";
  return resolve(`${assetOrigin}/page/${fileId}/${page}${suffix}`);
}
