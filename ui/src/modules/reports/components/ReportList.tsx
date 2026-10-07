import { useTranslation } from "react-i18next";

import { Tip } from "../../../shared/ui/Tip";
import { REPORT_LIST, type ReportKind } from "../request";

type ReportListProps = {
  selected: ReportKind;
  // A cashier without "all reports" sees only the shift reports (RPT-10).
  allowed: (kind: ReportKind) => boolean;
  onPick: (kind: ReportKind) => void;
};

// The list of reports on the left, grouped: Sales, Profit, Stock, Tax, Customers and
// suppliers, Control.
export function ReportList({ selected, allowed, onPick }: ReportListProps) {
  const { t } = useTranslation("reports");
  return (
    <nav aria-label={t("list.label")} className="flex w-64 shrink-0 flex-col gap-3">
      {REPORT_LIST.map(({ group, kinds }) => {
        const shown = kinds.filter(allowed);
        if (shown.length === 0) {
          return null;
        }
        return (
          <section key={group} className="flex flex-col gap-1">
            <h3 className="font-semibold text-text-muted">{t(`list.groups.${group}`)}</h3>
            {shown.map((kind) => (
              <Tip key={kind} tip={t(`reports.${kind}.tip`)}>
                <button
                  type="button"
                  aria-current={kind === selected ? "page" : undefined}
                  onClick={() => onPick(kind)}
                  className={
                    "min-h-10 rounded-ui border px-3 text-left " +
                    (kind === selected
                      ? "border-blue bg-blue-soft font-semibold text-blue"
                      : "border-border bg-panel")
                  }
                >
                  {t(`reports.${kind}.label`)}
                </button>
              </Tip>
            ))}
          </section>
        );
      })}
    </nav>
  );
}
