import { ArrowLeft, XCircle } from "lucide-react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";

type CancelDialogProps = {
  itemCount: number;
  onConfirm: () => void;
  onClose: () => void;
};

// F9: asks before the whole sale is thrown away (CHK-05).
export function CancelDialog({ itemCount, onConfirm, onClose }: CancelDialogProps) {
  const { t } = useTranslation("checkout");
  useHotkeys("enter", onConfirm, { enableOnFormTags: true, preventDefault: true });
  useHotkeys("escape", onClose, { enableOnFormTags: true });
  return (
    <Dialog title={t("cancel.title")} colour="red">
      <p>{t("cancel.body", { count: itemCount })}</p>
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("cancel.back.label")}
          tip={t("cancel.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="red"
          icon={XCircle}
          label={t("cancel.confirm.label")}
          tip={t("cancel.confirm.tip")}
          shortcut="Enter"
          onClick={onConfirm}
        />
      </div>
    </Dialog>
  );
}
