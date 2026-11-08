import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api, pageUrl } from "../lib/api";
import type { FitMode, ReaderMode, TransitionMode } from "../lib/types";
import {
  cx,
  eventMatches,
  fitLabel,
  isEditableTarget,
  modeLabel,
} from "../lib/utils";
import {
  buildSpreads,
  firstPageOfSpread,
  spreadForPage,
} from "../lib/spreads";
import { useReaderZoom } from "../hooks/useReaderZoom";
import { useSettings } from "../store/settings";
import { useToast } from "../components/Toast";
import { Icon } from "../components/Icon";
import { Spinner } from "../components/ui";

const MODES: ReaderMode[] = ["single", "double", "vertical", "horizontal"];
const FITS: FitMode[] = ["page", "width", "height", "original"];

export default function ReaderPage() {
  const { issueId = "" } = useParams();
  const [searchParams] = useSearchParams();
  const activeOrder = searchParams.get("order");
  const navigate = useNavigate();
  const toast = useToast();
  const updateSettings = useSettings((state) => state.update);
  const settings = useSettings((state) => state.settings);

  const query = useQuery({
    queryKey: ["reader", issueId],
    queryFn: () => api.readerPayload(issueId),
    enabled: Boolean(issueId),
    staleTime: Infinity,
  });

  const [page, setPage] = useState(0);
  const [mode, setMode] = useState<ReaderMode>("single");
  const [fit, setFit] = useState<FitMode>("page");
  const [transition, setTransition] = useState<TransitionMode>("slide");
  const [readingDirection, setReadingDirection] = useState<"ltr" | "rtl">("ltr");
  const [spreadOffset, setSpreadOffset] = useState(0);
  const [chrome, setChrome] = useState(true);
  const [transitioning, setTransitioning] = useState(false);
  const [bookmarksOpen, setBookmarksOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [bookmarks, setBookmarks] = useState<
    { id: string; page: number; label: string | null; note: string | null }[]
  >([]);
  const [bookmarkLabel, setBookmarkLabel] = useState("");
  const [bookmarkNote, setBookmarkNote] = useState("");
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [noteDraft, setNoteDraft] = useState("");

  const viewportRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const sessionStart = useRef(0);
  const pagesVisited = useRef(0);
  const persistTimer = useRef<number | null>(null);

  const zoom = useReaderZoom(viewportRef);

  // Initialise reader state once per loaded payload using React's documented
  // "adjust state when props change" pattern rather than an effect.
  const [appliedPayload, setAppliedPayload] = useState<string | null>(null);
  if (query.data && appliedPayload !== query.data.fileId + String(query.data.progress.page)) {
    setAppliedPayload(query.data.fileId + String(query.data.progress.page));
    setPage(query.data.progress.page ?? 0);
    setMode(query.data.mode);
    setFit(query.data.fit);
    setTransition(query.data.transition);
    setReadingDirection(query.data.readingDirection);
    setSpreadOffset(query.data.spreadOffset ?? 0);
    setChrome(query.data.showInterface);
  }

  useEffect(() => {
    pagesVisited.current = 0;
    sessionStart.current = Date.now();
  }, [issueId]);

  const pages = query.data?.pages ?? [];
  const spreads =
    mode === "double"
      ? buildSpreads(pages, "double", spreadOffset)
      : buildSpreads(pages, "single");
  const total = pages.length;
  const spreadCount = spreads.length as number;
  const spreadIndex = spreadForPage(spreads, page);

  const persist = useCallback(
    (nextPage: number) => {
      if (!issueId) return;
      if (persistTimer.current) window.clearTimeout(persistTimer.current);
      persistTimer.current = window.setTimeout(() => {
        void api.saveProgress(issueId, nextPage, settings.completionThreshold);
      }, 400);
    },
    [issueId, settings.completionThreshold],
  );

  const goToPage = useCallback(
    (next: number, direction: 1 | -1 | 0 = 0) => {
      const clamped = Math.max(0, Math.min(total - 1, next));
      if (clamped === page) return;
      if (direction !== 0 && settings.animationsEnabled) {
        setTransitioning(true);
        window.setTimeout(() => setTransitioning(false), settings.transitionDuration);
      }
      pagesVisited.current += Math.abs(clamped - page);
      setPage(clamped);
      persist(clamped);
    },
    [page, persist, settings.animationsEnabled, settings.transitionDuration, total],
  );

  const goToSpread = useCallback(
    (index: number) => {
      const clamped = Math.max(0, Math.min(spreadCount - 1, index));
      const target = firstPageOfSpread(spreads, clamped);
      if (target !== page) {
        if (settings.animationsEnabled) {
          setTransitioning(true);
          window.setTimeout(() => setTransitioning(false), settings.transitionDuration);
        }
        setPage(target);
        persist(target);
        pagesVisited.current += 1;
      }
    },
    [page, persist, settings.animationsEnabled, settings.transitionDuration, spreadCount, spreads],
  );

  const rtl = readingDirection === "rtl";

  const next = useCallback(() => {
    if (mode === "double") {
      goToSpread(spreadIndex + 1);
    } else if (mode === "vertical") {
      scrollRef.current?.scrollBy({ top: window.innerHeight * 0.85, behavior: "smooth" });
    } else if (mode === "horizontal") {
      scrollRef.current?.scrollBy({
        left: (rtl ? -1 : 1) * window.innerWidth * 0.9,
        behavior: "smooth",
      });
    } else {
      goToPage(page + 1, 1);
    }
  }, [goToPage, goToSpread, mode, page, rtl, spreadIndex]);

  const prev = useCallback(() => {
    if (mode === "double") {
      goToSpread(spreadIndex - 1);
    } else if (mode === "vertical") {
      scrollRef.current?.scrollBy({ top: -window.innerHeight * 0.85, behavior: "smooth" });
    } else if (mode === "horizontal") {
      scrollRef.current?.scrollBy({
        left: (rtl ? 1 : -1) * window.innerWidth * 0.9,
        behavior: "smooth",
      });
    } else {
      goToPage(page - 1, -1);
    }
  }, [goToPage, goToSpread, mode, page, rtl, spreadIndex]);

  const openIssue = useCallback(
    (target: string | null | undefined) => {
      if (!target) return;
      void api.recordReadingSession(
        issueId,
        pagesVisited.current,
        Math.round((Date.now() - sessionStart.current) / 1000),
      );
      const suffix = activeOrder ? `?order=${activeOrder}` : "";
      navigate(`/reader/${target}${suffix}`);
    },
    [activeOrder, issueId, navigate],
  );

  const nextInOrder = useQuery({
    queryKey: ["next-in-order", activeOrder, issueId],
    queryFn: () => api.nextInOrder(activeOrder ?? "", issueId),
    enabled: Boolean(activeOrder),
    staleTime: Infinity,
  });

  const goNextIssue = useCallback(() => {
    if (activeOrder && nextInOrder.data) {
      openIssue(nextInOrder.data.id);
      return;
    }
    if (activeOrder && nextInOrder.isSuccess && !nextInOrder.data) {
      toast.push("You reached the end of this reading order", "info");
      return;
    }
    openIssue(query.data?.nextIssueId);
  }, [activeOrder, nextInOrder.data, nextInOrder.isSuccess, openIssue, query.data, toast]);

  useEffect(() => {
    void api.bookmarks(issueId).then(setBookmarks).catch(() => undefined);
  }, [issueId]);

  const toggleFullscreen = useCallback(async () => {
    try {
      if (document.fullscreenElement) {
        await document.exitFullscreen();
      } else {
        await document.documentElement.requestFullscreen();
      }
    } catch {
      toast.push("Fullscreen is not available", "error");
    }
  }, [toast]);

  const addBookmark = useCallback(async () => {
    const created = await api.addBookmark(
      issueId,
      page,
      bookmarkLabel || `Page ${page + 1}`,
      bookmarkNote.trim() || undefined,
    );
    setBookmarks((current) => [
      ...current,
      { id: created.id, page: created.page, label: created.label, note: created.note },
    ]);
    setBookmarkLabel("");
    setBookmarkNote("");
    toast.push(`Bookmarked page ${page + 1}`, "success");
  }, [bookmarkLabel, bookmarkNote, issueId, page, toast]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) return;
      const shortcuts = settings.shortcuts;
      if (eventMatches(event, shortcuts.nextPage)) {
        event.preventDefault();
        next();
      } else if (eventMatches(event, shortcuts.prevPage)) {
        event.preventDefault();
        prev();
      } else if (eventMatches(event, shortcuts.nextIssue)) {
        event.preventDefault();
        goNextIssue();
      } else if (eventMatches(event, shortcuts.prevIssue)) {
        event.preventDefault();
        openIssue(query.data?.previousIssueId);
      } else if (eventMatches(event, shortcuts.toggleFullscreen)) {
        event.preventDefault();
        void toggleFullscreen();
      } else if (eventMatches(event, shortcuts.toggleZoom)) {
        event.preventDefault();
        zoom.toggleAt();
      } else if (eventMatches(event, shortcuts.escape)) {
        if (zoom.zoom.scale > 1) {
          zoom.reset();
        } else if (document.fullscreenElement) {
          void document.exitFullscreen();
        } else if (mode === "vertical") {
          navigate(-1);
        } else {
          navigate(-1);
        }
      } else if (eventMatches(event, shortcuts.bookmark)) {
        event.preventDefault();
        void addBookmark();
      } else if (event.key === "d" || event.key === "D") {
        setMode((current) => MODES[(MODES.indexOf(current) + 1) % MODES.length]);
      } else if (event.key === "h" || event.key === "H") {
        setChrome((current) => !current);
      } else if (event.key === "1" || event.key === "2" || event.key === "3" || event.key === "4") {
        setFit(FITS[Number(event.key) - 1]);
      } else if (event.key === "?" || (event.key === "/" && event.shiftKey)) {
        setShortcutsOpen((current) => !current);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    addBookmark,
    goNextIssue,
    mode,
    navigate,
    next,
    openIssue,
    prev,
    query.data,
    settings.shortcuts,
    toggleFullscreen,
    zoom,
  ]);

  // Scroll continuous modes to the resumed page once per issue/mode change.
  const scrolledKey = useRef<string | null>(null);
  useEffect(() => {
    if (mode !== "vertical" && mode !== "horizontal") return;
    const container = scrollRef.current;
    if (!container) return;
    const key = `${issueId}:${mode}`;
    if (scrolledKey.current === key) return;
    const vertical = mode === "vertical";
    const align = () => {
      const target = container.querySelector<HTMLElement>(`[data-page="${page}"]`);
      if (!target) return;
      if (vertical) {
        container.scrollTop = target.offsetTop;
      } else {
        container.scrollLeft = target.offsetLeft;
      }
    };
    align();
    const raf = requestAnimationFrame(align);
    const images = Array.from(container.querySelectorAll("img"));
    const pending = images.filter((image) => !image.complete);
    const onLoad = () => align();
    pending.forEach((image) => image.addEventListener("load", onLoad, { once: true }));
    scrolledKey.current = key;
    return () => {
      cancelAnimationFrame(raf);
      pending.forEach((image) => image.removeEventListener("load", onLoad));
    };
  }, [issueId, mode, page]);

  useEffect(() => {
    if ((mode !== "vertical" && mode !== "horizontal") || !scrollRef.current) return;
    const container = scrollRef.current;
    const vertical = mode === "vertical";
    let raf = 0;
    const onScroll = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        const children = Array.from(container.querySelectorAll<HTMLElement>("[data-page]"));
        const center = vertical
          ? container.scrollTop + container.clientHeight / 2
          : container.scrollLeft + container.clientWidth / 2;
        let closest = 0;
        let best = Infinity;
        for (const child of children) {
          const start = vertical ? child.offsetTop : child.offsetLeft;
          const size = vertical ? child.offsetHeight : child.offsetWidth;
          const distance = Math.abs(start + size / 2 - center);
          if (distance < best) {
            best = distance;
            closest = Number(child.dataset.page);
          }
        }
        if (closest !== page) {
          setPage(closest);
          persist(closest);
        }
      });
    };
    container.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      container.removeEventListener("scroll", onScroll);
      cancelAnimationFrame(raf);
    };
  }, [mode, page, persist]);

  const autoAdvanced = useRef<string | null>(null);
  useEffect(() => {
    if (!query.data || total === 0) return;
    const lastPage = page >= total - 1;
    if (!lastPage) return;
    if (autoAdvanced.current === issueId) return;
    if (!activeOrder) return;
    if (query.data.progress.status !== "completed") return;
    if (!nextInOrder.data) return;
    autoAdvanced.current = issueId;
    const timer = window.setTimeout(() => goNextIssue(), 900);
    return () => window.clearTimeout(timer);
  }, [activeOrder, goNextIssue, issueId, nextInOrder.data, page, query.data, total]);

  useEffect(() => {
    return () => {
      if (persistTimer.current) window.clearTimeout(persistTimer.current);
      void api.saveProgress(
        issueId,
        page,
        settings.completionThreshold,
      );
      void api.recordReadingSession(
        issueId,
        pagesVisited.current,
        Math.round((Date.now() - sessionStart.current) / 1000),
      );
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [issueId]);

  if (query.isLoading) {
    return (
      <div className="grid h-full place-items-center bg-[#08090c]">
        <Spinner className="size-7" />
      </div>
    );
  }

  if (query.isError || !query.data) {
    return (
      <div className="grid h-full place-items-center bg-[#08090c] p-6">
        <div className="max-w-sm text-center">
          <h1 className="font-display text-lg font-semibold">This issue could not be opened</h1>
          <p className="mt-2 text-sm text-muted">{String(query.error ?? "The archive may be missing or corrupt.")}</p>
          <button className="btn mt-4" onClick={() => navigate(-1)}>
            Go back
          </button>
        </div>
      </div>
    );
  }

  const data = query.data;
  const ratio = total ? (page + 1) / total : 0;
  const background = data.background || "#0b0c10";

  return (
    <div className="relative flex h-full flex-col overflow-hidden" style={{ background }} data-reader>
      <div
        className={cx(
          "absolute inset-x-0 top-0 z-30 flex items-center gap-3 border-b border-[var(--line)] bg-[color-mix(in_srgb,var(--panel)_88%,transparent)] px-4 py-2.5 backdrop-blur-md transition-transform duration-300",
          chrome ? "translate-y-0" : "-translate-y-full",
        )}
      >
        <button className="btn btn-ghost btn-sm" onClick={() => navigate(-1)} aria-label="Back to library">
          <Icon name="arrowLeft" className="size-4" />
        </button>
        <div className="min-w-0 flex-1">
          <p className="truncate text-[13px] font-medium">{data.title}</p>
          <p className="truncate text-[10px] text-muted">
            {data.seriesTitle ? `${data.seriesTitle} · ` : ""}
            {page + 1} / {total}
            {mode === "double" ? ` · spread ${spreadIndex + 1}/${spreadCount}` : ""}
          </p>
        </div>
        <div className="hidden items-center gap-1 md:flex">
          <ReaderSelect
            value={mode}
            onChange={(value) => {
              setMode(value as ReaderMode);
              void updateSettings({ defaultMode: value as ReaderMode });
            }}
            options={MODES.map((entry) => ({ value: entry, label: modeLabel(entry) }))}
            label="Reading mode"
          />
          <ReaderSelect
            value={fit}
            onChange={(value) => {
              setFit(value as FitMode);
              void updateSettings({ defaultFit: value as FitMode });
            }}
            options={FITS.map((entry) => ({ value: entry, label: fitLabel(entry) }))}
            label="Page fit"
          />
          <ReaderSelect
            value={transition}
            onChange={(value) => {
              setTransition(value as TransitionMode);
              void updateSettings({ transition: value as TransitionMode });
            }}
            options={[
              { value: "instant", label: "Instant" },
              { value: "slide", label: "Slide" },
              { value: "fade", label: "Fade" },
              { value: "page-turn", label: "Page turn" },
            ]}
            label="Transition"
          />
        </div>
        {mode === "double" && (
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => {
              const nextOffset = spreadOffset === 0 ? 1 : 0;
              setSpreadOffset(nextOffset);
              void api.setSpreadOffset(issueId, nextOffset).catch(() => undefined);
            }}
            title="Shift spread pairing so the first page is paired"
            aria-label="Toggle spread offset"
          >
            <span className="text-[10px] font-semibold">Offset {spreadOffset}</span>
          </button>
        )}
        <button
          className="btn btn-ghost btn-sm"
          onClick={() => {
            const nextDirection = rtl ? "ltr" : "rtl";
            setReadingDirection(nextDirection);
            void api.setDirectionOverride(issueId, nextDirection).catch(() => undefined);
          }}
          aria-label="Toggle reading direction"
          title={`Reading direction for this file: ${rtl ? "right to left" : "left to right"}`}
        >
          <span className="text-[10px] font-semibold">{rtl ? "RTL" : "LTR"}</span>
        </button>
        <button className="btn btn-ghost btn-sm" onClick={() => setBookmarksOpen(true)} aria-label="Bookmarks">
          <Icon name="bookmark" className="size-4" />
        </button>
        <button className="btn btn-ghost btn-sm" onClick={() => setShortcutsOpen(true)} aria-label="Shortcuts">
          <Icon name="keyboard" className="size-4" />
        </button>
        <div className="flex items-center gap-0.5">
          <button className="btn btn-ghost btn-sm" onClick={() => zoom.zoomAt(zoom.zoom.scale - 0.3)} aria-label="Zoom out">
            <Icon name="zoomOut" className="size-4" />
          </button>
          <span className="w-10 text-center text-[10px] text-muted">{Math.round(zoom.zoom.scale * 100)}%</span>
          <button className="btn btn-ghost btn-sm" onClick={() => zoom.zoomAt(zoom.zoom.scale + 0.3)} aria-label="Zoom in">
            <Icon name="zoomIn" className="size-4" />
          </button>
          <button className="btn btn-ghost btn-sm" onClick={toggleFullscreen} aria-label="Toggle fullscreen">
            <Icon name="expand" className="size-4" />
          </button>
        </div>
      </div>

      <div
        ref={viewportRef}
        className={cx(
          "relative min-h-0 flex-1 touch-none overflow-hidden",
          zoom.dragging ? "cursor-grabbing" : zoom.zoom.scale > 1 ? "cursor-grab" : "",
        )}
        onWheel={mode === "vertical" ? undefined : zoom.onWheel}
        onPointerDown={zoom.onPointerDown}
        onPointerMove={zoom.onPointerMove}
        onPointerUp={zoom.endDrag}
        onPointerCancel={zoom.endDrag}
        onClick={(event) => {
          if (event.detail === 2) {
            zoom.toggleAt(event.clientX, event.clientY);
          }
        }}
      >
        {mode === "horizontal" ? (
          <div
            ref={scrollRef}
            className="flex h-full items-center overflow-x-auto overflow-y-hidden px-4 pb-14 pt-16"
            data-scroll="true"
          >
            <div
              className={cx("flex h-full max-h-full w-max items-center", rtl && "flex-row-reverse")}
              style={{ gap: settings.pageSpacing }}
            >
              {pages.map((entry, index) => (
                <img
                  key={entry.index}
                  data-page={index}
                  src={pageUrl(data.fileId, index)}
                  alt={`Page ${index + 1}`}
                  draggable={false}
                  loading={Math.abs(index - page) <= 2 ? "eager" : "lazy"}
                  width={entry.width || undefined}
                  height={entry.height || undefined}
                  className="reader-page h-auto max-h-full w-auto select-none rounded-sm"
                  style={{
                    objectFit: "contain",
                    maxHeight: "100%",
                    width: fit === "original" ? `${entry.width}px` : undefined,
                  }}
                />
              ))}
            </div>
          </div>
        ) : mode === "vertical" ? (
          <div ref={scrollRef} className="h-full overflow-y-auto" data-scroll="true">
            <div className="mx-auto flex w-full flex-col items-center" style={{ maxWidth: 1000 }}>
              {pages.map((entry, index) => (
                <img
                  key={entry.index}
                  data-page={index}
                  src={pageUrl(data.fileId, index)}
                  alt={`Page ${index + 1}`}
                  draggable={false}
                  loading={Math.abs(index - page) <= 2 ? "eager" : "lazy"}
                  width={entry.width || undefined}
                  height={entry.height || undefined}
                  className="reader-page w-full select-none"
                  style={{
                    marginBottom: settings.pageSpacing,
                    width: fit === "width" || fit === "page" ? "100%" : undefined,
                    height: "auto",
                    aspectRatio:
                      entry.width && entry.height
                        ? `${entry.width} / ${entry.height}`
                        : undefined,
                  }}
                />
              ))}
            </div>
          </div>
        ) : (
          <div
            className="relative flex h-full w-full items-center justify-center p-4"
            style={{ transitionDuration: `${settings.transitionDuration}ms` }}
          >
            <div
              className={cx(
                "flex items-center justify-center gap-2 will-change-transform",
                settings.animationsEnabled && transitioning && transition !== "instant"
                  ? transition === "fade"
                    ? "opacity-0"
                    : transition === "slide"
                      ? "translate-x-3 opacity-70"
                      : "rotate-1 scale-[0.985] opacity-0"
                  : "opacity-100",
              )}
              style={{
                transform: `translate(${zoom.zoom.pan.x}px, ${zoom.zoom.pan.y}px) scale(${zoom.zoom.scale})`,
                transitionProperty: "transform, opacity",
                transitionDuration: `${settings.transitionDuration}ms`,
              }}
            >
              {(mode === "double" ? spreads[spreadIndex]?.pages ?? [page] : [page]).map((index) => {
                const pageInfo = pages[index];
                if (!pageInfo) return null;
                return (
                  <img
                    key={`${index}-${pageInfo.name}`}
                    src={pageUrl(data.fileId, index)}
                    alt={`Page ${index + 1}`}
                    draggable={false}
                    className="reader-page max-h-full w-auto select-none rounded-sm shadow-2xl shadow-black/60"
                    style={fitStyle(fit, pageInfo, mode === "double" && (spreads[spreadIndex]?.pages.length ?? 1) > 1)}
                  />
                );
              })}
            </div>
          </div>
        )}

        {(mode === "vertical" || mode === "horizontal") && (
          <div
            className={cx(
              "pointer-events-none absolute inset-x-0 bottom-0 z-20 flex items-center justify-center gap-3 bg-gradient-to-t from-[color-mix(in_srgb,var(--panel)_92%,transparent)] to-transparent px-4 pb-3 pt-10 transition-opacity",
              chrome ? "opacity-100" : "opacity-0",
            )}
          >
            <span className="text-[11px] text-white/70">
              {page + 1} / {total}
            </span>
          </div>
        )}

        {mode !== "vertical" && (
          <>
            <EdgeButton side="left" onClick={prev} visible={chrome || zoom.zoom.scale === 1} />
            <EdgeButton side="right" onClick={next} visible={chrome || zoom.zoom.scale === 1} />
          </>
        )}
      </div>

      <div
        className={cx(
          "absolute inset-x-0 bottom-0 z-30 border-t border-[var(--line)] bg-[color-mix(in_srgb,var(--panel)_88%,transparent)] px-4 py-2 backdrop-blur-md transition-transform duration-300",
          chrome ? "translate-y-0" : "translate-y-full",
        )}
      >
        <div className="flex items-center gap-3">
          <button className="btn btn-ghost btn-sm" onClick={prev} aria-label="Previous page">
            <Icon name="chevronLeft" className="size-4" />
          </button>
          <input
            type="range"
            min={0}
            max={Math.max(0, total - 1)}
            value={page}
            onChange={(event) => {
              const value = Number(event.currentTarget.value);
              if (mode === "double") goToSpread(spreadForPage(spreads, value));
              else goToPage(value);
            }}
            className="h-1 flex-1 cursor-pointer appearance-none rounded-full bg-[var(--surface-3)] accent-[var(--accent)]"
            aria-label="Page position"
          />
          <button className="btn btn-ghost btn-sm" onClick={next} aria-label="Next page">
            <Icon name="chevronRight" className="size-4" />
          </button>
          <span className="w-16 text-right text-[11px] tabular-nums text-muted">
            {page + 1} / {total}
          </span>
          <span className="hidden text-[10px] text-muted sm:inline">{Math.round(ratio * 100)}%</span>
        </div>
      </div>

      {bookmarksOpen && (
        <div className="absolute inset-0 z-40 flex justify-end">
          <div className="absolute inset-0 bg-black/55" onClick={() => setBookmarksOpen(false)} />
          <aside className="relative flex h-full w-80 flex-col border-l border-[var(--line)] bg-[var(--panel)] animate-rise">
            <header className="flex items-center justify-between border-b border-[var(--line)] px-4 py-3">
              <h2 className="text-sm font-semibold">Bookmarks</h2>
              <button className="btn btn-ghost btn-sm" onClick={() => setBookmarksOpen(false)}>
                <Icon name="x" className="size-4" />
              </button>
            </header>
            <div className="space-y-1.5 border-b border-[var(--line)] p-3">
              <div className="flex gap-1">
                <input
                  value={bookmarkLabel}
                  onChange={(event) => setBookmarkLabel(event.currentTarget.value)}
                  placeholder={`Label for page ${page + 1}`}
                  className="input h-8 text-xs"
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void addBookmark();
                  }}
                />
                <button className="btn btn-sm btn-primary" onClick={addBookmark}>
                  Add
                </button>
              </div>
              <textarea
                value={bookmarkNote}
                onChange={(event) => setBookmarkNote(event.currentTarget.value)}
                placeholder="Note (optional)"
                className="input min-h-14 resize-none py-1.5 text-xs"
              />
            </div>
            <div className="flex-1 overflow-y-auto p-2">
              {bookmarks.length === 0 && (
                <p className="px-2 py-6 text-center text-xs text-muted">No bookmarks yet.</p>
              )}
              {bookmarks.map((mark) => (
                <div key={mark.id} className="group rounded-lg px-2 py-1.5 hover:bg-[var(--surface-2)]">
                  <div className="flex items-center gap-2">
                    <button
                      className="flex-1 text-left text-xs"
                      onClick={() => {
                        goToPage(mark.page);
                        setBookmarksOpen(false);
                      }}
                    >
                      <span className="font-medium">Page {mark.page + 1}</span>
                      <span className="ml-2 text-muted">{mark.label}</span>
                    </button>
                    <button
                      className="rounded p-1 text-muted hover:text-[var(--text)]"
                      aria-label={mark.note ? "Edit bookmark note" : "Add bookmark note"}
                      onClick={() => {
                        setEditingNoteId(editingNoteId === mark.id ? null : mark.id);
                        setNoteDraft(mark.note ?? "");
                      }}
                    >
                      <Icon name="edit" className="size-3.5" />
                    </button>
                    <button
                      className="rounded p-1 text-muted opacity-0 hover:text-rose-300 group-hover:opacity-100"
                      aria-label="Delete bookmark"
                      onClick={async () => {
                        await api.deleteBookmark(mark.id);
                        setBookmarks((current) => current.filter((item) => item.id !== mark.id));
                      }}
                    >
                      <Icon name="trash" className="size-3.5" />
                    </button>
                  </div>
                  {editingNoteId === mark.id ? (
                    <div className="mt-1 flex gap-1">
                      <textarea
                        value={noteDraft}
                        onChange={(event) => setNoteDraft(event.currentTarget.value)}
                        placeholder="Note"
                        className="input min-h-12 resize-none py-1 text-[11px]"
                      />
                      <button
                        className="btn btn-sm btn-primary"
                        onClick={async () => {
                          const updated = await api.updateBookmarkNote(mark.id, noteDraft.trim() || null);
                          setBookmarks((current) =>
                            current.map((item) =>
                              item.id === mark.id ? { ...item, note: updated.note } : item,
                            ),
                          );
                          setEditingNoteId(null);
                          toast.push("Bookmark note saved", "success");
                        }}
                      >
                        Save
                      </button>
                    </div>
                  ) : (
                    mark.note && <p className="mt-1 whitespace-pre-wrap text-[11px] text-muted">{mark.note}</p>
                  )}
                </div>
              ))}
            </div>
          </aside>
        </div>
      )}

      {shortcutsOpen && (
        <div className="absolute inset-0 z-40 flex items-center justify-center p-4">
          <div className="absolute inset-0 bg-black/60" onClick={() => setShortcutsOpen(false)} />
          <div className="relative w-full max-w-md rounded-2xl border border-[var(--line)] bg-[var(--panel)] p-5 animate-pop">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="font-display text-sm font-semibold">Keyboard shortcuts</h2>
              <button className="btn btn-ghost btn-sm" onClick={() => setShortcutsOpen(false)}>
                <Icon name="x" className="size-4" />
              </button>
            </div>
            <dl className="space-y-1.5 text-xs">
              {[
                ["Next page", settings.shortcuts.nextPage],
                ["Previous page", settings.shortcuts.prevPage],
                ["Next issue", settings.shortcuts.nextIssue],
                ["Previous issue", settings.shortcuts.prevIssue],
                ["Toggle zoom", settings.shortcuts.toggleZoom],
                ["Fullscreen", settings.shortcuts.toggleFullscreen],
                ["Bookmark", settings.shortcuts.bookmark],
                ["Cycle reading mode", "D"],
                ["Cycle page fit", "1 2 3 4"],
                ["Hide interface", "H"],
                ["Back / reset zoom", settings.shortcuts.escape],
              ].map(([label, keys]) => (
                <div key={label} className="flex items-center justify-between gap-3">
                  <dt className="text-muted">{label}</dt>
                  <dd className="rounded border border-[var(--line)] bg-[var(--surface-2)] px-1.5 py-0.5 font-mono text-[10px]">
                    {keys}
                  </dd>
                </div>
              ))}
            </dl>
            <p className="mt-3 text-[11px] text-muted">
              Customise these in Settings. Shortcuts are ignored while typing in a text field.
            </p>
          </div>
        </div>
      )}

      {zoom.zoom.scale > 1 && (
        <button
          className="absolute right-4 top-16 z-30 btn btn-sm"
          onClick={zoom.reset}
          title="Reset zoom"
        >
          Reset zoom · {Math.round(zoom.zoom.scale * 100)}%
        </button>
      )}
    </div>
  );
}

