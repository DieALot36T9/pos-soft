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

const SUBTITLE =
  "Automatic inventory health check for low stock, near-expiry batches, and negative levels.";

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
      className="flex w-full flex-col gap-4 rounded-ui border border-border bg-panel p-5"
    >
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-3">
        <div>
          <h2 id="stock-alerts-heading" className="text-xl font-bold text-text">
            {t("alerts.title")}
          </h2>
          <p className="text-xs text-text-muted mt-0.5">{SUBTITLE}</p>
        </div>
        <ActionButton
          colour="blue"
          icon={Package}
          label={t("alerts.open.label")}
          tip={t("alerts.open.tip")}
          onClick={onOpenStock}
        />
      </div>

      {nothing ? (
        <div className="rounded-ui border border-green bg-green-soft p-3 flex items-center gap-2 font-semibold text-green">
          <CheckCircle aria-hidden="true" size={20} />
          <span>{t("alerts.none")}</span>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {alerts.low_stock_count > 0 ? (
            <div className="rounded-ui border border-amber bg-amber-soft/50 p-3.5 flex flex-col gap-2">
              <p className="flex items-center gap-2 font-bold text-amber">
                <TrendingDown aria-hidden="true" size={18} />
                {t("alerts.low", { count: alerts.low_stock_count })}
              </p>
              <ul className="list-disc pl-6 text-sm space-y-1">
                {alerts.low_stock.map((row) => (
                  <li key={row.product_id} className="tabular-nums">
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
            <div className="rounded-ui border border-amber bg-amber-soft/50 p-3.5 flex flex-col gap-2">
              <p className="flex items-center gap-2 font-bold text-amber">
                <Clock aria-hidden="true" size={18} />
                {t("alerts.expiring", {
                  count: alerts.near_expiry_count,
                  days: alerts.near_expiry_days,
                })}
              </p>
              <ul className="list-disc pl-6 text-sm space-y-1">
                {alerts.near_expiry.map((row) => (
                  <li
                    key={row.batch_id}
                    className={
                      "tabular-nums " + (row.expired ? "font-semibold text-red" : "text-text")
                    }
                  >
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
            <div className="rounded-ui border border-red bg-red-soft p-3.5 flex items-start gap-2 font-bold text-red">
              <AlertTriangle aria-hidden="true" size={18} className="mt-0.5 shrink-0" />
              <span>{t("alerts.negative", { count: alerts.negative_count })}</span>
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}
