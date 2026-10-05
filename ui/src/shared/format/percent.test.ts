import { describe, expect, it } from "vitest";

import { formatPercent, parsePercent } from "./percent";

describe("percentages", () => {
  it("tax_02: shows basis points as a short percentage", () => {
    expect(formatPercent(1500)).toBe("15%");
    expect(formatPercent(750)).toBe("7.5%");
    expect(formatPercent(1234)).toBe("12.34%");
    expect(formatPercent(0)).toBe("0%");
  });

  it("tax_02: reads a typed percentage with up to 2 decimals", () => {
    expect(parsePercent("15")).toEqual({ ok: true, bps: 1500 });
    expect(parsePercent("7.5%")).toEqual({ ok: true, bps: 750 });
    expect(parsePercent("1.234")).toEqual({ ok: false });
    expect(parsePercent("-1")).toEqual({ ok: false });
  });
});
