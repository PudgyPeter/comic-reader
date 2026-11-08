import { describe, expect, it } from "vitest";
import { buildSpreads, firstPageOfSpread, spreadForPage } from "./spreads";
import type { PageInfo } from "./types";

const page = (index: number, width: number, height: number): PageInfo => ({
  index,
  name: `page-${index}.png`,
  width,
  height,
});

describe("buildSpreads", () => {
  it("returns one page per spread in single mode", () => {
    const pages = [page(0, 800, 1200), page(1, 800, 1200), page(2, 800, 1200)];
    expect(buildSpreads(pages, "single").map((spread) => spread.pages)).toEqual([[0], [1], [2]]);
  });

  it("keeps the cover alone and pairs the rest", () => {
    const pages = Array.from({ length: 6 }, (_, index) => page(index, 800, 1200));
    expect(buildSpreads(pages, "double").map((spread) => spread.pages)).toEqual([
      [0],
      [1, 2],
      [3, 4],
      [5],
    ]);
  });

  it("gives wide pages their own spread", () => {
    const pages = [
      page(0, 800, 1200),
      page(1, 1600, 1000),
      page(2, 800, 1200),
      page(3, 800, 1200),
    ];
    expect(buildSpreads(pages, "double").map((spread) => spread.pages)).toEqual([
      [0],
      [1],
      [2, 3],
    ]);
  });

  it("maps pages to spread indexes", () => {
    const pages = Array.from({ length: 5 }, (_, index) => page(index, 800, 1200));
    const spreads = buildSpreads(pages, "double");
    expect(spreadForPage(spreads, 0)).toBe(0);
    expect(spreadForPage(spreads, 2)).toBe(1);
    expect(firstPageOfSpread(spreads, 2)).toBe(3);
  });

  it("handles an empty page list", () => {
    expect(buildSpreads([], "double")).toEqual([]);
  });
});

describe("spread offset", () => {
  const pages = Array.from({ length: 6 }, (_, index) => page(index, 800, 1200));

  it("pairs from the first page when offset is set", () => {
    expect(buildSpreads(pages, "double", 1).map((spread) => spread.pages)).toEqual([
      [0, 1],
      [2, 3],
      [4, 5],
    ]);
  });

  it("keeps the cover alone at offset 0", () => {
    expect(buildSpreads(pages, "double", 0).map((spread) => spread.pages)).toEqual([
      [0],
      [1, 2],
      [3, 4],
      [5],
    ]);
  });

  it("tolerates a single-page archive with offset", () => {
    const single = [page(0, 800, 1200)];
    expect(buildSpreads(single, "double", 1).map((spread) => spread.pages)).toEqual([[0]]);
  });
});
