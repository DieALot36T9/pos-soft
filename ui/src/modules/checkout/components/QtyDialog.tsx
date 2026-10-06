import { Check, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { CartLineView } from "../../../generated/CartLineView";
import { formatQty, parseQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";

type QtyDialogProps = {
  line: CartLineView;
  onSave: (qty: number) => void;
  onClose: () => void;
};

// F4: a new quantity for the selected line (CHK-04). Rust refuses 1.5 pieces; 0 removes.
export function QtyDialog({ line, onSave, onClose }: QtyDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const [text, setText] = useState(formatQty(line.qty, line.unit_decimals));
  const [problem, setProblem] = useState<string | null>(null);

  const save = () => {
    // Read with three decimals: Rust decides how many the unit allows.
    const parsed = parseQty(text, 3);
    if (!parsed.ok) {
      setProblem(tErrors("qty_invalid"));
      return;
    }
    onSave(parsed.milli);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      save();
    } else if (event.key === "Escape") {
      onClose();
    }
  };

  return (
    <Dialog title={t("qty.title")} colour="grey">
      <Field
        label={t("qty.field.label")}
        tip={t("qty.field.tip", { name: line.name })}
        value={text}
        onChange={setText}
        onKeyDown={onKeyDown}
        error={problem}
        width="short"
        autoFocus
      />
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("qty.back.label")}
          tip={t("qty.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="green"
          icon={Check}
          label={t("qty.save.label")}
          tip={t("qty.save.tip")}
          shortcut="Enter"
          onClick={save}
        />
      </div>
    </Dialog>
  );
}
