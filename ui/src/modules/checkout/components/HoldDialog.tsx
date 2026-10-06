import { PauseCircle, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";

type HoldDialogProps = {
  onHold: (note: string | null) => void;
  onClose: () => void;
};

// F5: puts the sale aside with an optional note (CHK-09).
export function HoldDialog({ onHold, onClose }: HoldDialogProps) {
  const { t } = useTranslation("checkout");
  const [note, setNote] = useState("");
  const hold = () => onHold(note.trim() === "" ? null : note);
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      hold();
    } else if (event.key === "Escape") {
      onClose();
    }
  };
  return (
    <Dialog title={t("hold.title")} colour="amber">
      <Field
        label={t("hold.note.label")}
        tip={t("hold.note.tip")}
        value={note}
        onChange={setNote}
        onKeyDown={onKeyDown}
        autoFocus
      />
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("hold.back.label")}
          tip={t("hold.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="amber"
          icon={PauseCircle}
          label={t("hold.save.label")}
          tip={t("hold.save.tip")}
          shortcut="Enter"
          onClick={hold}
        />
      </div>
    </Dialog>
  );
}
