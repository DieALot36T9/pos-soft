import { describe, expect, it } from "vitest";

import { formatDateTime, secondsUntil } from "./time";

describe("secondsUntil", () => {
  it("usr_02: rounds a lock countdown up to whole seconds", () => {
    expect(secondsUntil(60_000, 800)).toBe(60);
    expect(secondsUntil(60_000, 59_001)).toBe(1);
  });

  it("never counts below zero", () => {
    expect(secondsUntil(1_000, 5_000)).toBe(0);
  });
});

describe("formatDateTime", () => {
  it("shows year-month-day and hours:minutes", () => {
    expect(formatDateTime(new Date(2026, 9, 5, 14, 3).getTime())).toBe("2026-10-05 14:03");
  });
});
