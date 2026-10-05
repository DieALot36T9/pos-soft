import { Save, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { RateWithComponents } from "../../../generated/RateWithComponents";
import { useSave } from "../../../shared/bridge/useSave";
import { formatPercent, parsePercent } from "../../../shared/format/percent";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { saveRate } from "../api";

type RateFormProps = {
  // The rate being changed, or null to add a new one.
  rate: RateWithComponents | null;
  onSaved: () => void;
  onCancel: () => void;
};

// Adds or changes one rate. Parts of a rate (components) are kept as they are.
export function RateForm({ rate, onSaved, onCancel }: RateFormProps) {
  const { t } = useTranslation("tax");
  const [name, setName] = useState(rate?.name ?? "");
  const [percent, setPercent] = useState(rate ? formatPercent(rate.rate_bps).replace("%", "") : "");
  const [percentProblem, setPercentProblem] = useState<string | null>(null);
  const save = useSave();

  const submit = () => {
    const parsed = parsePercent(percent);
    if (!parsed.ok) {
      setPercentProblem(t("form.percentInvalid"));
      return;
    }
    setPercentProblem(null);
    save.run(
      () =>
        saveRate({
          id: rate?.id ?? null,
          rate: {
            name,
            rate_bps: parsed.bps,
            components: rate?.components ?? [],
            sort: rate?.sort ?? 0,
          },
        }),
      () => {
        setName("");
        setPercent("");
        onSaved();
      },
    );
  };

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {rate ? t("form.editHeading", { name: rate.name }) : t("form.addHeading")}
      </h3>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("form.name.label")}
          tip={t("form.name.tip")}
          value={name}
          onChange={setName}
          error={save.errorFor("name")}
        />
        <Field
          label={t("form.percent.label")}
          tip={t("form.percent.tip")}
          value={percent}
          onChange={setPercent}
          error={percentProblem ?? save.errorFor("rate_bps")}
          width="short"
        />
      </div>
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("form.saved")}</Alert> : null}
      <div className="flex justify-between">
        {rate ? (
          <ActionButton
            colour="grey"
            icon={X}
            label={t("form.cancel.label")}
            tip={t("form.cancel.tip")}
            onClick={onCancel}
          />
        ) : (
          <span />
        )}
        <ActionButton
          colour="green"
          icon={Save}
          label={t("form.save.label")}
          tip={t("form.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
