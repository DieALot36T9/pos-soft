import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { RateWithComponents } from "../../../generated/RateWithComponents";
import type { TaxMode } from "../../../generated/TaxMode";
import { useSave } from "../../../shared/bridge/useSave";
import { formatPercent } from "../../../shared/format/percent";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { listRates, saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

// Tax name, whether prices include tax, and the default rate (SET-07, TAX-01, TAX-04).
export function TaxSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const [label, setLabel] = useState(saved.tax.label);
  const [mode, setMode] = useState<TaxMode>(saved.tax.mode);
  const [defaultRate, setDefaultRate] = useState(saved.tax.default_rate_id ?? "");
  const [rates, setRates] = useState<RateWithComponents[]>([]);
  const save = useSave();

  useEffect(() => {
    listRates()
      .then(setRates)
      .catch(() => undefined);
  }, []);

  const submit = () =>
    save.run(
      () =>
        saveSettings({
          section: "tax",
          value: { label, mode, default_rate_id: defaultRate === "" ? null : defaultRate },
        }),
      onSaved,
    );

  return (
    <SectionFrame
      title={t("sections.tax")}
      busy={save.busy}
      done={save.done}
      problem={save.problem}
      onSave={submit}
    >
      <div className="grid grid-cols-2 gap-4">
        <Field
          label={t("tax.label.label")}
          tip={t("tax.label.tip")}
          value={label}
          onChange={setLabel}
          error={save.errorFor("label")}
        />
        <SelectField
          label={t("tax.mode.label")}
          tip={t("tax.mode.tip")}
          value={mode}
          options={(["inclusive", "exclusive"] as const).map((value) => ({
            value,
            label: t(`tax.mode.${value}`),
          }))}
          onChange={setMode}
        />
      </div>
      {mode !== saved.tax.mode ? <Alert kind="warning">{t("tax.modeWarning")}</Alert> : null}
      {rates.length > 0 ? (
        <SelectField
          label={t("tax.defaultRate.label")}
          tip={t("tax.defaultRate.tip")}
          value={defaultRate}
          options={rates.map((rate) => ({
            value: rate.id,
            label: `${rate.name} (${formatPercent(rate.rate_bps)})`,
          }))}
          onChange={setDefaultRate}
          error={save.errorFor("default_rate_id")}
        />
      ) : null}
      <p className="text-text-muted">{t("tax.ratesNote")}</p>
    </SectionFrame>
  );
}
