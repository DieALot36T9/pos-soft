import { Calculator, PlayCircle, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CountLine } from "../../../generated/CountLine";
import type { ShiftState } from "../../../generated/ShiftState";
import { errorMessage } from "../../../shared/bridge";
import { parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { openShift, shiftState } from "../api";
import { CountForm } from "../components/CountForm";

type OpenShiftDialogProps = {
  onOpened: () => void;
  onClose: () => void;
};

// Opens a shift with the float in the drawer, typed or counted note by note (SHF-01).
export function OpenShiftDialog({ onOpened, onClose }: OpenShiftDialogProps) {
  const { t } = useTranslation("shifts");
  const { t: tErrors } = useTranslation("errors");
  const [state, setState] = useState<ShiftState | null>(null);
  const [float, setFloat] = useState("0");
  const [counting, setCounting] = useState(false);
  const [counts, setCounts] = useState<CountLine[]>([]);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    shiftState()
      .then(setState)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const save = () => {
    if (state === null) {
      return;
    }
    const parsed = counting
      ? { ok: true as const, minor: 0 }
      : parseMoney(float, state.currency.minor_digits);
    if (!parsed.ok) {
      setProblem(
        errorMessage(
          { kind: "validation", field: "float", code: parsed.code, params: {} },
          tErrors,
        ),
      );
      return;
    }
    openShift(parsed.minor, counting ? counts : [])
      .then(onOpened)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  useHotkeys("enter", save, { enableOnFormTags: true, preventDefault: true }, [
    float,
    counts,
    counting,
    state,
  ]);
  useHotkeys("escape", onClose, { enableOnFormTags: true });

  return (
    <Dialog title={t("open.title")} colour="green">
      <p className="text-text-muted">{t("open.intro")}</p>
      {state && counting ? (
        <CountForm
          denominations={state.denominations}
          currency={state.currency}
          onChange={setCounts}
        />
      ) : (
        <Field
          label={t("open.float.label")}
          tip={t("open.float.tip")}
          value={float}
          onChange={setFloat}
          width="short"
          autoFocus
        />
      )}
      {!counting ? (
        <ActionButton
          colour="grey"
          icon={Calculator}
          label={t("open.countInstead.label")}
          tip={t("open.countInstead.tip")}
          onClick={() => setCounting(true)}
        />
      ) : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("open.back.label")}
          tip={t("open.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="green"
          icon={PlayCircle}
          label={t("open.save.label")}
          tip={t("open.save.tip")}
          shortcut="Enter"
          onClick={save}
        />
      </div>
    </Dialog>
  );
}
