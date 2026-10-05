import { Archive, Printer, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { PreviewPrintout } from "../../../generated/PreviewPrintout";
import { errorMessage } from "../../../shared/bridge";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Badge } from "../../../shared/ui/Badge";
import { printWindow } from "../api";

type PreviewPrinterPanelProps = {
  // Newest first.
  printouts: PreviewPrintout[];
  onClear: () => void;
};

// The Preview printer (rule HW-10): each printout as the exact picture the printer
// would print, a "Drawer opened" mark for drawer pulses, and printing the newest one
// through the computer's print dialog (rule HW-07, ADR-013).
export function PreviewPrinterPanel({ printouts, onClear }: PreviewPrinterPanelProps) {
  const { t } = useTranslation("hardware");
  const { t: tErrors } = useTranslation("errors");
  const [problem, setProblem] = useState<string | null>(null);

  const print = () => {
    setProblem(null);
    printWindow().catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <section
      aria-labelledby="preview-heading"
      className="rounded-ui border border-border bg-panel p-4"
    >
      <h3 id="preview-heading" className="mb-1 text-xl font-semibold">
        {t("preview.heading")}
      </h3>
      <p className="mb-3 text-text-muted">{t("preview.intro")}</p>
      {printouts.length === 0 ? (
        <p className="rounded-ui border border-border bg-bg p-3">{t("preview.empty")}</p>
      ) : (
        <>
          <div className="mb-3 flex flex-wrap gap-2">
            <ActionButton
              colour="blue"
              icon={Printer}
              label={t("preview.printButton.label")}
              tip={t("preview.printButton.tip")}
              onClick={print}
            />
            <ActionButton
              colour="grey"
              icon={X}
              label={t("preview.clearButton.label")}
              tip={t("preview.clearButton.tip")}
              onClick={onClear}
            />
          </div>
          {problem ? (
            <p role="alert" className="mb-3 rounded-ui border border-red bg-red-soft p-3 text-red">
              {problem}
            </p>
          ) : null}
          <ol className="flex flex-col gap-4">
            {printouts.map((printout, index) => (
              <PrintoutItem
                key={printouts.length - index}
                printout={printout}
                number={printouts.length - index}
                isNewest={index === 0}
              />
            ))}
          </ol>
        </>
      )}
    </section>
  );
}

type PrintoutItemProps = {
  printout: PreviewPrintout;
  number: number;
  // Only the newest printout goes to the print dialog.
  isNewest: boolean;
};

// One printout: its pictures at their real size in dots, and where they were saved.
function PrintoutItem({ printout, number, isNewest }: PrintoutItemProps) {
  const { t } = useTranslation("hardware");
  return (
    <li className="flex flex-col gap-2 border-t border-border pt-3">
      {printout.drawer_opened ? (
        <div>
          <Badge
            colour="green"
            icon={Archive}
            word={t("preview.drawerOpened")}
            tip={t("preview.drawerTip")}
          />
        </div>
      ) : null}
      <div data-print-area={isNewest ? "" : undefined} className="flex flex-col items-start">
        {printout.pictures.map((picture, pictureIndex) => (
          <img
            key={pictureIndex}
            src={picture}
            alt={t("preview.pictureAlt", { number })}
            className="max-w-full border border-border [image-rendering:pixelated]"
          />
        ))}
      </div>
      {printout.saved_files.map((file) => (
        <p key={file} className="text-sm break-all text-text-muted">
          {t("preview.savedTo", { file })}
        </p>
      ))}
    </li>
  );
}
