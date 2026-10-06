import { ArrowLeft, Ban } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { VoidDone } from "../../../generated/VoidDone";
import { useApproval } from "../../../shared/approval/useApproval";
import { errorMessage } from "../../../shared/bridge";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { voidSale } from "../api";

type VoidDialogProps = {
  saleId: string;
  invoiceNo: string;
  onVoided: (done: VoidDone) => void;
  onClose: () => void;
};

// Voids a whole sale after a reason is typed (RET-04). Rust checks the permission (an
// admin may approve), that the sale's shift is still open, and reverses everything.
export function VoidDialog({ saleId, invoiceNo, onVoided, onClose }: VoidDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const { withApproval, dialog: approvalDialog } = useApproval();
  const [reason, setReason] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const confirm = () => {
    withApproval((approval) => voidSale(saleId, reason, approval))
      .then(onVoided)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  return (
    <Dialog title={t("void.title", { number: invoiceNo })} colour="red">
      <p>{t("void.body")}</p>
      <Field
        label={t("void.reason.label")}
        tip={t("void.reason.tip")}
        value={reason}
        onChange={setReason}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            confirm();
          } else if (event.key === "Escape") {
            onClose();
          }
        }}
        autoFocus
      />
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("void.back.label")}
          tip={t("void.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="red"
          icon={Ban}
          label={t("void.confirm.label")}
          tip={t("void.confirm.tip")}
          shortcut="Enter"
          onClick={confirm}
        />
      </div>
      {approvalDialog}
    </Dialog>
  );
}
