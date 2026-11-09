export interface Combo {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  key: string;
}

/**
 * Normalises a binding string such as "Ctrl+ArrowRight, Space" into canonical
 * combos so two bindings can be compared regardless of spacing or case.
 */
export function parseBindings(binding: string | undefined): Combo[] {
  if (!binding) return [];
  return binding
    .split(",")
    .map((combo) => combo.trim())
    .filter(Boolean)
    .map((combo) => {
      const parts = combo.split("+").map((part) => part.trim().toLowerCase());
      let ctrl = false;
      let shift = false;
      let alt = false;
      let key = "";
      for (const part of parts) {
        if (part === "ctrl" || part === "cmd" || part === "meta") ctrl = true;
        else if (part === "shift") shift = true;
        else if (part === "alt") alt = true;
        else key = part;
      }
      return { ctrl, shift, alt, key: canonicalKey(key) };
    });
}

export function canonicalKey(key: string): string {
  if (key === "space") return " ";
  if (key === "esc") return "escape";
  return key.toLowerCase();
}

export function comboId(combo: Combo): string {
  return `${combo.ctrl ? "ctrl+" : ""}${combo.shift ? "shift+" : ""}${combo.alt ? "alt+" : ""}${combo.key}`;
}

export function formatCombo(combo: Combo): string {
  const parts: string[] = [];
  if (combo.ctrl) parts.push("Ctrl");
  if (combo.shift) parts.push("Shift");
  if (combo.alt) parts.push("Alt");
  parts.push(combo.key === " " ? "Space" : combo.key.toUpperCase());
  return parts.join(" + ");
}

export interface ShortcutConflict {
  combo: string;
  keys: string[];
}

/**
 * Returns every key combination bound to more than one action. Bindings are
 * validated per action rather than globally because the reader and the shell
 * intentionally reuse Escape.
 */
export function findConflicts(shortcuts: Record<string, string>): ShortcutConflict[] {
  const owners = new Map<string, Set<string>>();
  for (const [action, binding] of Object.entries(shortcuts)) {
    for (const combo of parseBindings(binding)) {
      const id = comboId(combo);
      const set = owners.get(id) ?? new Set<string>();
      set.add(action);
      owners.set(id, set);
    }
  }
  return [...owners.entries()]
    .filter(([, actions]) => actions.size > 1)
    .map(([combo, actions]) => ({ combo, keys: [...actions].sort() }));
}

export function invalidBindings(shortcuts: Record<string, string>): string[] {
  return Object.entries(shortcuts)
    .filter(([, binding]) => parseBindings(binding).some((combo) => combo.key === ""))
    .map(([action]) => action);
}
