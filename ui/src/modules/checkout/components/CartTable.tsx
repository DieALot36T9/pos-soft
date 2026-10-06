import { AlertTriangle, PenLine, RefreshCw, Scale, Tag } from "lucide-react";
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
};

// The cart lines: product, quantity, price and amount, all as Rust sent them.
// Blue is the selected line, purple a discount, amber a stock warning (Checkout Screen).
export function CartTable({
  lines,
  currency,
  selected,
  flash,
  priceChanged,
  onSelect,
}: CartTableProps) {
  const { t } = useTranslation("checkout");
  const money = (minor: number) => formatMoney(minor, currency);
  if (lines.length === 0) {
    return <p className="p-6 text-xl text-text-muted">{t("table.empty")}</p>;
  }
  return (
    <table className="w-full text-xl">
      <thead className="border-b border-border text-left text-base text-text-muted">
        <tr>
          <th className="w-12 p-2">{t("table.number")}</th>
          <th className="p-2">{t("table.product")}</th>
          <th className="p-2 text-right">{t("table.qty")}</th>
          <th className="p-2 text-right">{t("table.price")}</th>
          <th className="p-2 text-right">{t("table.amount")}</th>
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
              className={cn(
                "cursor-pointer border-b border-border align-top",
                isSelected && "bg-blue-soft text-blue",
                line.line_id === flash && "bg-green-soft",
                line.stock_warning !== null && !isSelected && "border-l-4 border-l-amber",
              )}
            >
              <td className="p-2 tabular-nums">{index + 1}</td>
              <td className="p-2">
                <Tip tip={t("table.selectTip")}>
                  <span className="font-semibold">{line.name}</span>
                </Tip>
                <span className="ml-2 inline-flex flex-wrap gap-1 align-middle">
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
                  <div className="flex justify-between text-base text-purple">
                    <span>{line.promotion_name ?? t("totals.discounts")}</span>
                    <span className="tabular-nums">
                      {t("table.discountAmount", { amount: money(line.discount) })}
                    </span>
                  </div>
                ) : null}
              </td>
              <td className="p-2 text-right tabular-nums">
                {formatQty(line.qty, line.unit_decimals)}
              </td>
              <td className="p-2 text-right tabular-nums">{money(line.unit_price)}</td>
              <td className="p-2 text-right font-semibold tabular-nums">{money(line.amount)}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
