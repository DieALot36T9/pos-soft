import { AlertTriangle, CheckCircle, Clock, Package, TrendingDown } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { StockAlerts } from "../../../generated/StockAlerts";
import { errorMessage } from "../../../shared/bridge";
import { formatQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { stockAlerts } from "../api";

type StockAlertsCardProps = {
  onOpenStock: () => void;
};

// Low stock, near expiry and stock below zero, on the dashboard at login (INV-03, INV-04, INV-05).
export function StockAlertsCard({ onOpenStock }: StockAlertsCardProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [alerts, setAlerts] = useState<StockAlerts | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const loadOnce = () => {
    stockAlerts()
      .then(setAlerts)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the dashboard opens.
  useEffect(loadOnce, []);

  if (!alerts) {
    return problem ? <Alert kind="problem">{problem}</Alert> : null;
  }
  const nothing =
    alerts.low_stock_count === 0 && alerts.near_expiry_count === 0 && alerts.negative_count === 0;

  return (
    <section
      aria-labelledby="stock-alerts-heading"
      className="flex w-full max-w-xl flex-col gap-3 rounded-ui border border-border bg-panel p-6"
    >
      <h2 id="stock-alerts-heading" className="text-xl font-semibold">
        {t("alerts.title")}
      </h2>
      {nothing ? (
        <p className="flex items-center gap-2 text-green">
          <CheckCircle aria-hidden="true" size={20} />
          {t("alerts.none")}
        </p>
      ) : null}
      {alerts.low_stock_count > 0 ? (
        <div>
          <p className="flex items-center gap-2 font-semibold text-amber">
            <TrendingDown aria-hidden="true" size={20} />
            {t("alerts.low", { count: alerts.low_stock_count })}
          </p>
          <ul className="list-disc pl-8">
            {alerts.low_stock.map((row) => (
              <li key={row.product_id}>
                {t("alerts.lowRow", {
                  name: row.name,
                  qty: formatQty(row.on_hand, row.unit_decimals),
                  unit: row.unit_symbol,
                  level: formatQty(row.reorder_level, row.unit_decimals),
                })}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {alerts.near_expiry_count > 0 ? (
        <div>
          <p className="flex items-center gap-2 font-semibold text-amber">
            <Clock aria-hidden="true" size={20} />
            {t("alerts.expiring", {
              count: alerts.near_expiry_count,
              days: alerts.near_expiry_days,
            })}
          </p>
          <ul className="list-disc pl-8">
            {alerts.near_expiry.map((row) => (
              <li key={row.batch_id} className={row.expired ? "text-red" : undefined}>
                {t(row.expired ? "alerts.expiredRow" : "alerts.expiringRow", {
                  name: row.name,
                  qty: formatQty(row.on_hand, row.unit_decimals),
                  unit: row.unit_symbol,
                  date: row.expiry_date,
                })}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {alerts.negative_count > 0 ? (
        <p className="flex items-center gap-2 font-semibold text-red">
          <AlertTriangle aria-hidden="true" size={20} />
          {t("alerts.negative", { count: alerts.negative_count })}
        </p>
      ) : null}
      <div>
        <ActionButton
          colour="blue"
          icon={Package}
          label={t("alerts.open.label")}
          tip={t("alerts.open.tip")}
          onClick={onOpenStock}
        />
      </div>
    </section>
  );
}
