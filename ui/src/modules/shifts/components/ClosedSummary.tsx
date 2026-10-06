import { CheckCircle } from "lucide-react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ShiftClosed } from "../../../generated/ShiftClosed";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";

type ClosedSummaryProps = {
  result: ShiftClosed;
  currency: CurrencyFormat;
  onDone: () => void;
};

// What closing found (SHF-06): expected, counted and the difference. A shortage is red.
export function ClosedSummary({ result, currency, onDone }: ClosedSummaryProps) {
  const { t } = useTranslation("shifts");
  useHotkeys("enter,escape", onDone, { enableOnFormTags: true, preventDefault: true });
  const money = (minor: number) => formatMoney(minor, currency);
  const difference = result.difference;
  const word =
    difference === null
      ? t("closed.notCounted")
      : difference < 0
        ? t("closed.short")
        : difference > 0
          ? t("closed.over")
          : t("closed.exact");
  return (
    <Dialog title={t("closed.title")} colour="grey">
      <dl className="grid grid-cols-2 gap-2 text-xl">
        <dt>{t("closed.expected")}</dt>
        <dd className="text-right tabular-nums">{money(result.expected_cash)}</dd>
        <dt>{t("closed.counted")}</dt>
        <dd className="text-right tabular-nums">
          {result.counted_cash === null ? t("closed.notCounted") : money(result.counted_cash)}
        </dd>
        <dt>{t("closed.difference")}</dt>
        <dd
          className={
            "text-right font-semibold tabular-nums " +
            (difference !== null && difference < 0 ? "text-red" : "")
          }
        >
          {difference === null ? word : `${money(difference)} · ${word}`}
        </dd>
      </dl>
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={CheckCircle}
          label={t("closed.ok.label")}
          tip={t("closed.ok.tip")}
          shortcut="Enter"
          onClick={onDone}
        />
      </div>
    </Dialog>
  );
}
