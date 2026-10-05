import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { finish } from "../api";
import { StepFrame } from "./StepFrame";
import type { StepProps } from "./types";

// Step 8: products. For now only "Add products later" (SET-12).
export function ProductsStep({ onSaved, onBack }: Omit<StepProps, "saved">) {
  const { t } = useTranslation("setup");
  const save = useSave();
  return (
    <StepFrame
      title={t("products.title")}
      intro={t("products.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={() => save.run(finish, onSaved)}
    >
      <p className="rounded-ui border border-green bg-green-soft p-3 font-semibold text-green">
        {t("products.later")}
      </p>
      <p className="text-text-muted">{t("products.laterNote")}</p>
    </StepFrame>
  );
}
