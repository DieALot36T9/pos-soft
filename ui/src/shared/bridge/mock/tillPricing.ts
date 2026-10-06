import type { CartLineView } from "../../../generated/CartLineView";
import type { CartTotals } from "../../../generated/CartTotals";

// The fake till's totals: 15% VAT inside the prices, rounded to whole units for cash.
// Rust's pricing and tax modules do the real work; this only feeds the screens.
export function fakeTillPricing(lines: CartLineView[]): {
  totals: CartTotals;
  tax_label: string;
  tax_mode: "inclusive";
} {
  const gross = lines.reduce((sum, line) => sum + line.amount, 0);
  const tax = lines.reduce((sum, line) => sum + Math.round((line.amount * 1500) / 11500), 0);
  const total = Math.round(gross / 100) * 100;
  const items = lines.reduce(
    (sum, line) => sum + (line.unit_decimals === 0 ? line.qty / 1000 : 1),
    0,
  );
  return {
    totals: {
      item_count: items,
      subtotal: gross,
      discounts: 0,
      net: gross - tax,
      tax,
      gross,
      rounding: total - gross,
      total,
    },
    tax_label: "VAT",
    tax_mode: "inclusive",
  };
}
