import { Ban, CheckCircle, Pencil, PauseCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { RateWithComponents } from "../../../generated/RateWithComponents";
import { errorMessage } from "../../../shared/bridge";
import { formatPercent } from "../../../shared/format/percent";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { IconButton } from "../../../shared/ui/IconButton";
import { deactivateRate, listRates } from "../api";
import { RateForm } from "../components/RateForm";

export function TaxRatesPage() {
  const { t } = useTranslation("tax");
  const { t: tErrors } = useTranslation("errors");
  const [rates, setRates] = useState<RateWithComponents[]>([]);
  const [showInactive, setShowInactive] = useState(false);
  const [editing, setEditing] = useState<RateWithComponents | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    listRates(showInactive)
      .then(setRates)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload only when the filter changes.
  useEffect(load, [showInactive]);

  const deactivate = (id: string) => {
    deactivateRate(id)
      .then(load)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <section aria-labelledby="tax-heading" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 id="tax-heading" className="text-[28px] font-semibold">
            {t("page.title")}
          </h2>
          <p className="text-text-muted">{t("page.intro")}</p>
        </div>
        <CheckBox
          label={t("showInactive.label")}
          tip={t("showInactive.tip")}
          checked={showInactive}
          onChange={setShowInactive}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {rates.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("table.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("table.name")}</th>
              <th className="p-2.5 text-right">{t("table.percent")}</th>
              <th className="p-2.5">{t("table.status")}</th>
              <th className="p-2.5">{t("table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {rates.map((rate) => (
              <tr key={rate.id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="p-2.5 font-semibold">{rate.name}</td>
                <td className="p-2.5 text-right font-semibold tabular-nums">
                  {formatPercent(rate.rate_bps)}
                </td>
                <td className="p-2.5">
                  {rate.active ? (
                    <Badge
                      colour="green"
                      icon={CheckCircle}
                      word={t("table.active")}
                      tip={t("table.activeTip")}
                    />
                  ) : (
                    <Badge
                      colour="amber"
                      icon={PauseCircle}
                      word={t("table.inactive")}
                      tip={t("table.inactiveTip")}
                    />
                  )}
                </td>
                <td className="flex gap-2 p-2.5">
                  <IconButton
                    icon={Pencil}
                    colour="purple"
                    tip={t("edit.tip")}
                    onClick={() => setEditing(rate)}
                  />
                  {rate.active ? (
                    <IconButton
                      icon={Ban}
                      colour="red"
                      tip={t("deactivate.tip")}
                      onClick={() => deactivate(rate.id)}
                    />
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <RateForm
        key={editing?.id ?? "new"}
        rate={editing}
        onSaved={() => {
          setEditing(null);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </section>
  );
}
