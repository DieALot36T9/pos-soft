import { formatMoney, parseMoney } from "./money";

// Quantities are thousandths of a unit (Qty): 1000 is 1 piece, 1 kg or 1 litre.
const THOUSANDTHS = 3;

// Shows a quantity with the unit's number of decimals. Example: (1250, 3) → "1.250", (2000, 0) → "2".
export function formatQty(milli: number, decimals: number): string {
  const shown = Math.min(Math.max(decimals, 0), THOUSANDTHS);
  const scaled = Math.trunc(milli / 10 ** (THOUSANDTHS - shown));
  return formatMoney(scaled, {
    code: "",
    symbol: "",
    symbol_before: true,
    minor_digits: shown,
    decimal_sep: ".",
    group_sep: "",
    grouping: "western",
  });
}

// Reads a typed quantity such as "1.5" into thousandths (1500), with at most the
// unit's decimals. Below zero is refused. The error codes match the money reader's.
export function parseQty(
  text: string,
  decimals: number,
): { ok: true; milli: number } | { ok: false; code: string } {
  const allowed = Math.min(Math.max(decimals, 0), THOUSANDTHS);
  const parsed = parseMoney(text, allowed);
  if (!parsed.ok) {
    return { ok: false, code: parsed.code === "money_empty" ? "qty_empty" : "qty_invalid" };
  }
  if (parsed.minor < 0) {
    return { ok: false, code: "qty_invalid" };
  }
  return { ok: true, milli: parsed.minor * 10 ** (THOUSANDTHS - allowed) };
}
