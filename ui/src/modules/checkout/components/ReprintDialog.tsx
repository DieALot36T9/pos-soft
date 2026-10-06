import { Printer, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";

type ReprintDialogProps = {
  onReprint: (invoiceNo: string) => void;
  onClose: () => void;
};

// "Old receipt": reprints any receipt by its invoice number (PAY-10). Rust checks the
// permission and marks the printout COPY.
export function ReprintDialog({ onReprint, onClose }: ReprintDialogProps) {
  const { t } = useTranslation("checkout");
  const [invoice, setInvoice] = useState("");
  const print = () => {
    if (invoice.trim() !== "") {
      onReprint(invoice.trim());
    }
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      print();
    } else if (event.key === "Escape") {
      onClose();
    }
  };
  return (
    <Dialog title={t("reprint.title")} colour="blue">
      <Field
        label={t("reprint.invoice.label")}
        tip={t("reprint.invoice.tip")}
        value={invoice}
        onChange={setInvoice}
        onKeyDown={onKeyDown}
        autoFocus
      />
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("reprint.back.label")}
          tip={t("reprint.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="blue"
          icon={Printer}
          label={t("reprint.print.label")}
          tip={t("reprint.print.tip")}
          shortcut="Enter"
          onClick={print}
        />
      </div>
    </Dialog>
  );
}
