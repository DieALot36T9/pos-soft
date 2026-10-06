import { RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { formatMoney } from "../../../shared/format/money";

// What a return, a void or an exchange cash-out says afterwards.
export type RefundDone = {
  // "Return T1-R000001 saved", "Sale T1-000012 voided", …
  title: string;
  cashBack: number;
  pointsTakenBack: number;
};

// After money went back: what was done and the cash to hand over, in big red numbers.
export function RefundDoneBar({ done, currency }: { done: RefundDone; currency: CurrencyFormat }) {
  const { t } = useTranslation("checkout");
  return (
    <div
      role="status"
      className="flex items-center justify-between gap-4 rounded-ui border border-red bg-red-soft p-3 text-red"
    >
      <span className="flex items-center gap-2 text-xl font-semibold">
        <RotateCcw aria-hidden="true" />
        {done.title}
        {done.pointsTakenBack > 0 ? (
          <span className="ml-2 text-purple">
            {t("refundDone.points", { points: done.pointsTakenBack })}
          </span>
        ) : null}
      </span>
      {done.cashBack > 0 ? (
        <span className="text-right">
          <span className="block text-xl font-semibold">{t("refundDone.giveBack")}</span>
          <span className="text-[56px] leading-tight font-semibold tabular-nums">
            {formatMoney(done.cashBack, currency)}
          </span>
        </span>
      ) : (
        <span className="text-xl">{t("refundDone.nothing")}</span>
      )}
    </div>
  );
}
