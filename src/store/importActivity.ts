import { create } from "zustand";

interface ImportActivity {
  active: boolean;
  label: string;
  start: (label: string) => void;
  end: () => void;
}

/**
 * Tracks any long-running library job (folder import, rescan) so the sidebar
 * progress panel and its cancel button appear for both. Progress payloads
 * themselves arrive through the `import-progress` Tauri event.
 */
export const useImportActivity = create<ImportActivity>((set) => ({
  active: false,
  label: "",
  start: (label) => set({ active: true, label }),
  end: () => set({ active: false, label: "" }),
}));