function fitStyle(fit: FitMode, pageInfo: { width: number; height: number }, double: boolean) {
  const base = double ? { maxWidth: "calc(50% - 0.25rem)" as const } : { maxWidth: "100%" as const };
  switch (fit) {
    case "width":
      return { ...base, width: "100%", height: "auto" };
    case "height":
      return { ...base, height: "100%", width: "auto", maxHeight: "calc(100vh - 7rem)" };
    case "original":
      return {
        ...base,
        width: `${pageInfo.width}px`,
        height: "auto",
        maxWidth: "none",
      };
    default:
      return { ...base, maxHeight: "calc(100vh - 7rem)", width: "auto" };
  }
}

function EdgeButton({
  side,
  onClick,
  visible,
}: {
  side: "left" | "right";
  onClick: () => void;
  visible: boolean;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={side === "left" ? "Previous page" : "Next page"}
      className={cx(
        "group absolute top-0 z-20 flex h-full w-16 items-center justify-center transition-opacity",
        side === "left" ? "left-0" : "right-0",
        visible ? "opacity-100" : "opacity-0",
      )}
    >
      <span
        className={cx(
          "grid size-9 place-items-center rounded-full border border-[var(--line)] bg-[color-mix(in_srgb,var(--panel)_75%,transparent)] text-[var(--text)] opacity-0 backdrop-blur transition-all group-hover:opacity-100",
          side === "left" ? "group-hover:-translate-x-0.5" : "group-hover:translate-x-0.5",
        )}
      >
        <Icon name={side === "left" ? "chevronLeft" : "chevronRight"} className="size-5" />
      </span>
    </button>
  );
}

function ReaderSelect({
  value,
  onChange,
  options,
  label,
}: {
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  label: string;
}) {
  return (
    <select
      value={value}
      onChange={(event) => onChange(event.currentTarget.value)}
      className="h-7 rounded-md border border-[var(--line)] bg-[var(--surface-2)] px-2 text-[11px] text-[var(--text)] outline-none"
      aria-label={label}
    >
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
