import {
  ArrowDownToLine,
  ArrowLeft,
  ArrowUpFromLine,
  FileText,
  Inbox,
  Landmark,
  Power,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { PrintResult } from "../../../generated/PrintResult";
import { errorMessage } from "../../../shared/bridge";
import { PrintProblem } from "../../../shared/print/PrintProblem";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { printXReport, shiftState } from "../api";
import { CashMoveDialog, type DrawerAction } from "./CashMoveDialog";
import { CloseShiftDialog } from "./CloseShiftDialog";

type ShiftMenuProps = {
  onClosedShift: () => void;
  onClose: () => void;
};

// F11 at the till: cash in and out, the no-sale drawer, the X report and closing the
// shift (SHF-03 to SHF-06).
export function ShiftMenu({ onClosedShift, onClose }: ShiftMenuProps) {
  const { t } = useTranslation("shifts");
  const { t: tErrors } = useTranslation("errors");
  // What the last action did: "saved" or "x", a print or drawer problem, or an error.
  const [printFailed, setPrintFailed] = useState<PrintResult | null>(null);
  const [note, setNote] = useState<"saved" | "x" | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [currency, setCurrency] = useState<CurrencyFormat | null>(null);
  const [action, setAction] = useState<DrawerAction | null>(null);
  const [closing, setClosing] = useState(false);

  useEffect(() => {
    shiftState()
      .then((state) => setCurrency(state.currency))
      .catch(() => undefined);
  }, []);
  useHotkeys("escape", onClose, { enableOnFormTags: true, enabled: action === null && !closing });

  if (closing) {
    return (
      <CloseShiftDialog force={false} onClosed={onClosedShift} onClose={() => setClosing(false)} />
    );
  }
  if (action && currency) {
    return (
      <CashMoveDialog
        action={action}
        currency={currency}
        onClose={() => setAction(null)}
        onSaved={(drawer) => {
          setAction(null);
          show(drawer, "saved");
        }}
      />
    );
  }
  function show(result: PrintResult, done: "saved" | "x") {
    setProblem(null);
    setNote(done);
    setPrintFailed(result.printed ? null : result);
  }
  const choose = (next: DrawerAction) => () => {
    setNote(null);
    setPrintFailed(null);
    setAction(next);
  };
  const xReport = () => {
    setNote(null);
    printXReport()
      .then((result) => show(result, "x"))
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  return (
    <Dialog title={t("menu.title")} colour="grey">
      {note === "saved" ? <Alert kind="done">{t("menu.done")}</Alert> : null}
      {note === "x" && printFailed === null ? (
        <Alert kind="done">{t("menu.xPrinted")}</Alert>
      ) : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {printFailed ? (
        <PrintProblem result={printFailed} onGone={() => setPrintFailed(null)} />
      ) : null}
      <div className="grid grid-cols-2 gap-2">
        <ActionButton
          colour="green"
          icon={ArrowDownToLine}
          size="till"
          label={t("menu.cashIn.label")}
          tip={t("menu.cashIn.tip")}
          onClick={choose("cash_in")}
        />
        <ActionButton
          colour="red"
          icon={ArrowUpFromLine}
          size="till"
          label={t("menu.cashOut.label")}
          tip={t("menu.cashOut.tip")}
          onClick={choose("cash_out")}
        />
        <ActionButton
          colour="red"
          icon={Landmark}
          size="till"
          label={t("menu.cashDrop.label")}
          tip={t("menu.cashDrop.tip")}
          onClick={choose("cash_drop")}
        />
        <ActionButton
          colour="amber"
          icon={Inbox}
          size="till"
          label={t("menu.noSale.label")}
          tip={t("menu.noSale.tip")}
          onClick={choose("no_sale")}
        />
        <ActionButton
          colour="blue"
          icon={FileText}
          size="till"
          label={t("menu.xReport.label")}
          tip={t("menu.xReport.tip")}
          onClick={xReport}
        />
      </div>
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("menu.back.label")}
          tip={t("menu.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="red"
          icon={Power}
          label={t("menu.close.label")}
          tip={t("menu.close.tip")}
          onClick={() => setClosing(true)}
        />
      </div>
    </Dialog>
  );
}
