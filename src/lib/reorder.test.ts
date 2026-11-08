import { describe, expect, it } from "vitest";
import { moveItem } from "./reorder";

describe("moveItem", () => {
  it("moves an item forward", () => {
    expect(moveItem(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
  });

  it("moves an item backward", () => {
    expect(moveItem(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
  });

  it("returns the same array for no-op moves", () => {
    const items = ["a", "b", "c"];
    expect(moveItem(items, 1, 1)).toBe(items);
    expect(moveItem(items, -1, 2)).toBe(items);
    expect(moveItem(items, 0, 5)).toBe(items);
  });
});
