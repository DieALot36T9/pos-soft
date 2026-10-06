import { Save, X } from "lucide-react";
import { useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CashMoveKind } from "../../../generated/CashMoveKind";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { errorMessage } from "../../../shared/bridge";
import { useApproval } from "../../../shared/approval/useApproval";
import { parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { moveCash, noSale } from "../api";

// Which drawer action the dialog does: a cash movement, or the no-sale drawer.
export type DrawerAction = CashMoveKind | "no_sale";

const titles = {
  cash_in: "move.titleCashIn",
  cash_out: "move.titleCashOut",
  cash_drop: "move.titleCashDrop",
  no_sale: "move.titleNoSale",
} as const;

type CashMoveDialogProps = {
  action: DrawerAction;
  currency: CurrencyFormat;
  onSaved: () => void;
  onClose: () => void;
};

// Cash in, paid-out, cash drop (SHF-03) or the no-sale drawer (SHF-04), each with a
// reason. A cashier gets the admin approval dialog; Rust checks it and audits both users.
export function CashMoveDialog({ action, currency, onSaved, onClose }: CashMoveDialogProps) {
  const { t } = useTranslation("shifts");
  const { t: tErrors } = useTranslation("errors");
  const { withApproval, dialog } = useApproval();
  const [amount, setAmount] = useState("");
  const [reason, setReason] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const save = () => {
    let work: (approval: string | null) => Promise<null>;
    if (action === "no_sale") {
      work = (approval) => noSale(reason, approval);
    } else {
      const parsed = parseMoney(amount, currency.minor_digits);
      if (!parsed.ok) {
        setProblem(
          errorMessage(
            { kind: "validation", field: "amount", code: parsed.code, params: {} },
            tErrors,
          ),
        );
        return;
      }
      work = (approval) => moveCash({ kind: action, amount: parsed.minor, reason }, approval);
    }
    withApproval(work)
      .then(onSaved)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const active = dialog === null;
  useHotkeys("enter", save, { enableOnFormTags: true, preventDefault: true, enabled: active }, [
    amount,
    reason,
  ]);
  useHotkeys("escape", onClose, { enableOnFormTags: true, enabled: active });

  return (
    <>
      <Dialog title={t(titles[action])} colour={action === "cash_in" ? "green" : "red"}>
        {action !== "no_sale" ? (
          <Field
            label={t("move.amount.label")}
            tip={t("move.amount.tip")}
            value={amount}
            onChange={setAmount}
            width="short"
            autoFocus
          />
        ) : null}
        <Field
          label={t("move.reason.label")}
          tip={t("move.reason.tip")}
          value={reason}
          onChange={setReason}
          autoFocus={action === "no_sale"}
        />
        {problem ? <Alert kind="problem">{problem}</Alert> : null}
        <div className="flex justify-between">
          <ActionButton
            colour="grey"
            icon={X}
            label={t("move.back.label")}
            tip={t("move.back.tip")}
            shortcut="Esc"
            onClick={onClose}
          />
          <ActionButton
            colour={action === "cash_in" ? "green" : "red"}
            icon={Save}
            label={t("move.save.label")}
            tip={t("move.save.tip")}
            shortcut="Enter"
            onClick={save}
          />
        </div>
      </Dialog>
      {dialog}
    </>
  );
}
