import { describe, expect, it } from "vitest";

import { isDay, presetRange } from "./dates";

describe("report date presets", () => {
  // 2026-03-04 is a Wednesday.
  const today = "2026-03-04";

  it("gives today, yesterday, this week from Monday and this month from the 1st", () => {
    expect(presetRange("today", today)).toEqual({ from: today, to: today });
    expect(presetRange("yesterday", today)).toEqual({ from: "2026-03-03", to: "2026-03-03" });
    expect(presetRange("this_week", today)).toEqual({ from: "2026-03-02", to: today });
    expect(presetRange("this_month", today)).toEqual({ from: "2026-03-01", to: today });
  });

  it("starts a week on Monday even on a Sunday, and crosses a month", () => {
    expect(presetRange("this_week", "2026-03-01")).toEqual({
      from: "2026-02-23",
      to: "2026-03-01",
    });
    expect(presetRange("yesterday", "2026-03-01").from).toBe("2026-02-28");
  });

  it("accepts only real days written as year-month-day", () => {
    expect(isDay("2026-02-28")).toBe(true);
    expect(isDay("2026-02-30")).toBe(false);
    expect(isDay("02/03/2026")).toBe(false);
    expect(isDay("")).toBe(false);
  });
});
