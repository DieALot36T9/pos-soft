import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveTill } from "../api";
import { StepFrame } from "./StepFrame";
import type { StepProps } from "./types";

const hours = Array.from({ length: 24 }, (_, hour) => String(hour));

// Step 5: the till's code and when the business day starts (SET-09).
export function TillStep({ saved, onSaved, onBack }: StepProps) {
  const { t } = useTranslation("setup");
  const [code, setCode] = useState(saved.till.till_code);
  const [cutoff, setCutoff] = useState(String(saved.till.business_day_cutoff_hour));
  const save = useSave();

  const next = () =>
    save.run(
      () => saveTill({ till_code: code, business_day_cutoff_hour: Number(cutoff) }),
      onSaved,
    );

  return (
    <StepFrame
      title={t("till.title")}
      intro={t("till.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={next}
    >
      <Field
        label={t("till.code.label")}
        tip={t("till.code.tip")}
        value={code}
        onChange={setCode}
        error={save.errorFor("till_code")}
        width="short"
      />
      <SelectField
        label={t("till.cutoff.label")}
        tip={t("till.cutoff.tip")}
        value={cutoff}
        options={hours.map((hour) => ({
          value: hour,
          label: t("till.hour", { hour: hour.padStart(2, "0") }),
        }))}
        onChange={setCutoff}
      />
    </StepFrame>
  );
}
