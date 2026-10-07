// The date presets on the Reports screen (Back Office Screens: today, yesterday, this
// week, this month, custom). Dates are "YYYY-MM-DD" business days; weeks start on Monday.
// This picks days only. Every number in a report comes from Rust.

export type Preset = "today" | "yesterday" | "this_week" | "this_month" | "custom";
export const PRESETS: Preset[] = ["today", "yesterday", "this_week", "this_month", "custom"];

// True for a real day written as year-month-day. Example: "2026-03-02".
export function isDay(text: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) {
    return false;
  }
  return toText(fromText(text)) === text;
}

// The first and last day of a preset, counted from today. Custom keeps today to today.
export function presetRange(preset: Preset, today: string): { from: string; to: string } {
  const day = fromText(today);
  switch (preset) {
    case "yesterday": {
      const yesterday = addDays(day, -1);
      return { from: toText(yesterday), to: toText(yesterday) };
    }
    case "this_week": {
      // getUTCDay: Sunday 0 … Saturday 6. Monday is the first day of the week.
      const sinceMonday = (day.getUTCDay() + 6) % 7;
      return { from: toText(addDays(day, -sinceMonday)), to: today };
    }
    case "this_month":
      return { from: `${today.slice(0, 8)}01`, to: today };
    case "today":
    case "custom":
      return { from: today, to: today };
  }
}

function fromText(text: string): Date {
  return new Date(`${text}T00:00:00Z`);
}

function toText(day: Date): string {
  return day.toISOString().slice(0, 10);
}

function addDays(day: Date, days: number): Date {
  return new Date(day.getTime() + days * 86_400_000);
}
