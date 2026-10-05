import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { DrawerPin } from "../../../generated/DrawerPin";
import type { PaperWidth } from "../../../generated/PaperWidth";
import { useSave } from "../../../shared/bridge/useSave";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveHardware } from "../api";
import { StepFrame } from "./StepFrame";
import type { StepProps } from "./types";

type PrinterChoice = "preview" | "none";

type HardwareStepProps = StepProps & {
  // Leaves the step open; it can be finished later in Settings (SET-10).
  onSkip: () => void;
};

// Step 6: the receipt printer, paper width and drawer pin. Can be skipped (SET-10).
export function HardwareStep({ saved, onSaved, onBack, onSkip }: HardwareStepProps) {
  const { t } = useTranslation("setup");
  const [printer, setPrinter] = useState<PrinterChoice>(
    saved.hardware.receipt_printer ? "preview" : "none",
  );
  const [paper, setPaper] = useState<PaperWidth>(saved.receipt.paper_width);
  const [pin, setPin] = useState<DrawerPin>(saved.hardware.drawer_pin);
  const save = useSave();

  const next = () =>
    save.run(
      () =>
        saveHardware({
          receipt_printer: printer === "preview" ? { kind: "preview" } : null,
          paper_width: paper,
          drawer_pin: pin,
        }),
      onSaved,
    );

  return (
    <StepFrame
      title={t("hardware.title")}
      intro={t("hardware.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={next}
      onSkip={onSkip}
    >
      <SelectField
        label={t("hardware.printer.label")}
        tip={t("hardware.printer.tip")}
        value={printer}
        options={(["preview", "none"] as const).map((value) => ({
          value,
          label: t(`hardware.printer.${value}`),
        }))}
        onChange={setPrinter}
      />
      <div className="grid grid-cols-2 gap-4">
        <SelectField
          label={t("hardware.paper.label")}
          tip={t("hardware.paper.tip")}
          value={paper}
          options={(["mm58", "mm80"] as const).map((value) => ({
            value,
            label: t(`hardware.paper.${value}`),
          }))}
          onChange={setPaper}
        />
        <SelectField
          label={t("hardware.drawer.label")}
          tip={t("hardware.drawer.tip")}
          value={pin}
          options={(["pin2", "pin5"] as const).map((value) => ({
            value,
            label: t(`hardware.drawer.${value}`),
          }))}
          onChange={setPin}
        />
      </div>
      <p className="text-text-muted">{t("hardware.later")}</p>
    </StepFrame>
  );
}
