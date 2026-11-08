import { describe, expect, it } from "vitest";
import {
  eventMatches,
  fitLabel,
  isEditableTarget,
  modeLabel,
  progressRatio,
  statusLabel,
} from "./utils";

describe("eventMatches", () => {
  const base = { key: "ArrowRight", ctrlKey: false, metaKey: false, altKey: false, shiftKey: false };

  it("matches simple bindings", () => {
    expect(eventMatches(base, "ArrowRight")).toBe(true);
    expect(eventMatches(base, "ArrowLeft")).toBe(false);
  });

  it("matches comma-separated alternatives", () => {
    expect(eventMatches(base, "ArrowLeft, ArrowRight, Space")).toBe(true);
  });

  it("requires modifiers", () => {
    expect(eventMatches(base, "Ctrl+ArrowRight")).toBe(false);
    expect(eventMatches({ ...base, ctrlKey: true }, "Ctrl+ArrowRight")).toBe(true);
    expect(eventMatches({ ...base, shiftKey: true }, "Shift+Space")).toBe(false);
    expect(eventMatches({ ...base, key: " ", shiftKey: true }, "Shift+Space")).toBe(true);
  });

  it("treats meta as ctrl for macOS parity", () => {
    expect(eventMatches({ ...base, metaKey: true }, "Ctrl+ArrowRight")).toBe(true);
  });

  it("returns false for empty bindings", () => {
    expect(eventMatches(base, undefined)).toBe(false);
    expect(eventMatches(base, "")).toBe(false);
  });
});

describe("isEditableTarget", () => {
  it("detects text inputs", () => {
    const input = document.createElement("input");
    expect(isEditableTarget(input)).toBe(true);
    const area = document.createElement("textarea");
    expect(isEditableTarget(area)).toBe(true);
    const div = document.createElement("div");
    expect(isEditableTarget(div)).toBe(false);
  });
});

describe("progressRatio", () => {
  it("returns full progress for completed issues", () => {
    expect(progressRatio({ status: "completed", lastPage: 2, pageCount: 40 })).toBe(1);
  });

  it("clamps partial progress", () => {
    expect(progressRatio({ status: "in-progress", lastPage: 10, pageCount: 40 })).toBeCloseTo(0.25);
    expect(progressRatio({ status: "in-progress", lastPage: 50, pageCount: 40 })).toBe(1);
    expect(progressRatio({ status: "in-progress", lastPage: 0, pageCount: 0 })).toBe(0);
  });
});

describe("labels", () => {
  it("maps reader modes and fits", () => {
    expect(modeLabel("double")).toBe("Double page");
    expect(modeLabel("vertical")).toBe("Vertical scroll");
    expect(fitLabel("original")).toBe("Original size");
    expect(statusLabel("in-progress")).toBe("In progress");
  });
});
