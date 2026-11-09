import { useState } from "react";
import { useSettings } from "../store/settings";
import { useToast } from "../components/Toast";
import { Icon } from "../components/Icon";
import { cx, fitLabel, modeLabel, readingDirectionLabel } from "../lib/utils";
import { findConflicts, invalidBindings, parseBindings, formatCombo } from "../lib/shortcuts";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api";

const SHORTCUT_LABELS: Record<string, string> = {
  nextPage: "Next page",
  prevPage: "Previous page",
  nextIssue: "Next issue",
  prevIssue: "Previous issue",
  toggleFullscreen: "Toggle fullscreen",
  toggleZoom: "Toggle zoom",
  escape: "Escape / back",
  bookmark: "Add bookmark",
  search: "Search",
};

export default function SettingsPage() {
  const settings = useSettings((state) => state.settings);
  const update = useSettings((state) => state.update);
  const reset = useSettings((state) => state.reset);
  const toast = useToast();
  const [exporting, setExporting] = useState(false);
  const orders = useQuery({ queryKey: ["orders"], queryFn: api.readingOrders });
  const conflicts = findConflicts(settings.shortcuts);
  const invalid = invalidBindings(settings.shortcuts);

  const exportData = async () => {
    setExporting(true);
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const target = await save({
        defaultPath: "panel-library-export.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!target || Array.isArray(target)) return;
      await api.exportLibraryTo(target);
      toast.push("Library exported", "success");
    } catch (error) {
      toast.push(String(error), "error");
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="mx-auto w-full max-w-[900px] px-6 py-6">
      <h1 className="font-display text-2xl font-semibold tracking-tight">Settings</h1>
      <p className="mt-1 text-[13px] text-muted">
        Reader defaults, appearance, and keyboard bindings. Changes save automatically.
      </p>

      <Section title="Reader defaults" description="Applied when an issue has no per-issue override.">
        <Field label="Reading mode">
          <select
            className="input"
            value={settings.defaultMode}
            onChange={(event) =>
              update({ defaultMode: event.currentTarget.value as typeof settings.defaultMode })
            }
          >
            {(["single", "double", "vertical", "horizontal"] as const).map((mode) => (
              <option key={mode} value={mode}>
                {modeLabel(mode)}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Page fit">
          <select
            className="input"
            value={settings.defaultFit}
            onChange={(event) =>
              update({ defaultFit: event.currentTarget.value as typeof settings.defaultFit })
            }
          >
            {(["page", "width", "height", "original"] as const).map((fit) => (
              <option key={fit} value={fit}>
                {fitLabel(fit)}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Reading direction">
          <select
            className="input"
            value={settings.readingDirection}
            onChange={(event) =>
              update({
                readingDirection: event.currentTarget.value as typeof settings.readingDirection,
              })
            }
          >
            <option value="ltr">{readingDirectionLabel("ltr")}</option>
            <option value="rtl">{readingDirectionLabel("rtl")}</option>
          </select>
          <p className="mt-1 text-[11px] text-muted">
            Used when an issue's series or file has no direction set. Override per series on the
            series page, or per file from the reader's direction button.
          </p>
        </Field>
        <Field label="Zoom behaviour">
          <select
            className="input"
            value={settings.defaultZoomBehavior}
            onChange={(event) =>
              update({
                defaultZoomBehavior: event.currentTarget.value as typeof settings.defaultZoomBehavior,
              })
            }
          >
            <option value="cursor">Zoom toward cursor</option>
            <option value="center">Zoom toward centre</option>
            <option value="page">Zoom page only</option>
          </select>
        </Field>
      </Section>

      <Section title="Transitions" description="Navigation is instant when animations are disabled.">
        <Field label="Style">
          <select
            className="input"
            value={settings.transition}
            onChange={(event) =>
              update({ transition: event.currentTarget.value as typeof settings.transition })
            }
          >
            <option value="instant">Instant</option>
            <option value="slide">Horizontal slide</option>
            <option value="fade">Fade</option>
            <option value="page-turn">Page turn</option>
          </select>
        </Field>
        <Field label={`Duration — ${settings.transitionDuration} ms`}>
          <input
            type="range"
            min={0}
            max={700}
            step={20}
            value={settings.transitionDuration}
            disabled={!settings.animationsEnabled}
            onChange={(event) =>
              update({ transitionDuration: Number(event.currentTarget.value) })
            }
            className="w-full accent-[var(--accent)]"
          />
        </Field>
        <Toggle
          label="Enable animations"
          description="Respects the operating system reduced-motion preference as well."
          value={settings.animationsEnabled}
          onChange={(value) => update({ animationsEnabled: value })}
        />
      </Section>

      <Section title="Appearance" description="Reader background, spacing and interface density.">
        <Field label="Theme">
          <div className="flex gap-2">
            {(["dark", "light", "midnight"] as const).map((theme) => (
              <button
                key={theme}
                onClick={() => update({ theme })}
                className={cx(
                  "btn flex-1 capitalize",
                  settings.theme === theme && "border-[var(--accent)]/50 text-[var(--text-strong)]",
                )}
              >
                {theme}
              </button>
            ))}
          </div>
        </Field>
        <Field label="Accent colour">
          <div className="flex items-center gap-2">
            <input
              type="color"
              value={settings.accent}
              onChange={(event) => update({ accent: event.currentTarget.value })}
              className="h-9 w-14 cursor-pointer rounded border border-[var(--line)] bg-transparent"
              aria-label="Accent colour"
            />
            <span className="font-mono text-xs text-muted">{settings.accent}</span>
          </div>
        </Field>
        <Field label="Reader background">
          <div className="flex items-center gap-2">
            <input
              type="color"
              value={settings.background}
              onChange={(event) => update({ background: event.currentTarget.value })}
              className="h-9 w-14 cursor-pointer rounded border border-[var(--line)] bg-transparent"
              aria-label="Reader background"
            />
            <span className="font-mono text-xs text-muted">{settings.background}</span>
          </div>
        </Field>
        <Field label={`Page spacing — ${settings.pageSpacing}px`}>
          <input
            type="range"
            min={0}
            max={64}
            step={4}
            value={settings.pageSpacing}
            onChange={(event) => update({ pageSpacing: Number(event.currentTarget.value) })}
            className="w-full accent-[var(--accent)]"
          />
        </Field>
        <Field label="Interface density">
          <select
            className="input"
            value={settings.density}
            onChange={(event) =>
              update({ density: event.currentTarget.value as typeof settings.density })
            }
          >
            <option value="compact">Compact sidebar</option>
            <option value="comfortable">Comfortable</option>
            <option value="spacious">Spacious</option>
          </select>
        </Field>
        <Toggle
          label="Show reader interface by default"
          description="Chrome can still be hidden during reading with H."
          value={settings.showInterface}
          onChange={(value) => update({ showInterface: value })}
        />
      </Section>

      <Section title="Progress and spoilers" description="How completion is detected and what is hidden.">
        <Field label={`Completion threshold — ${Math.round(settings.completionThreshold * 100)}%`}>
          <input
            type="range"
            min={50}
            max={100}
            step={1}
            value={Math.round(settings.completionThreshold * 100)}
            onChange={(event) =>
              update({ completionThreshold: Number(event.currentTarget.value) / 100 })
            }
            className="w-full accent-[var(--accent)]"
          />
          <p className="mt-1 text-[11px] text-muted">
            An issue is marked complete when you reach this share of its pages, or when you mark it
            manually.
          </p>
        </Field>
        <Toggle
          label="Spoiler protection"
          description="Hides descriptions of issues beyond your reading frontier."
          value={settings.spoilerProtection}
          onChange={(value) => update({ spoilerProtection: value })}
        />
        <Field label="Spoiler frontier source">
          <select
            className="input"
            value={settings.spoilerOrderId ?? ""}
            disabled={!settings.spoilerProtection}
            onChange={(event) =>
              update({ spoilerOrderId: event.currentTarget.value || null })
            }
          >
            <option value="">Per-series progress (default)</option>
            {(orders.data ?? []).map((order) => (
              <option key={order.id} value={order.id}>
                Follow reading order: {order.name}
              </option>
            ))}
          </select>
          <p className="mt-1 text-[11px] text-muted">
            Following a custom order hides everything after your furthest completed position in
            that order, matching how you actually read the universe.
          </p>
        </Field>
      </Section>

      <Section title="Keyboard shortcuts" description="Comma-separate multiple bindings. Use Ctrl, Shift and Alt modifiers.">
        {(conflicts.length > 0 || invalid.length > 0) && (
          <div className="mb-3 rounded-lg border border-[var(--warn)]/40 bg-[var(--warn)]/8 px-3 py-2">
            {conflicts.map((conflict) => (
              <p key={conflict.combo} className="text-[11px] text-[var(--warn)]">
                <span className="font-mono">{formatCombo(parseBindings(conflict.combo)[0] ?? { ctrl: false, shift: false, alt: false, key: conflict.combo })}</span>
                {" "}is bound to {conflict.keys.map((key) => SHORTCUT_LABELS[key] ?? key).join(" and ")}.
              </p>
            ))}
            {invalid.map((action) => (
              <p key={action} className="text-[11px] text-[var(--warn)]">
                {SHORTCUT_LABELS[action] ?? action} has a binding with no key (only modifiers).
              </p>
            ))}
            <p className="mt-1 text-[10px] text-muted">
              Conflicting bindings still work, but only the first matching action runs.
            </p>
          </div>
        )}
        <div className="space-y-2">
          {Object.keys(SHORTCUT_LABELS).map((key) => (
            <div key={key} className="flex items-center gap-3">
              <span className="w-40 shrink-0 text-xs text-muted">{SHORTCUT_LABELS[key]}</span>
              <input
                className="input h-8 flex-1 font-mono text-xs"
                value={settings.shortcuts[key] ?? ""}
                onChange={(event) =>
                  update({
                    shortcuts: {
                      ...settings.shortcuts,
                      [key]: event.currentTarget.value,
                    },
                  })
                }
                aria-label={`${SHORTCUT_LABELS[key]} binding`}
              />
            </div>
          ))}
        </div>
        <details className="mt-4 rounded-lg border border-[var(--line)] bg-[var(--surface-1)] px-3 py-2">
          <summary className="cursor-pointer text-xs font-medium text-[var(--text-strong)]">
            Shortcut cheat sheet
          </summary>
          <dl className="mt-2 space-y-1.5 text-[11px]">
            {[
              ...Object.keys(SHORTCUT_LABELS).map((key) => [
                SHORTCUT_LABELS[key],
                settings.shortcuts[key] ?? "",
              ]),
              ["Cycle reading mode", "D"],
              ["Cycle page fit", "1 2 3 4"],
              ["Hide interface", "H"],
              ["Show this cheat sheet", "?"],
            ].map(([label, keys]) => (
              <div key={label} className="flex items-center justify-between gap-3">
                <dt className="text-muted">{label}</dt>
                <dd className="rounded border border-[var(--line)] bg-[var(--surface-2)] px-1.5 py-0.5 font-mono text-[10px]">
                  {keys || "—"}
                </dd>
              </div>
            ))}
          </dl>
        </details>
      </Section>

      <Section title="Data" description="Your library lives in a local SQLite database.">
        <div className="flex flex-wrap gap-2">
          <button className="btn" onClick={exportData} disabled={exporting}>
            <Icon name="folder" className="size-3.5" />
            {exporting ? "Exporting…" : "Export library metadata"}
          </button>
          <button
            className="btn btn-danger"
            onClick={async () => {
              await reset();
              toast.push("Settings reset to defaults", "success");
            }}
          >
            <Icon name="refresh" className="size-3.5" /> Reset settings
          </button>
        </div>
        <p className="mt-3 text-[11px] leading-relaxed text-muted">
          Comic archives are never modified. Database backups are written beside the database before
          migrations run.
        </p>
      </Section>
    </div>
  );
}

function Section({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mt-6 rounded-xl border border-[var(--line)] bg-[var(--surface-1)] p-5">
      <h2 className="font-display text-sm font-semibold">{title}</h2>
      {description && <p className="mt-0.5 text-[11px] text-muted">{description}</p>}
      <div className="mt-4 space-y-4">{children}</div>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-1.5 sm:grid-cols-[200px_1fr] sm:items-start sm:gap-4">
      <span className="pt-1.5 text-xs text-muted">{label}</span>
      <div>{children}</div>
    </div>
  );
}

function Toggle({
  label,
  description,
  value,
  onChange,
}: {
  label: string;
  description?: string;
  value: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div>
        <p className="text-xs font-medium">{label}</p>
        {description && <p className="text-[11px] text-muted">{description}</p>}
      </div>
      <button
        role="switch"
        aria-checked={value}
        aria-label={label}
        onClick={() => onChange(!value)}
        className={cx(
          "relative h-5 w-9 shrink-0 rounded-full border transition-colors",
          value
            ? "border-transparent bg-[var(--accent)]"
            : "border-[var(--line-strong)] bg-[var(--surface-3)]",
        )}
      >
        <span
          className={cx(
            "absolute top-0.5 size-3.5 rounded-full bg-white transition-transform",
            value ? "translate-x-[18px]" : "translate-x-0.5",
          )}
        />
      </button>
    </div>
  );
}
