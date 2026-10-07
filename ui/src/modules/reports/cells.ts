import type { Cell } from "../../generated/Cell";
import type { ColumnKind } from "../../generated/ColumnKind";
import type { CurrencyFormat } from "../../generated/CurrencyFormat";
import { formatMoney } from "../../shared/format/money";
import { formatPercent } from "../../shared/format/percent";
import { formatQty } from "../../shared/format/qty";

// One report cell as text. Quantities show without extra zeros: 2000 → "2", 1250 → "1.25".
// Money uses the shop's format; percentages come as basis points (1234 → "12.34%").
export function cellText(cell: Cell, currency: CurrencyFormat): string {
  switch (cell.kind) {
    case "empty":
      return "";
    case "text":
      return cell.value;
    case "money":
      return formatMoney(cell.value, currency);
    case "qty":
      return formatQty(cell.value, 3).replace(/\.?0+$/, "");
    case "int":
      return String(cell.value);
    case "percent":
      return formatPercent(cell.value);
    case "date":
      return cell.value;
  }
}

// True when the cell is a number below zero. Negative stock shows in red (INV-04).
export function isNegative(cell: Cell): boolean {
  return (cell.kind === "money" || cell.kind === "qty" || cell.kind === "int") && cell.value < 0;
}

// Numbers line up on the right (Design System); text and dates on the left.
export function alignRight(kind: ColumnKind): boolean {
  return kind === "money" || kind === "qty" || kind === "int" || kind === "percent";
}
