import { Star } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ReturnPreview } from "../../../generated/ReturnPreview";
import { Badge } from "../../../shared/ui/Badge";

type RefundSummaryProps = {
  preview: ReturnPreview;
  money: (minor: number) => string;
};

// What goes back and how: the refund total in big red numbers, then each part (debt,
// points, the chosen method), the rounding and the points taken back. All from Rust.
export function RefundSummary({ preview, money }: RefundSummaryProps) {
  const { t } = useTranslation("checkout");
  return (
    <div className="flex flex-col gap-1 border-y border-border py-3" aria-live="polite">
      <div className="flex justify-between text-2xl font-semibold text-red">
        <span>{t("returns.refund")}</span>
        <span className="tabular-nums">{money(preview.total)}</span>
      </div>
      {preview.refunds.map((refund) => (
        <div key={refund.method_id} className="flex justify-between text-lg">
          <span>{refund.name}</span>
          <span className="tabular-nums">{money(refund.amount)}</span>
        </div>
      ))}
      {preview.rounding !== 0 ? (
        <p className="text-right text-text-muted">
          {t("returns.rounding")} {money(preview.rounding)}
        </p>
      ) : null}
      {preview.points_taken_back > 0 ? (
        <div>
          <Badge
            colour="purple"
            icon={Star}
            word={t("returns.pointsBack", { points: preview.points_taken_back })}
            tip={t("returns.pointsBackTip")}
          />
        </div>
      ) : null}
    </div>
  );
}
