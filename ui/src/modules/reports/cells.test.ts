import { describe, expect, it } from "vitest";

import type { CurrencyFormat } from "../../generated/CurrencyFormat";
import { cellText, isNegative } from "./cells";

const taka: CurrencyFormat = {
  code: "BDT",
  symbol: "৳",
  symbol_before: true,
  minor_digits: 2,
  decimal_sep: ".",
  group_sep: ",",
  grouping: "western",
};

describe("report cells", () => {
  it("shows each kind of value in the shop's format", () => {
    expect(cellText({ kind: "empty" }, taka)).toBe("");
    expect(cellText({ kind: "text", value: "Bread" }, taka)).toBe("Bread");
    expect(cellText({ kind: "money", value: 123_450 }, taka)).toBe("৳1,234.50");
    expect(cellText({ kind: "qty", value: 2000 }, taka)).toBe("2");
    expect(cellText({ kind: "qty", value: 1250 }, taka)).toBe("1.25");
    expect(cellText({ kind: "int", value: 7 }, taka)).toBe("7");
    expect(cellText({ kind: "percent", value: 1234 }, taka)).toBe("12.34%");
    expect(cellText({ kind: "date", value: "2026-03-02" }, taka)).toBe("2026-03-02");
  });

  it("marks numbers below zero, such as negative stock (INV-04)", () => {
    expect(isNegative({ kind: "qty", value: -1000 })).toBe(true);
    expect(isNegative({ kind: "money", value: 0 })).toBe(false);
    expect(isNegative({ kind: "text", value: "-1" })).toBe(false);
  });
});
