import { describe, expect, it } from "vitest";
import { clamp01, offsetWithin, scrollPositionFor } from "./scroll";

describe("offsetWithin", () => {
  it("returns the fractional position of the centre inside a page", () => {
    expect(offsetWithin(550, 500, 200)).toBeCloseTo(0.25);
    expect(offsetWithin(500, 500, 200)).toBe(0);
    expect(offsetWithin(700, 500, 200)).toBe(1);
  });

  it("clamps to 0..1 and tolerates a zero-size page", () => {
    expect(offsetWithin(100, 500, 200)).toBe(0);
    expect(offsetWithin(900, 500, 200)).toBe(1);
    expect(offsetWithin(900, 500, 0)).toBe(0);
    expect(offsetWithin(900, 500, Number.NaN)).toBe(0);
  });
});

describe("scrollPositionFor", () => {
  it("centres the saved offset in the viewport", () => {
    expect(scrollPositionFor(1000, 400, 0.5, 800)).toBe(800);
    expect(scrollPositionFor(1000, 400, 0, 800)).toBe(600);
    expect(scrollPositionFor(1000, 400, 1, 800)).toBe(1000);
  });

  it("never goes negative", () => {
    expect(scrollPositionFor(0, 400, 0, 900)).toBe(0);
  });

  it("round-trips with offsetWithin", () => {
    const start = 2000;
    const size = 600;
    const viewport = 500;
    const scroll = scrollPositionFor(start, size, 0.6, viewport);
    expect(offsetWithin(scroll + viewport / 2, start, size)).toBeCloseTo(0.6);
  });
});

describe("clamp01", () => {
  it("clamps and rejects non-finite input", () => {
    expect(clamp01(-1)).toBe(0);
    expect(clamp01(2)).toBe(1);
    expect(clamp01(Number.POSITIVE_INFINITY)).toBe(0);
    expect(clamp01(0.42)).toBeCloseTo(0.42);
  });
});
