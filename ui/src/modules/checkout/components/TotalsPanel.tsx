import { CheckCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CartTotals } from "../../../generated/CartTotals";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { TaxMode } from "../../../generated/TaxMode";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";

type TotalsPanelProps = {
  totals: CartTotals;
  taxLabel: string;
  taxMode: TaxMode;
  currency: CurrencyFormat;
  // No items yet: Pay is disabled and says why.
  empty: boolean;
  onPay: () => void;
};

// The totals on the right (CHK-18): every figure comes from Rust. TOTAL is the
// biggest text on the screen; the green Pay button sits bottom-right.
export function TotalsPanel({
  totals,
  taxLabel,
  taxMode,
  currency,
  empty,
  onPay,
}: TotalsPanelProps) {
  const { t } = useTranslation("checkout");
  const money = (minor: number) => formatMoney(minor, currency);
  const taxText =
    taxMode === "inclusive"
      ? t("totals.taxIncluded", { label: taxLabel })
      : t("totals.taxAdded", { label: taxLabel });
  const row = (label: string, value: string, purple = false) => (
    <div className={"flex justify-between text-xl " + (purple ? "text-purple" : "")}>
      <span>{label}</span>
      <span className="tabular-nums">{value}</span>
    </div>
  );
  return (
    <aside className="flex w-96 shrink-0 flex-col gap-2 border-l border-border bg-panel p-4">
      {row(t("totals.items"), String(totals.item_count))}
      {row(t("totals.subtotal"), money(totals.subtotal))}
      {totals.discounts > 0
        ? row(t("totals.discounts"), `−${money(totals.discounts)}`, true)
        : null}
      {row(taxText, money(totals.tax))}
      {totals.rounding !== 0 ? row(t("totals.rounding"), money(totals.rounding)) : null}
      <div
        aria-label={t("totals.total")}
        className="mt-2 flex flex-col rounded-ui border-2 border-text p-3"
      >
        <span className="text-xl font-semibold">{t("totals.total")}</span>
        <span className="text-right text-[56px] leading-tight font-semibold tabular-nums">
          {money(totals.total)}
        </span>
      </div>
      <div className="mt-auto flex justify-end">
        <ActionButton
          colour="green"
          icon={CheckCircle}
          label={t("pay.label")}
          tip={t("pay.tip")}
          shortcut={shortcuts.checkoutPay.label}
          size="till"
          disabledReason={empty ? t("pay.empty") : undefined}
          onClick={onPay}
        />
      </div>
    </aside>
  );
}
