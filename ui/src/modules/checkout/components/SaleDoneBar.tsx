import { CheckCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SaleDone } from "../../../generated/SaleDone";
import { formatMoney } from "../../../shared/format/money";

// After a sale: the invoice number and the CHANGE in big green numbers, until the next scan.
export function SaleDoneBar({ done, currency }: { done: SaleDone; currency: CurrencyFormat }) {
  const { t } = useTranslation("checkout");
  return (
    <div
      role="status"
      className="flex items-center justify-between gap-4 rounded-ui border border-green bg-green-soft p-3 text-green"
    >
      <span className="flex items-center gap-2 text-xl font-semibold">
        <CheckCircle aria-hidden="true" />
        {t("done.invoice", { number: done.invoice_no })}
        {done.points_earned > 0 ? (
          <span className="ml-2 text-purple">
            {t("done.points", { points: done.points_earned })}
          </span>
        ) : null}
      </span>
      {done.change > 0 ? (
        <span className="text-right">
          <span className="block text-xl font-semibold">{t("done.change")}</span>
          <span className="text-[56px] leading-tight font-semibold tabular-nums">
            {formatMoney(done.change, currency)}
          </span>
        </span>
      ) : (
        <span className="text-xl">{t("done.noChange")}</span>
      )}
    </div>
  );
}
