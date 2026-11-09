import { describe, expect, it } from "vitest";
import { findConflicts, formatCombo, invalidBindings, parseBindings } from "./shortcuts";

describe("parseBindings", () => {
  it("parses modifiers and comma-separated alternatives", () => {
    const combos = parseBindings("Ctrl+ArrowRight, Space");
    expect(combos).toHaveLength(2);
    expect(combos[0]).toEqual({ ctrl: true, shift: false, alt: false, key: "arrowright" });
    expect(combos[1].key).toBe(" ");
  });

  it("normalises aliases and case", () => {
    expect(parseBindings("cmd+Shift+ESC")[0]).toEqual({
      ctrl: true,
      shift: true,
      alt: false,
      key: "escape",
    });
  });

  it("ignores empty bindings", () => {
    expect(parseBindings(undefined)).toEqual([]);
    expect(parseBindings("  ,  ")).toEqual([]);
  });
});

describe("formatCombo", () => {
  it("renders a readable label", () => {
    expect(formatCombo(parseBindings("ctrl+shift+Space")[0])).toBe("Ctrl + Shift + Space");
  });
});

describe("findConflicts", () => {
  it("reports a combination bound to two actions", () => {
    const conflicts = findConflicts({
      nextPage: "ArrowRight",
      nextIssue: "ArrowRight",
      prevPage: "ArrowLeft",
    });
    expect(conflicts).toHaveLength(1);
    expect(conflicts[0].keys).toEqual(["nextIssue", "nextPage"]);
  });

  it("treats different modifier sets as distinct", () => {
    expect(
      findConflicts({ nextPage: "ArrowRight", nextIssue: "Ctrl+ArrowRight" }),
    ).toEqual([]);
  });

  it("detects a conflict inside one action's alternative list", () => {
    const conflicts = findConflicts({ nextPage: "Space", prevPage: "Space" });
    expect(conflicts[0].combo).toBe(" ");
  });
});

describe("invalidBindings", () => {
  it("flags modifier-only bindings", () => {
    expect(invalidBindings({ a: "Ctrl", b: "Ctrl+x" })).toEqual(["a"]);
  });
});
