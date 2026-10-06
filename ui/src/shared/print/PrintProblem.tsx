import { Printer, RotateCcw, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { PrintResult } from "../../generated/PrintResult";
import { errorMessage } from "../bridge";
import { ActionButton } from "../ui/ActionButton";
import { Alert } from "../ui/Alert";
import { dismissPrint, printWindow, retryPrint, systemPrinterPictures } from "./api";

type PrintProblemProps = {
  result: PrintResult;
  // Called when the banner should go: printed after all, or closed.
  onGone: () => void;
};

// The banner for a printout or drawer pulse that failed (PAY-09, HW-08): what went
// wrong, then Retry and "Print to system printer" (HW-07). Nothing is undone.
export function PrintProblem({ result, onGone }: PrintProblemProps) {
  const { t } = useTranslation("print");
  const { t: tErrors } = useTranslation("errors");
  const [current, setCurrent] = useState(result);
  const [pictures, setPictures] = useState<string[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const { printable } = current;

  if (current.printed) {
    return null;
  }
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));
  const heading =
    printable === null
      ? t("drawerFailed")
      : printable.kind === "receipt"
        ? t("receiptFailed")
        : t("reportFailed");
  const why = current.problem ? errorMessage(current.problem, tErrors) : null;

  const retry = () => {
    if (printable === null) {
      return;
    }
    setProblem(null);
    retryPrint(printable)
      .then((next) => {
        setCurrent(next);
        if (next.printed) {
          onGone();
        }
      })
      .catch(fail);
  };
  const systemPrint = () => {
    if (printable === null) {
      return;
    }
    setProblem(null);
    systemPrinterPictures(printable)
      .then((found) => {
        setPictures(found);
        // Let the pictures reach the page before the print dialog copies it.
        requestAnimationFrame(() => {
          printWindow().catch(fail);
        });
      })
      .catch(fail);
  };
  const close = () => {
    if (printable === null) {
      onGone();
      return;
    }
    dismissPrint(printable).then(onGone).catch(fail);
  };

  return (
    <div className="flex flex-col gap-2">
      <Alert kind="problem">
        <p className="font-semibold">{heading}</p>
        {why ? <p>{why}</p> : null}
        {problem ? <p>{problem}</p> : null}
      </Alert>
      <div className="flex flex-wrap gap-2">
        {printable !== null ? (
          <>
            <ActionButton
              colour="blue"
              icon={RotateCcw}
              label={t("retry.label")}
              tip={t("retry.tip")}
              onClick={retry}
            />
            <ActionButton
              colour="blue"
              icon={Printer}
              label={t("system.label")}
              tip={t("system.tip")}
              onClick={systemPrint}
            />
          </>
        ) : null}
        <ActionButton
          colour="grey"
          icon={X}
          label={t("dismiss.label")}
          tip={t("dismiss.tip")}
          onClick={close}
        />
      </div>
      {pictures.length > 0 ? (
        // Shown only on paper: the print CSS prints just this area (ADR-013).
        <div data-print-area="" className="hidden print:block">
          {pictures.map((picture, index) => (
            <img key={index} src={picture} alt={t("pictureAlt")} />
          ))}
        </div>
      ) : null}
    </div>
  );
}
