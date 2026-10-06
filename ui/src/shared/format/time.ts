// Whole seconds from now until a time in milliseconds since 1970, never below 0.
// Example: a lock ending in 59.2 seconds shows as 60.
export function secondsUntil(untilMs: number, nowMs: number = Date.now()): number {
  return Math.max(0, Math.ceil((untilMs - nowMs) / 1000));
}

// A moment as "2026-10-05 14:00" in this computer's time zone.
export function formatDateTime(atMs: number): string {
  const at = new Date(atMs);
  const two = (value: number) => String(value).padStart(2, "0");
  return (
    `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())} ` +
    `${two(at.getHours())}:${two(at.getMinutes())}`
  );
}
