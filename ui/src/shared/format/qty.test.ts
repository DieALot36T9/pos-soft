import { describe, expect, it } from "vitest";

import { formatQty, parseQty } from "./qty";

describe("quantities", () => {
  it("shows the unit's decimals", () => {
    expect(formatQty(1250, 3)).toBe("1.250");
    expect(formatQty(2000, 0)).toBe("2");
    expect(formatQty(24000, 0)).toBe("24");
  });

  it("reads typed quantities into thousandths", () => {
    expect(parseQty("1.5", 3)).toEqual({ ok: true, milli: 1500 });
    expect(parseQty("12", 0)).toEqual({ ok: true, milli: 12000 });
  });

  it("refuses too many decimals, words and amounts below zero", () => {
    expect(parseQty("1.5", 0)).toEqual({ ok: false, code: "qty_invalid" });
    expect(parseQty("abc", 3)).toEqual({ ok: false, code: "qty_invalid" });
    expect(parseQty("-1", 3)).toEqual({ ok: false, code: "qty_invalid" });
    expect(parseQty(" ", 3)).toEqual({ ok: false, code: "qty_empty" });
  });
});
