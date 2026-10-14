import { Clock, HelpCircle, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ShiftView } from "../../../generated/ShiftView";
import { formatDateTime } from "../../../shared/format/time";

type TillHeaderProps = {
  shift: ShiftView | null;
  onHelp?: () => void;
};

// The strip at the top of the till: who has the shift, business date, and F1 Help button.
export function TillHeader({ shift, onHelp }: TillHeaderProps) {
  const { t } = useTranslation("checkout");
  const since = shift ? formatDateTime(shift.opened_at).slice(11) : "";
  const businessDay = shift ? `Business Day: ${shift.business_date}` : "";
  const helpLabel = "Help";
  const f1Key = "F1";
  return (
    <header className="flex h-12 items-center justify-between border-b border-border bg-panel px-4 shrink-0">
      <div className="flex items-center gap-3">
        <span
          className={
            "inline-flex items-center gap-1.5 rounded-ui border px-2.5 py-0.5 text-xs font-bold " +
            (shift
              ? "border-green bg-green-soft text-green"
              : "border-amber bg-amber-soft text-amber")
          }
        >
          <ShieldCheck aria-hidden="true" className="w-3.5 h-3.5" />
          <span>{t("header.till")}</span>
        </span>
        <span className="font-semibold text-sm text-text">
          {shift
            ? t("header.shift", { name: shift.cashier_name, time: since })
            : t("header.noShift")}
        </span>
      </div>
      <div className="flex items-center gap-3">
        {shift ? (
          <div className="flex items-center gap-1.5 text-xs font-medium text-text-muted tabular-nums">
            <Clock aria-hidden="true" className="w-3.5 h-3.5 text-blue" />
            <span>{businessDay}</span>
          </div>
        ) : null}
        {onHelp ? (
          <button
            type="button"
            onClick={onHelp}
            title="Keyboard shortcuts and colour guide (F1)"
            className="inline-flex items-center gap-1.5 rounded-ui border border-border bg-bg hover:bg-grey-soft px-2.5 py-1 text-xs font-semibold text-text cursor-pointer"
          >
            <HelpCircle aria-hidden="true" className="w-3.5 h-3.5 text-blue" />
            <span>{helpLabel}</span>
            <kbd className="rounded border border-border bg-panel px-1 py-0.2 font-mono text-[10px]">
              {f1Key}
            </kbd>
          </button>
        ) : null}
      </div>
    </header>
  );
}
