import type { CurrencyFormat } from "../../generated/CurrencyFormat";
import type { Money } from "../../generated/Money";

// Shows an amount the way the shop writes money. Example: 123456750 → "৳12,34,567.50".
// This must give exactly the same text as core::format_money in Rust. Both are
// tested against crates/core/tests/money_vectors.json.
export function formatMoney(minor: Money, format: CurrencyFormat): string {
  const digits = Math.min(format.minor_digits, 18);
  const size = Math.abs(minor);
  const divisor = 10 ** digits;
  const whole = Math.trunc(size / divisor);
  const fraction = size % divisor;

  let number = groupDigits(String(whole), format.group_sep, format.grouping);
  if (digits > 0) {
    number += format.decimal_sep + String(fraction).padStart(digits, "0");
  }

  let withSymbol = number;
  if (format.symbol !== "") {
    withSymbol = format.symbol_before ? format.symbol + number : `${number} ${format.symbol}`;
  }
  return minor < 0 ? `-${withSymbol}` : withSymbol;
}

// Puts group separators into a string of digits, working from the right.
function groupDigits(digits: string, separator: string, grouping: CurrencyFormat["grouping"]) {
  if (separator === "") {
    return digits;
  }
  const groups: string[] = [];
  let end = digits.length;
  let groupSize = 3;
  while (end > 0) {
    const start = Math.max(0, end - groupSize);
    groups.unshift(digits.slice(start, end));
    end = start;
    // After the first group of three, Indian grouping uses groups of two.
    if (grouping === "indian") {
      groupSize = 2;
    }
  }
  return groups.join(separator);
}

// The result of reading a typed amount: the amount, or the error code to show.
export type ParsedMoney = { ok: true; minor: Money } | { ok: false; code: string };

// Reads a typed amount such as "123.45" into minor units, with the same rules as
// core::parse_money: digits, an optional minus and at most `minorDigits` decimals.
export function parseMoney(text: string, minorDigits: number): ParsedMoney {
  const trimmed = text.trim();
  if (trimmed === "") {
    return { ok: false, code: "money_empty" };
  }
  const match = /^(-?)(\d+)(?:\.(\d+))?$/.exec(trimmed);
  if (!match) {
    return { ok: false, code: "money_not_a_number" };
  }
  const [, sign = "", whole = "", fraction = ""] = match;
  if (fraction.length > minorDigits) {
    return { ok: false, code: "money_too_many_decimals" };
  }
  const digits = whole + fraction.padEnd(minorDigits, "0");
  const size = Number(digits);
  if (!Number.isSafeInteger(size)) {
    return { ok: false, code: "money_too_large" };
  }
  return { ok: true, minor: sign === "-" ? -size : size };
}
