import type { PageInfo } from "./types";

export interface Spread {
  pages: number[];
  index: number;
}

export function buildSpreads(
  pages: PageInfo[],
  mode: "single" | "double",
  offset = 0,
): Spread[] {
  if (mode === "single" || pages.length <= 1) {
    return pages.map((_, index) => ({ pages: [index], index }));
  }

  const spreads: Spread[] = [];
  let cursor = 0;

  const isWide = (index: number) => {
    const page = pages[index];
    if (!page || page.width === 0 || page.height === 0) return false;
    return page.width / page.height > 1.15;
  };

  if (pages.length > 0) {
    if (offset === 0) {
      spreads.push({ pages: [0], index: spreads.length });
      cursor = 1;
    } else {
      spreads.push({ pages: [0, 1].filter((index) => index < pages.length), index: spreads.length });
      cursor = 2;
    }
  }

  while (cursor < pages.length) {
    if (isWide(cursor)) {
      spreads.push({ pages: [cursor], index: spreads.length });
      cursor += 1;
      continue;
    }
    if (cursor + 1 < pages.length && !isWide(cursor + 1)) {
      spreads.push({ pages: [cursor, cursor + 1], index: spreads.length });
      cursor += 2;
    } else {
      spreads.push({ pages: [cursor], index: spreads.length });
      cursor += 1;
    }
  }

  return spreads;
}

export function spreadForPage(spreads: Spread[], page: number): number {
  const found = spreads.findIndex((spread) => spread.pages.includes(page));
  return found === -1 ? 0 : found;
}

export function pageForSpread(spreads: Spread[], spreadIndex: number): number {
  const spread = spreads[Math.max(0, Math.min(spreads.length - 1, spreadIndex))];
  return spread ? spread.pages[0] : 0;
}

export function firstPageOfSpread(spreads: Spread[], spreadIndex: number): number {
  return pageForSpread(spreads, spreadIndex);
}

export function lastPageOfSpread(spreads: Spread[], spreadIndex: number): number {
  const spread = spreads[Math.max(0, Math.min(spreads.length - 1, spreadIndex))];
  if (!spread) return 0;
  return spread.pages[spread.pages.length - 1];
}
