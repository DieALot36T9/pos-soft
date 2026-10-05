import { Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { RateWithComponents } from "../../../generated/RateWithComponents";
import type { TaxMode } from "../../../generated/TaxMode";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatPercent, parsePercent } from "../../../shared/format/percent";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { addRate, listRates, saveTax } from "../api";
import { StepFrame } from "./StepFrame";
import type { StepProps } from "./types";

// Step 3: tax name, whether shelf prices include tax, the rates and the default (SET-07, TAX-01 to TAX-04).
export function TaxStep({ saved, onSaved, onBack }: StepProps) {
  const { t } = useTranslation("setup");
  const { t: tErrors } = useTranslation("errors");
  const [label, setLabel] = useState(saved.tax.label);
  const [mode, setMode] = useState<TaxMode>(saved.tax.mode);
  const [rates, setRates] = useState<RateWithComponents[]>([]);
  const [defaultRate, setDefaultRate] = useState(saved.tax.default_rate_id ?? "");
  const [rateName, setRateName] = useState("");
  const [percent, setPercent] = useState("");
  const [rateProblem, setRateProblem] = useState<string | null>(null);
  const save = useSave();

  const loadRates = () => {
    listRates()
      .then(setRates)
      .catch(() => undefined);
  };
  useEffect(loadRates, []);

  const add = () => {
    const parsed = parsePercent(percent);
    if (!parsed.ok) {
      setRateProblem(t("tax.percentInvalid"));
      return;
    }
    setRateProblem(null);
    addRate({
      id: null,
      rate: { name: rateName, rate_bps: parsed.bps, components: [], sort: rates.length },
    })
      .then(() => {
        setRateName("");
        setPercent("");
        loadRates();
      })
      .catch((error: unknown) => setRateProblem(errorMessage(error, tErrors)));
  };

  const next = () =>
    save.run(
      () => saveTax({ label, mode, default_rate_id: defaultRate === "" ? null : defaultRate }),
      onSaved,
    );

  return (
    <StepFrame
      title={t("tax.title")}
      intro={t("tax.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={next}
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
      <p className="rounded-ui border border-border bg-bg p-3">{t(`tax.example.${mode}`)}</p>
      <h3 className="text-xl font-semibold">{t("tax.ratesHeading")}</h3>
      {rates.length === 0 ? (
        <p className="text-text-muted">{t("tax.noRates")}</p>
      ) : (
        <ul className="flex flex-col gap-1" aria-label={t("tax.ratesHeading")}>
          {rates.map((rate) => (
            <li
              key={rate.id}
              className="flex justify-between rounded-ui border border-border px-3 py-2"
            >
              <span>{rate.name}</span>
              <span className="tabular-nums">{formatPercent(rate.rate_bps)}</span>
            </li>
          ))}
        </ul>
      )}
      <div className="flex items-end gap-2">
        <Field
          label={t("tax.rateName.label")}
          tip={t("tax.rateName.tip")}
          value={rateName}
          onChange={setRateName}
        />
        <Field
          label={t("tax.ratePercent.label")}
          tip={t("tax.ratePercent.tip")}
          value={percent}
          onChange={setPercent}
          width="short"
        />
        <ActionButton
          colour="green"
          icon={Plus}
          label={t("tax.addRate.label")}
          tip={t("tax.addRate.tip")}
          onClick={add}
        />
      </div>
      {rateProblem ? (
        <p role="alert" className="text-red">
          {rateProblem}
        </p>
      ) : null}
      {rates.length > 0 ? (
        <SelectField
          label={t("tax.defaultRate.label")}
          tip={t("tax.defaultRate.tip")}
          value={defaultRate === "" ? (rates[0]?.id ?? "") : defaultRate}
          options={rates.map((rate) => ({
            value: rate.id,
            label: `${rate.name} (${formatPercent(rate.rate_bps)})`,
          }))}
          onChange={setDefaultRate}
          error={save.errorFor("default_rate_id")}
        />
      ) : null}
      <p className="text-text-muted">{t("tax.modeNote")}</p>
    </StepFrame>
  );
}
