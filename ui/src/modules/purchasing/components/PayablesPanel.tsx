import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { AgingView } from "../../../generated/AgingView";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { Alert } from "../../../shared/ui/Alert";
import { loadAging } from "../api";

// What the shop owes each supplier and how late it is (PUR-05). All amounts from Rust.
export function PayablesPanel() {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const [aging, setAging] = useState<AgingView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    loadAging()
      .then(setAging)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, []);

  if (aging === null) {
    return problem ? <Alert kind="problem">{problem}</Alert> : <p>{t("common.loading")}</p>;
  }
  const money = (minor: number) => formatMoney(minor, aging.currency);
  return (
    <section className="flex flex-col gap-3">
      <p className="text-text-muted">{t("payables.today", { date: aging.today })}</p>
      {aging.rows.length === 0 ? <p>{t("payables.empty")}</p> : null}
      {aging.rows.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("payables.supplier")}</th>
              <th className="p-2 text-right">{t("payables.current")}</th>
              <th className="p-2 text-right">{t("payables.days30")}</th>
              <th className="p-2 text-right">{t("payables.days60")}</th>
              <th className="p-2 text-right">{t("payables.days90")}</th>
              <th className="p-2 text-right">{t("payables.over90")}</th>
              <th className="p-2 text-right">{t("payables.balance")}</th>
            </tr>
          </thead>
          <tbody>
            {aging.rows.map((row) => (
              <tr key={row.supplier_id} className="border-b border-border">
                <td className="p-2">{row.name}</td>
                <td className="p-2 text-right tabular-nums">{money(row.current)}</td>
                <td className="p-2 text-right tabular-nums">{money(row.days_1_30)}</td>
                <td className="p-2 text-right tabular-nums">{money(row.days_31_60)}</td>
                <td className="p-2 text-right tabular-nums text-amber">{money(row.days_61_90)}</td>
                <td className="p-2 text-right tabular-nums text-red">{money(row.days_over_90)}</td>
                <td className="p-2 text-right font-semibold tabular-nums">{money(row.balance)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </section>
  );
}
