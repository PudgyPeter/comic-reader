import { useEffect, useRef, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api, MOCK_MODE } from "../lib/api";
import type { ImportCandidate, ImportProgress } from "../lib/types";
import { ImportReview } from "./ImportReview";
import { cx, eventMatches, isEditableTarget } from "../lib/utils";
import { useImportActivity } from "../store/importActivity";
import { useSettings } from "../store/settings";
import { useToast } from "./Toast";
import { Icon } from "./Icon";

const NAV = [
  { to: "/", label: "Library", icon: "grid", end: true },
  { to: "/series", label: "Series", icon: "layers" },
  { to: "/unmatched", label: "Unmatched", icon: "filter" },
  { to: "/collections", label: "Collections", icon: "bookmark" },
  { to: "/orders", label: "Reading orders", icon: "list" },
  { to: "/history", label: "History", icon: "clock" },
  { to: "/stats", label: "Statistics", icon: "chart" },
];

function CommandPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const navigate = useNavigate();
  const inputRef = useRef<HTMLInputElement>(null);

  const { data } = useQuery({
    queryKey: ["command-search", query],
    queryFn: () => api.search(query, 8),
    enabled: open && query.trim().length > 0,
  });

  useEffect(() => {
    if (!open) return;
    const timer = window.setTimeout(() => inputRef.current?.focus(), 30);
    return () => window.clearTimeout(timer);
  }, [open]);

  const issues = data?.issues ?? [];
  const series = data?.series ?? [];
  const results = [
    ...series.map((entry) => ({
      key: `s-${entry.id}`,
      label: entry.title,
      hint: "Series",
      to: `/series/${entry.id}`,
    })),
    ...issues.map((entry) => ({
      key: `i-${entry.id}`,
      label: entry.title,
      hint: entry.seriesTitle ?? "Issue",
      to: `/reader/${entry.id}`,
    })),
  ];

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[90] flex items-start justify-center bg-black/60 p-4 pt-[12vh] backdrop-blur-sm animate-fade">
      <div
        className="absolute inset-0"
        onClick={onClose}
        aria-hidden
      />
      <div className="relative w-full max-w-xl overflow-hidden rounded-2xl border border-[var(--line)] bg-[var(--panel)] shadow-2xl animate-pop">
        <div className="flex items-center gap-2 border-b border-[var(--line)] px-4">
          <Icon name="search" className="size-4 text-muted" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => {
              setQuery(event.currentTarget.value);
              setSelected(0);
            }}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                onClose();
              }
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setSelected((current) => Math.min(results.length - 1, current + 1));
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                setSelected((current) => Math.max(0, current - 1));
              }
              if (event.key === "Enter") {
                const result = results[selected];
                if (result) {
                  navigate(result.to);
                  onClose();
                } else if (query.trim()) {
                  navigate(`/search?q=${encodeURIComponent(query.trim())}`);
                  onClose();
                }
              }
            }}
            placeholder="Jump to a series or issue…"
            className="w-full bg-transparent py-4 text-sm outline-none placeholder:text-muted"
          />
          <kbd className="rounded border border-[var(--line)] px-1.5 py-0.5 text-[10px] text-muted">
            Esc
          </kbd>
        </div>
        <div className="max-h-80 overflow-y-auto py-2">
          {results.length === 0 ? (
            <p className="px-4 py-6 text-center text-xs text-muted">
              {query.trim() ? "No matches. Press Enter for full search." : "Start typing to search your library."}
            </p>
          ) : (
            results.map((result, index) => (
              <button
                key={result.key}
                onMouseEnter={() => setSelected(index)}
                onClick={() => {
                  navigate(result.to);
                  onClose();
                }}
                className={cx(
                  "flex w-full items-center justify-between gap-3 px-4 py-2 text-left text-sm",
                  index === selected ? "bg-[var(--surface-3)]" : "hover:bg-[var(--surface-2)]",
                )}
              >
                <span className="truncate">{result.label}</span>
                <span className="shrink-0 text-[11px] text-muted">{result.hint}</span>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

export function AppShell() {
  const location = useLocation();
  const navigate = useNavigate();
  const settings = useSettings((state) => state.settings);
  const toast = useToast();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [review, setReview] = useState<
    { path: string; candidates: ImportCandidate[] }[] | null
  >(null);
  const importing = useImportActivity((state) => state.active);
  const beginActivity = useImportActivity((state) => state.start);
  const endActivity = useImportActivity((state) => state.end);
  const contentRef = useRef<HTMLDivElement>(null);

  const navCollapsed = settings.density === "compact";
  const { data: snapshot } = useQuery({
    queryKey: ["library", "shell"],
    queryFn: () => api.library({ sort: "recent" }),
    staleTime: 15_000,
  });

  useEffect(() => {
    contentRef.current?.scrollTo({ top: 0 });
  }, [location.pathname]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (eventMatches(event, settings.shortcuts.search) || eventMatches(event, "Ctrl+K")) {
        if (isEditableTarget(event.target)) return;
        event.preventDefault();
        setPaletteOpen((current) => !current);
      }
      if (event.key === "Escape" && paletteOpen) {
        setPaletteOpen(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [settings.shortcuts.search, paletteOpen]);

  // Mock-harness affordance: ?mock&import opens the review screen without a
  // native folder picker so the flow can be screenshotted in a browser.
  useEffect(() => {
    if (!MOCK_MODE) return;
    if (!new URLSearchParams(window.location.search).has("import")) return;
    void api
      .previewImport("/mock/library")
      .then((candidates) => setReview([{ path: "/mock/library", candidates }]))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    if (MOCK_MODE) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const stop = await listen<ImportProgress>("import-progress", (event) => {
        setProgress(event.payload);
      });
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const cancelImport = async () => {
    await api.cancelImport().catch(() => undefined);
    toast.push("Cancelling import…", "info");
  };

  // Shared preview flow for folders and individual archive files. Multiple
  // sources are reviewed together so a folder and a huge single-file CBZ can be
  // added in one pass.
  const previewSources = async (selection: string[], label: string) => {
    beginActivity(label);
    setProgress(null);
    try {
      const sources: { path: string; candidates: ImportCandidate[] }[] = [];
      for (const path of selection) {
        const candidates = await api.previewImport(path).catch((error) => {
          toast.push(String(error), "error");
          return [] as ImportCandidate[];
        });
        if (candidates.length > 0) sources.push({ path, candidates });
      }
      if (sources.length === 0) {
        toast.push("Nothing new to import in that selection", "info");
        return;
      }
      setReview(sources);
    } catch (error) {
      if (
        (error instanceof Error && error.name === "cancelled") ||
        String(error).toLowerCase().includes("cancel")
      ) {
        toast.push("Import cancelled", "info");
      } else {
        toast.push(String(error), "error");
      }
    } finally {
      endActivity();
      setProgress(null);
    }
  };

  const addFolder = async () => {
    let selection: string | string[] | null;
    if (MOCK_MODE) {
      // The browser harness has no native folder picker; point at the fixture.
      selection = "/mock/library";
    } else {
      const { open } = await import("@tauri-apps/plugin-dialog");
      selection = await open({
        directory: true,
        multiple: false,
        title: "Add library folder",
      });
    }
    if (!selection || Array.isArray(selection)) return;
    await previewSources([selection], "Scanning folder");
  };

  const addFiles = async () => {
    let selection: string | string[] | null;
    if (MOCK_MODE) {
      selection = "/mock/library/Sonic 001.cbz";
    } else {
      const { open } = await import("@tauri-apps/plugin-dialog");
      selection = await open({
        directory: false,
        multiple: true,
        title: "Add comic files",
        filters: [
          { name: "Comics", extensions: ["cbz", "zip", "cbr", "pdf"] },
        ],
      });
    }
    if (!selection) return;
    const paths = Array.isArray(selection) ? selection : [selection];
    if (paths.length === 0) return;
    await previewSources(paths, "Scanning files");
  };

  return (
    <div className="flex h-full w-full overflow-hidden bg-[var(--bg)]">
      <aside
        className={cx(
          "flex shrink-0 flex-col border-r border-[var(--line)] bg-[var(--surface-1)] transition-[width] duration-200",
          navCollapsed ? "w-[68px]" : "w-[228px]",
        )}
      >
        <div className="flex items-center gap-2.5 px-4 py-5">
          <div className="grid size-9 shrink-0 place-items-center rounded-lg bg-[var(--accent)] font-display text-sm font-bold text-black">
            P
          </div>
          {!navCollapsed && (
            <div className="min-w-0">
              <p className="font-display text-sm font-semibold leading-none tracking-tight">
                Panel
              </p>
              <p className="mt-1 text-[10px] uppercase tracking-[0.18em] text-muted">
                Comic reader
              </p>
            </div>
          )}
        </div>

        <nav className="flex flex-1 flex-col gap-0.5 overflow-y-auto px-2">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                cx(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-[13px] font-medium transition-colors",
                  isActive
                    ? "bg-[var(--surface-3)] text-[var(--text-strong)]"
                    : "text-muted hover:bg-[var(--surface-2)] hover:text-[var(--text-strong)]",
                )
              }
              title={item.label}
            >
              <Icon name={item.icon} className="size-4 shrink-0" />
              {!navCollapsed && <span className="truncate">{item.label}</span>}
            </NavLink>
          ))}
        </nav>

        <div className="border-t border-[var(--line)] p-2">
          {importing && progress && (
            <div className={cx("mb-2 rounded-lg bg-[var(--surface-2)] px-2.5 py-2", navCollapsed && "px-1.5")}>
              {!navCollapsed && (
                <>
                  <div className="flex items-center justify-between gap-2">
                    <p className="text-[10px] font-medium text-muted">
                      {progress.stage === "done"
                        ? "Finishing…"
                        : progress.total > 0
                          ? `Scanning ${progress.processed}/${progress.total}`
                          : "Scanning…"}
                    </p>
                    <button
                      className="text-[10px] font-semibold text-[var(--accent)] hover:underline"
                      onClick={cancelImport}
                    >
                      Cancel
                    </button>
                  </div>
                  <div className="mt-1.5 h-1 overflow-hidden rounded-full bg-[var(--surface-3)]">
                    <div
                      className="h-full rounded-full bg-[var(--accent)] transition-[width] duration-200"
                      style={{
                        width: `${progress.total > 0 ? Math.round((progress.processed / progress.total) * 100) : 8}%`,
                      }}
                    />
                  </div>
                  <p className="mt-1 truncate text-[9px] text-muted" title={progress.current}>
                    {progress.current.split(/[/\\]/).pop() || "Collecting files"}
                  </p>
                </>
              )}
              {navCollapsed && (
                <button
                  className="grid w-full place-items-center py-1 text-[10px] font-semibold text-[var(--accent)]"
                  onClick={cancelImport}
                  title="Cancel import"
                >
                  <Icon name="x" className="size-3.5" />
                </button>
              )}
            </div>
          )}
          <button
            onClick={addFolder}
            disabled={importing}
            className={cx(
              "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-[var(--surface-2)] hover:text-[var(--text-strong)]",
            )}
            title="Add folder"
          >
            <Icon name="plus" className="size-4 shrink-0" />
            {!navCollapsed && <span>{importing ? "Importing…" : "Add folder"}</span>}
          </button>
          <button
            onClick={addFiles}
            disabled={importing}
            className={cx(
              "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-[var(--surface-2)] hover:text-[var(--text-strong)]",
            )}
            title="Add comic files (CBZ, CBR, PDF)"
          >
            <Icon name="book" className="size-4 shrink-0" />
            {!navCollapsed && <span>Add files</span>}
          </button>
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              cx(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-[13px] font-medium transition-colors",
                isActive
                  ? "bg-[var(--surface-3)] text-[var(--text-strong)]"
                  : "text-muted hover:bg-[var(--surface-2)] hover:text-[var(--text-strong)]",
              )
            }
            title="Settings"
          >
            <Icon name="settings" className="size-4 shrink-0" />
            {!navCollapsed && <span>Settings</span>}
          </NavLink>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center gap-3 border-b border-[var(--line)] px-5">
          <button
            onClick={() => setPaletteOpen(true)}
            className="group flex h-9 w-full max-w-md items-center gap-2 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 text-left text-xs text-muted transition-colors hover:border-[var(--line-strong)]"
          >
            <Icon name="search" className="size-3.5" />
            <span className="flex-1 truncate">Search comics, series, creators…</span>
            <kbd className="rounded border border-[var(--line)] px-1.5 py-0.5 text-[10px]">
              Ctrl K
            </kbd>
          </button>
          <div className="ml-auto flex items-center gap-3 text-xs text-muted">
            {snapshot && (
              <>
                <span className="hidden sm:inline">
                  {snapshot.totals.issues} issues
                </span>
                <span className="hidden sm:inline text-white/15">•</span>
                <span className="hidden sm:inline">
                  {snapshot.totals.completed} read
                </span>
              </>
            )}
            <button
              className="btn btn-sm"
              onClick={() => navigate("/settings")}
              title="Reader settings"
            >
              <Icon name="sliders" className="size-3.5" />
              <span className="hidden md:inline">Preferences</span>
            </button>
          </div>
        </header>

        <div ref={contentRef} className="min-h-0 flex-1 overflow-y-auto">
          <Outlet />
        </div>
      </div>

      {review && (
        <ImportReview
          sources={review}
          onClose={() => setReview(null)}
          onImported={() => {
            window.dispatchEvent(new Event("library:changed"));
            setReview(null);
          }}
        />
      )}
      {paletteOpen && (
        <CommandPalette open onClose={() => setPaletteOpen(false)} />
      )}
    </div>
  );
}
