import { Ban, CheckCircle, ChevronLeft } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { VarianceReport } from "../../../generated/VarianceReport";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { cancelCount, countVariances, postCount } from "../api";

type VarianceReviewProps = {
  countId: string;
  onBack: () => void;
};

// The variance report (expected, counted, difference, value) and Post or Cancel (INV-06).
// Rust works out every number; this screen only shows them.
export function VarianceReview({ countId, onBack }: VarianceReviewProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [report, setReport] = useState<VarianceReport | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    countVariances(countId)
      .then(setReport)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once for this count.
  useEffect(load, [countId]);

  const post = () => {
    postCount(countId, confirmed)
      .then((posted) => {
        setDone(t("review.posted", { rows: posted.rows_written, file: posted.backup_file }));
        setProblem(null);
        load();
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const cancel = () => {
    cancelCount(countId)
      .then(() => {
        setDone(t("review.cancelled"));
        setProblem(null);
        load();
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  if (!report) {
    return problem ? <Alert kind="problem">{problem}</Alert> : null;
  }
  const isOpen = report.count.status === "open";
  const needsConfirm = report.uncounted > 0 && !confirmed;

  return (
    <section aria-labelledby="review-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <h2 id="review-heading" className="text-[28px] font-semibold">
          {t("review.heading", { number: report.count.number, name: report.count.name })}
        </h2>
        <ActionButton
          colour="grey"
          icon={ChevronLeft}
          label={t("review.back.label")}
          tip={t("review.back.tip")}
          onClick={onBack}
        />
      </div>
      {done ? <Alert kind="done">{done}</Alert> : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {report.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("review.table.empty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("review.table.name")}</th>
              <th className="p-2">{t("review.table.batch")}</th>
              <th className="p-2 text-right">{t("review.table.expected")}</th>
              <th className="p-2 text-right">{t("review.table.counted")}</th>
              <th className="p-2 text-right">{t("review.table.difference")}</th>
              <th className="p-2 text-right">{t("review.table.value")}</th>
            </tr>
          </thead>
          <tbody>
            {report.rows.map((row) => {
              const qty = (milli: number) => formatQty(milli, row.unit_decimals);
              const colour =
                row.difference < 0 ? "text-red" : row.difference > 0 ? "text-green" : "";
              return (
                <tr
                  key={`${row.product_id}-${row.batch_id ?? ""}`}
                  className="border-b border-border"
                >
                  <td className="p-2">{row.name}</td>
                  <td className="p-2">{row.batch_no ?? ""}</td>
                  <td className="p-2 text-right tabular-nums">{qty(row.expected)}</td>
                  <td className="p-2 text-right tabular-nums">
                    {row.was_counted ? qty(row.counted) : t("review.table.notCounted")}
                  </td>
                  <td className={"p-2 text-right font-semibold tabular-nums " + colour}>
                    {(row.difference > 0 ? "+" : "") + qty(row.difference)}
                  </td>
                  <td className={"p-2 text-right tabular-nums " + colour}>
                    {formatMoney(row.cost_value, report.currency)}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
      <p className="text-xl font-semibold">
        {t("review.total", { value: formatMoney(report.total_value, report.currency) })}
      </p>
      {isOpen && report.uncounted > 0 ? (
        <Alert kind="warning">
          <p>{t("review.uncounted", { count: report.uncounted })}</p>
          <CheckBox
            label={t("review.confirm.label")}
            tip={t("review.confirm.tip")}
            checked={confirmed}
            onChange={setConfirmed}
          />
        </Alert>
      ) : null}
      {isOpen ? (
        <div className="flex gap-2">
          <ActionButton
            colour="green"
            icon={CheckCircle}
            label={t("review.post.label")}
            tip={t("review.post.tip")}
            disabledReason={needsConfirm ? t("review.confirm.tip") : undefined}
            onClick={post}
          />
          <ActionButton
            colour="red"
            icon={Ban}
            label={t("review.cancel.label")}
            tip={t("review.cancel.tip")}
            onClick={cancel}
          />
        </div>
      ) : null}
    </section>
  );
}
