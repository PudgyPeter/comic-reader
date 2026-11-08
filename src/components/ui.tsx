import { useEffect, useRef, useState } from "react";
import { coverUrl } from "../lib/api";
import { cx } from "../lib/utils";

interface CoverProps {
  fileId: string | null;
  page?: number | null;
  title: string;
  className?: string;
  width?: number;
  rounded?: string;
}

export function Cover({
  fileId,
  page = 0,
  title,
  className,
  width = 320,
  rounded = "rounded-lg",
}: CoverProps) {
  const src = coverUrl(fileId, page ?? 0, width);
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const [loadedSrc, setLoadedSrc] = useState<string | null>(null);
  const failed = failedSrc !== null && failedSrc === src;
  const loaded = loadedSrc !== null && loadedSrc === src;

  const initials = title
    .split(/\s+/)
    .slice(0, 3)
    .map((word) => word[0])
    .join("")
    .toUpperCase();

  return (
    <div
      className={cx(
        "relative overflow-hidden bg-[var(--surface-2)]",
        rounded,
        className,
      )}
      aria-label={`${title} cover`}
    >
      <div
        className="absolute inset-0 grid place-items-center"
        style={{
          backgroundImage:
            "linear-gradient(140deg, color-mix(in srgb, var(--text) 12%, var(--panel)), color-mix(in srgb, var(--text) 3%, var(--panel)) 60%)",
        }}
      >
        <span className="font-display text-2xl font-semibold text-[var(--muted)]">
          {initials || "?"}
        </span>
      </div>
      {src && !failed && (
        <img
          src={src}
          alt=""
          loading="lazy"
          draggable={false}
          onLoad={() => setLoadedSrc(src)}
          onError={() => setFailedSrc(src)}
          className={cx(
            "relative h-full w-full object-cover transition-opacity duration-500",
            loaded ? "opacity-100" : "opacity-0",
          )}
        />
      )}
    </div>
  );
}

export function ProgressBar({
  value,
  className,
  tone = "accent",
}: {
  value: number;
  className?: string;
  tone?: "accent" | "muted";
}) {
  const pct = Math.round(Math.min(1, Math.max(0, value)) * 100);
  const hasWidth = Boolean(className && /(^|\s)w-/.test(className));
  return (
    <div
      className={cx(
        "h-1 overflow-hidden rounded-full bg-[var(--surface-3)]",
        !hasWidth && "w-full",
        className,
      )}
      role="progressbar"
      aria-valuenow={pct}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div
        className={cx(
          "h-full rounded-full transition-[width] duration-500",
          tone === "accent" ? "bg-[var(--accent)]" : "bg-ink-300",
        )}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

export function StatusBadge({ status }: { status: string }) {
  if (status === "completed") {
    return (
      <span className="chip border-emerald-400/30 bg-emerald-400/10 text-emerald-300">
        Read
      </span>
    );
  }
  if (status === "in-progress") {
    return (
      <span className="chip border-amber-400/30 bg-amber-400/10 text-amber-300">
        Reading
      </span>
    );
  }
  return <span className="chip">Unread</span>;
}

export function Spinner({ className }: { className?: string }) {
  return (
    <span
      className={cx(
        "inline-block size-4 animate-spin rounded-full border-2 border-[var(--line-strong)] border-t-[var(--text)]",
        className,
      )}
      role="status"
      aria-label="Loading"
    />
  );
}

interface ModalProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: React.ReactNode;
  footer?: React.ReactNode;
  width?: string;
}

export function Modal({ open, title, onClose, children, footer, width = "max-w-lg" }: ModalProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", onKey, true);
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => {
      document.removeEventListener("keydown", onKey, true);
      previous?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/65 backdrop-blur-sm animate-fade"
        onClick={onClose}
      />
      <div
        ref={ref}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cx(
          "relative w-full overflow-hidden rounded-2xl border border-[var(--line)] bg-[var(--panel)] shadow-2xl animate-pop",
          width,
        )}
      >
        <header className="flex items-center justify-between border-b border-[var(--line)] px-5 py-4">
          <h2 className="font-display text-base font-semibold tracking-tight">{title}</h2>
          <button className="btn btn-ghost btn-sm" onClick={onClose} aria-label="Close">
            Esc
          </button>
        </header>
        <div className="max-h-[70vh] overflow-y-auto px-5 py-4">{children}</div>
        {footer && (
          <footer className="flex justify-end gap-2 border-t border-[var(--line)] px-5 py-3">
            {footer}
          </footer>
        )}
      </div>
    </div>
  );
}

export function EmptyState({
  title,
  body,
  action,
  icon,
}: {
  title: string;
  body: string;
  action?: React.ReactNode;
  icon?: React.ReactNode;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-[var(--line)] px-6 py-16 text-center animate-rise">
      <div className="grid size-12 place-items-center rounded-xl border border-[var(--line)] bg-[var(--surface-1)] text-[var(--accent)]">
        {icon ?? "◫"}
      </div>
      <h3 className="font-display text-lg font-semibold">{title}</h3>
      <p className="max-w-sm text-sm leading-relaxed text-muted">{body}</p>
      {action}
    </div>
  );
}
