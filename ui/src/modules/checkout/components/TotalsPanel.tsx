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
    <div className={"flex justify-between text-base " + (purple ? "text-purple" : "text-text")}>
      <span className="text-text-muted">{label}</span>
      <span className="tabular-nums font-semibold">{value}</span>
    </div>
  );
  return (
    <aside className="flex flex-col gap-2.5 border-t border-border bg-panel p-4 shrink-0">
      <div className="space-y-1.5">
        {row(t("totals.items"), String(totals.item_count))}
        {row(t("totals.subtotal"), money(totals.subtotal))}
        {totals.discounts > 0
          ? row(t("totals.discounts"), `−${money(totals.discounts)}`, true)
          : null}
        {row(taxText, money(totals.tax))}
        {totals.rounding !== 0 ? row(t("totals.rounding"), money(totals.rounding)) : null}
      </div>

      <div
        aria-label={t("totals.total")}
        className="mt-1 flex flex-col rounded-[6px] border-2 border-text p-3 bg-bg"
      >
        <span className="text-sm font-bold uppercase tracking-wider text-text-muted">
          {t("totals.total")}
        </span>
        <span className="text-right text-[56px] leading-none font-extrabold tabular-nums text-text">
          {money(totals.total)}
        </span>
      </div>

      <div className="mt-2 w-full flex [&>button]:w-full [&>button]:min-h-[72px] [&>button]:text-xl [&>button]:font-bold [&>button]:justify-center">
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
