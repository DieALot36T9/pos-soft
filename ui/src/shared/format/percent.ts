import { formatMoney, parseMoney } from "./money";

// A tax rate in basis points shown as a percentage, without extra zeros.
// Example: 1500 → "15%", 750 → "7.5%", 1234 → "12.34%".
export function formatPercent(bps: number): string {
  const text = formatMoney(bps, {
    code: "",
    symbol: "",
    symbol_before: true,
    minor_digits: 2,
    decimal_sep: ".",
    group_sep: "",
    grouping: "western",
  });
  return `${text.replace(/\.?0+$/, "")}%`;
}

// Reads a typed percentage such as "7.5" into basis points (750). A percentage
// with two decimals is the same as an amount with two decimals, so the money reader is used.
export function parsePercent(text: string): { ok: true; bps: number } | { ok: false } {
  const parsed = parseMoney(text.replace("%", ""), 2);
  return parsed.ok && parsed.minor >= 0 ? { ok: true, bps: parsed.minor } : { ok: false };
}
