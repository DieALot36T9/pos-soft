import { describe, expect, it } from "vitest";

import { categoryPaths } from "./categoryPaths";

describe("categoryPaths", () => {
  it("cat_03: shows each category under its parents, sorted", () => {
    const paths = categoryPaths([
      { id: "b", parent_id: "a", name: "Milk", sort: 0, archived_at: null },
      { id: "a", parent_id: null, name: "Dairy", sort: 0, archived_at: null },
      { id: "c", parent_id: null, name: "Bakery", sort: 0, archived_at: null },
    ]);
    expect(paths.map((each) => each.path)).toEqual(["Bakery", "Dairy", "Dairy › Milk"]);
    expect(paths[2]?.depth).toBe(1);
  });
});
