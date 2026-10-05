import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

const hours = Array.from({ length: 24 }, (_, hour) => String(hour));

// This till's code and when its business day starts (SET-09).
export function TillSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const [code, setCode] = useState(saved.till.till_code);
  const [cutoff, setCutoff] = useState(String(saved.till.business_day_cutoff_hour));
  const save = useSave();

  const submit = () =>
    save.run(
      () =>
        saveSettings({
          section: "till",
          value: { till_code: code, business_day_cutoff_hour: Number(cutoff) },
        }),
      onSaved,
    );

  return (
    <SectionFrame
      title={t("sections.till")}
      busy={save.busy}
      done={save.done}
      problem={save.problem}
      onSave={submit}
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
    </SectionFrame>
  );
}
