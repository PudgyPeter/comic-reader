import { create } from "zustand";
import { api } from "../lib/api";
import type { Settings } from "../lib/types";

const DEFAULT_SETTINGS: Settings = {
  background: "#0b0c10",
  pageSpacing: 16,
  brightness: 1,
  gamma: 1,
  readingDirection: "ltr",
  defaultMode: "single",
  defaultFit: "page",
  defaultZoomBehavior: "cursor",
  transition: "slide",
  transitionDuration: 220,
  animationsEnabled: true,
  showInterface: true,
  fullscreen: false,
  completionThreshold: 0.99,
  spoilerProtection: true,
  spoilerOrderId: null,
  theme: "dark",
  accent: "#f59e0b",
  density: "comfortable",
  libraryLayout: "grid",
  librarySort: "recent",
  shortcuts: {
    nextPage: "ArrowRight, Space, PageDown",
    prevPage: "ArrowLeft, Shift+Space, PageUp",
    nextIssue: "Ctrl+ArrowRight",
    prevIssue: "Ctrl+ArrowLeft",
    toggleFullscreen: "F",
    toggleZoom: "Z",
    escape: "Escape",
    bookmark: "B",
    search: "Ctrl+F",
  },
};

interface SettingsState {
  settings: Settings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (patch: Partial<Settings>) => Promise<void>;
  reset: () => Promise<void>;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loaded: false,
  load: async () => {
    try {
      const settings = await api.getSettings();
      set({ settings: { ...DEFAULT_SETTINGS, ...settings }, loaded: true });
    } catch {
      set({ loaded: true });
    }
  },
  update: async (patch) => {
    const next = { ...get().settings, ...patch };
    set({ settings: next });
    try {
      const saved = await api.saveSettings(next);
      set({ settings: { ...next, ...saved } });
    } catch {
      set({ settings: next });
    }
  },
  reset: async () => {
    set({ settings: DEFAULT_SETTINGS });
    try {
      await api.saveSettings(DEFAULT_SETTINGS);
    } catch {
      /* ignore */
    }
  },
}));

export { DEFAULT_SETTINGS };
