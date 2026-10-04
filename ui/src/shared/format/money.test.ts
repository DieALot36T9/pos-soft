// The same shared examples that core::format_money and core::parse_money pass.
import { describe, expect, it } from "vitest";

import vectors from "../../../../crates/core/tests/money_vectors.json";
import type { CurrencyFormat } from "../../generated/CurrencyFormat";
import { formatMoney, parseMoney } from "./money";

describe("formatMoney (rule SET-05)", () => {
  it.each(vectors.format)("$about", (example) => {
    expect(formatMoney(example.minor, example.format as CurrencyFormat)).toBe(example.text);
  });
});

describe("parseMoney", () => {
  it.each(vectors.parse)("reads $text", (example) => {
    const result = parseMoney(example.text, example.minor_digits);
    if ("error" in example) {
      expect(result).toEqual({ ok: false, code: example.error });
    } else {
      expect(result).toEqual({ ok: true, minor: example.minor });
    }
  });
});
