import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { SessionView } from "../generated/SessionView";
import { CheckoutPage } from "../modules/checkout/pages/CheckoutPage";
import { TillPreviewPanel } from "../modules/hardware/components/TillPreviewPanel";
import { CloseShiftDialog } from "../modules/shifts/pages/CloseShiftDialog";
import { OpenShiftDialog } from "../modules/shifts/pages/OpenShiftDialog";
import { ShiftMenu } from "../modules/shifts/pages/ShiftMenu";
import { errorMessage } from "../shared/bridge";
import { Alert } from "../shared/ui/Alert";
import { lockTill } from "./api";
import { useIdleLock } from "./useIdleLock";

type TillFrameProps = {
  session: SessionView;
  justLoggedIn: boolean;
  idleLockMinutes: number;
  onLocked: (session: SessionView) => void;
  onBackOffice: () => void;
};

// Till mode: the checkout screen full width, with the shift dialogs from the shifts
// module and the Preview printer's newest printout from the hardware module. The
// modules don't know each other; this frame joins them.
export function TillFrame({
  session,
  justLoggedIn,
  idleLockMinutes,
  onLocked,
  onBackOffice,
}: TillFrameProps) {
  const { t: tErrors } = useTranslation("errors");
  const [reloadKey, setReloadKey] = useState(0);
  const [shiftDialog, setShiftDialog] = useState<"open" | "menu" | "force" | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const lock = () => {
    lockTill()
      .then(onLocked)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  useIdleLock(idleLockMinutes, lock);
  const reload = () => {
    setShiftDialog(null);
    setReloadKey((old) => old + 1);
  };

  return (
    <div className="flex flex-1 flex-col">
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <CheckoutPage
        session={session}
        reloadKey={reloadKey}
        justLoggedIn={justLoggedIn}
        onOpenShift={() => setShiftDialog("open")}
        onForceClose={() => setShiftDialog("force")}
        onShiftMenu={() => setShiftDialog("menu")}
        onLock={lock}
        onBackOffice={onBackOffice}
      />
      <TillPreviewPanel />
      {shiftDialog === "open" ? (
        <OpenShiftDialog onOpened={reload} onClose={() => setShiftDialog(null)} />
      ) : null}
      {shiftDialog === "menu" ? <ShiftMenu onClosedShift={reload} onClose={reload} /> : null}
      {shiftDialog === "force" ? (
        <CloseShiftDialog force onClosed={reload} onClose={() => setShiftDialog(null)} />
      ) : null}
    </div>
  );
}
