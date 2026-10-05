import { Printer } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { PaperWidth } from "../../../generated/PaperWidth";
import { errorMessage } from "../../../shared/bridge";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Tip } from "../../../shared/ui/Tip";
import { testPrint } from "../api";

type TestPrintPanelProps = {
  // Called after the printer took the job, so the preview can show it at once.
  onPrinted: () => void;
};

const papers: PaperWidth[] = ["mm58", "mm80"];

// Paper width choice and the "Test print" button (rule HW-02).
export function TestPrintPanel({ onPrinted }: TestPrintPanelProps) {
  const { t } = useTranslation("hardware");
  const { t: tErrors } = useTranslation("errors");
  const [paper, setPaper] = useState<PaperWidth>("mm80");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const print = () => {
    setBusy(true);
    setMessage(null);
    setProblem(null);
    testPrint(paper)
      .then(() => {
        setMessage(t("testPrint.sent"));
        onPrinted();
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)))
      .finally(() => setBusy(false));
  };

  return (
    <section className="rounded-ui border border-border bg-panel p-4">
      <h3 className="mb-3 text-xl font-semibold">{t("testPrint.heading")}</h3>
      <fieldset className="mb-4">
        <legend className="mb-1 font-semibold">{t("testPrint.paperLabel")}</legend>
        <div className="flex gap-4">
          {papers.map((choice) => (
            <Tip key={choice} tip={t(`testPrint.${choice}.tip`)}>
              <label className="flex min-h-10 items-center gap-2">
                <input
                  type="radio"
                  name="paper"
                  value={choice}
                  checked={paper === choice}
                  onChange={() => setPaper(choice)}
                  className="size-5 accent-blue"
                />
                {t(`testPrint.${choice}.label`)}
              </label>
            </Tip>
          ))}
        </div>
      </fieldset>
      <ActionButton
        colour="blue"
        icon={Printer}
        label={t("testPrint.button.label")}
        tip={t("testPrint.button.tip")}
        disabledReason={busy ? t("testPrint.busy") : undefined}
        onClick={print}
      />
      {message ? (
        <p role="status" className="mt-3 text-green">
          {message}
        </p>
      ) : null}
      {problem ? (
        <p role="alert" className="mt-3 rounded-ui border border-red bg-red-soft p-3 text-red">
          {problem}
        </p>
      ) : null}
    </section>
  );
}
