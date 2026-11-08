import { createContext, useCallback, useContext, useMemo, useState } from "react";
import { cx } from "../lib/utils";

type ToastTone = "info" | "success" | "error";

interface ToastItem {
  id: number;
  message: string;
  tone: ToastTone;
}

interface ToastApi {
  push: (message: string, tone?: ToastTone) => void;
}

const ToastContext = createContext<ToastApi>({ push: () => undefined });

let counter = 0;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);

  const push = useCallback((message: string, tone: ToastTone = "info") => {
    const id = ++counter;
    setItems((current) => [...current.slice(-3), { id, message, tone }]);
    window.setTimeout(() => {
      setItems((current) => current.filter((item) => item.id !== id));
    }, 4200);
  }, []);

  const api = useMemo(() => ({ push }), [push]);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="pointer-events-none fixed bottom-5 left-1/2 z-[100] flex w-full max-w-md -translate-x-1/2 flex-col gap-2 px-4">
        {items.map((item) => (
          <div
            key={item.id}
            role="status"
            className={cx(
              "pointer-events-auto rounded-xl border px-4 py-2.5 text-sm shadow-xl backdrop-blur animate-rise",
              item.tone === "error" &&
                "border-rose-400/30 bg-rose-950/80 text-rose-100",
              item.tone === "success" &&
                "border-emerald-400/30 bg-emerald-950/80 text-emerald-100",
              item.tone === "info" &&
                "border-[var(--line)] bg-[color-mix(in_srgb,var(--panel)_92%,transparent)] text-[var(--text)]",
            )}
          >
            {item.message}
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  return useContext(ToastContext);
}
