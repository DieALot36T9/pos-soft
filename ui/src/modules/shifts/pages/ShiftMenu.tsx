import { ArrowDownToLine, ArrowLeft, ArrowUpFromLine, Inbox, Landmark, Power } from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { shiftState } from "../api";
import { CashMoveDialog, type DrawerAction } from "./CashMoveDialog";
import { CloseShiftDialog } from "./CloseShiftDialog";

type ShiftMenuProps = {
  onClosedShift: () => void;
  onClose: () => void;
};

// F11 at the till: cash in and out, the no-sale drawer, and closing the shift (SHF-03 to SHF-06).
export function ShiftMenu({ onClosedShift, onClose }: ShiftMenuProps) {
  const { t } = useTranslation("shifts");
  const [currency, setCurrency] = useState<CurrencyFormat | null>(null);
  const [action, setAction] = useState<DrawerAction | null>(null);
  const [closing, setClosing] = useState(false);
  const [saved, setSaved] = useState(false);

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
        onSaved={() => {
          setAction(null);
          setSaved(true);
        }}
      />
    );
  }
  const choose = (next: DrawerAction) => () => {
    setSaved(false);
    setAction(next);
  };
  return (
    <Dialog title={t("menu.title")} colour="grey">
      {saved ? <Alert kind="done">{t("menu.done")}</Alert> : null}
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
