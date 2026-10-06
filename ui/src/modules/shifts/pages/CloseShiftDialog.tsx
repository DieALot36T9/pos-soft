import { Power, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CountLine } from "../../../generated/CountLine";
import type { ShiftClosed } from "../../../generated/ShiftClosed";
import type { ShiftState } from "../../../generated/ShiftState";
import { useApproval } from "../../../shared/approval/useApproval";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { closeShift, forceClose, shiftState } from "../api";
import { CountForm } from "../components/CountForm";
import { ClosedSummary } from "../components/ClosedSummary";

type CloseShiftDialogProps = {
  // True: an admin closes a shift left open (SHF-07); the count is optional.
  force: boolean;
  onClosed: () => void;
  onClose: () => void;
};

// Closing a shift with the cash count (SHF-06). With "count first" on, the expected
// cash shows only after the count is saved. Rust works out the difference.
export function CloseShiftDialog({ force, onClosed, onClose }: CloseShiftDialogProps) {
  const { t } = useTranslation("shifts");
  const { t: tErrors } = useTranslation("errors");
  const { withApproval, dialog } = useApproval();
  const [state, setState] = useState<ShiftState | null>(null);
  const [counts, setCounts] = useState<CountLine[]>([]);
  const [note, setNote] = useState("");
  const [result, setResult] = useState<ShiftClosed | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    shiftState()
      .then(setState)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const save = () => {
    const typedNote = note.trim() === "" ? null : note;
    const work = force
      ? withApproval((approval) => forceClose(counts, typedNote, approval))
      : closeShift(counts, typedNote);
    work.then(setResult).catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const active = dialog === null && result === null;
  useHotkeys("escape", onClose, { enableOnFormTags: true, enabled: active });

  if (result && state) {
    return <ClosedSummary result={result} currency={state.currency} onDone={onClosed} />;
  }
  return (
    <>
      <Dialog title={force ? t("close.forceTitle") : t("close.title")} colour="red" width="wide">
        <p className="text-text-muted">{force ? t("close.forceIntro") : t("close.intro")}</p>
        {state?.expected_cash != null ? (
          <p className="text-xl font-semibold">
            {t("close.expected", { amount: formatMoney(state.expected_cash, state.currency) })}
          </p>
        ) : null}
        {state ? (
          <CountForm
            denominations={state.denominations}
            currency={state.currency}
            onChange={setCounts}
          />
        ) : null}
        <Field
          label={t("close.note.label")}
          tip={t("close.note.tip")}
          value={note}
          onChange={setNote}
        />
        {problem ? <Alert kind="problem">{problem}</Alert> : null}
        <div className="flex justify-between">
          <ActionButton
            colour="grey"
            icon={X}
            label={t("close.back.label")}
            tip={t("close.back.tip")}
            shortcut="Esc"
            onClick={onClose}
          />
          <ActionButton
            colour="red"
            icon={Power}
            label={force ? t("close.forceSave.label") : t("close.save.label")}
            tip={force ? t("close.forceSave.tip") : t("close.save.tip")}
            onClick={save}
          />
        </div>
      </Dialog>
      {dialog}
    </>
  );
}
