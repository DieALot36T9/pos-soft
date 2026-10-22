import { memo } from "react";
import {
  AlertTriangle,
  PenLine,
  RefreshCw,
  Scale,
  ShoppingBag,
  SlidersHorizontal,
  Tag,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CartLineView } from "../../../generated/CartLineView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { formatMoney } from "../../../shared/format/money";
import { formatQty } from "../../../shared/format/qty";
import { cn } from "../../../shared/lib/utils";
import { Badge } from "../../../shared/ui/Badge";
import { Tip } from "../../../shared/ui/Tip";

type CartTableProps = {
  lines: CartLineView[];
  currency: CurrencyFormat;
  selected: string | null;
  // The line just added: its background flashes for 0.3 s.
  flash: string | null;
  // Lines whose price changed while the sale was on hold (CHK-10).
  priceChanged: string[];
  onSelect: (lineId: string) => void;
  onInspectLine?: (lineId: string) => void;
};

// The cart lines: product, quantity, price and amount, all as Rust sent them.
// Blue is the selected line, purple a discount, amber a stock warning (Checkout Screen).
export const CartTable = memo(function CartTable({
  lines,
  currency,
  selected,
  flash,
  priceChanged,
  onSelect,
  onInspectLine,
}: CartTableProps) {
  const { t } = useTranslation("checkout");
  const money = (minor: number) => formatMoney(minor, currency);
  const numpadLabel = "Numpad";
  if (lines.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6 text-center text-text-muted">
        <ShoppingBag aria-hidden="true" className="w-10 h-10 mb-2 opacity-50" />
        <p className="text-xl font-medium">{t("table.empty")}</p>
      </div>
    );
  }
  return (
    <table className="w-full text-xl border-collapse">
      <thead className="border-b border-border bg-bg text-left text-sm font-bold text-text-muted">
        <tr>
          <th className="w-10 p-2.5">{t("table.number")}</th>
          <th className="p-2.5">{t("table.product")}</th>
          <th className="p-2.5 text-right">{t("table.qty")}</th>
          <th className="p-2.5 text-right">{t("table.price")}</th>
          <th className="p-2.5 text-right">{t("table.amount")}</th>
        </tr>
      </thead>
      <tbody>
        {lines.map((line, index) => {
          const isSelected = line.line_id === selected;
          return (
            <tr
              key={line.line_id}
              aria-selected={isSelected}
              onClick={() => onSelect(line.line_id)}
              onDoubleClick={() => {
                onSelect(line.line_id);
                onInspectLine?.(line.line_id);
              }}
              className={cn(
                "cursor-pointer border-b border-border align-top hover:bg-blue-soft/40",
                isSelected && "bg-blue-soft text-blue",
                line.line_id === flash && "bg-green-soft",
                line.stock_warning !== null && !isSelected && "border-l-4 border-l-amber",
              )}
            >
              <td className="p-2.5 text-base font-medium tabular-nums">{index + 1}</td>
              <td className="p-2.5">
                <div className="flex items-center justify-between gap-2">
                  <Tip tip={t("table.selectTip")}>
                    <span className="font-semibold text-base">{line.name}</span>
                  </Tip>
                  {isSelected && onInspectLine ? (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onInspectLine(line.line_id);
                      }}
                      title="Open Numpad Line Editor for Quantity, Discount, or Price"
                      className="inline-flex items-center gap-1 rounded-ui border border-blue bg-panel px-2 py-0.5 text-xs font-bold text-blue cursor-pointer"
                    >
                      <SlidersHorizontal aria-hidden="true" className="w-3 h-3" />
                      <span>{numpadLabel}</span>
                    </button>
                  ) : null}
                </div>
                <span className="mt-1 inline-flex flex-wrap gap-1 align-middle">
                  {line.is_weighed ? (
                    <Badge
                      colour="blue"
                      icon={Scale}
                      word={line.unit_symbol}
                      tip={t("table.weighed")}
                    />
                  ) : null}
                  {line.sticker_price ? (
                    <Badge
                      colour="purple"
                      icon={Tag}
                      word={t("table.stickerWord")}
                      tip={t("table.sticker")}
                    />
                  ) : null}
                  {line.price_overridden ? (
                    <Badge
                      colour="purple"
                      icon={PenLine}
                      word={t("table.priceTypedWord")}
                      tip={t("table.priceTyped")}
                    />
                  ) : null}
                  {priceChanged.includes(line.line_id) ? (
                    <Badge
                      colour="amber"
                      icon={RefreshCw}
                      word={t("table.priceChanged")}
                      tip={t("table.priceChangedTip")}
                    />
                  ) : null}
                  {line.stock_warning === "not_enough_stock" ? (
                    <Badge
                      colour="amber"
                      icon={AlertTriangle}
                      word={t("table.notEnoughStock")}
                      tip={t("table.notEnoughStockTip")}
                    />
                  ) : null}
                  {line.stock_warning === "expired_batch" ? (
                    <Badge
                      colour="amber"
                      icon={AlertTriangle}
                      word={t("table.expiredBatch")}
                      tip={t("table.expiredBatchTip")}
                    />
                  ) : null}
                </span>
                {line.promotion_name !== null || line.discount > 0 ? (
                  <div className="flex justify-between text-sm font-semibold text-purple mt-1">
                    <span>{line.promotion_name ?? t("totals.discounts")}</span>
                    <span className="tabular-nums">
                      {t("table.discountAmount", { amount: money(line.discount) })}
                    </span>
                  </div>
                ) : null}
              </td>
              <td className="p-2.5 text-right text-base font-semibold tabular-nums">
                {formatQty(line.qty, line.unit_decimals)}
              </td>
              <td className="p-2.5 text-right text-base tabular-nums">{money(line.unit_price)}</td>
              <td className="p-2.5 text-right text-base font-bold tabular-nums">
                {money(line.amount)}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
});
