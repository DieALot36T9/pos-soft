import { ArrowLeft, ListRestart } from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { HeldCartView } from "../../../generated/HeldCartView";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Tip } from "../../../shared/ui/Tip";
import { heldCarts } from "../api";

type HeldListDialogProps = {
  currency: CurrencyFormat;
  onResume: (cartId: string) => void;
  onClose: () => void;
};

// F6: the sales on hold at this till, oldest first (CHK-10). ↑/↓ move, Enter resumes.
export function HeldListDialog({ currency, onResume, onClose }: HeldListDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const [rows, setRows] = useState<HeldCartView[]>([]);
  const [chosen, setChosen] = useState(0);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    heldCarts()
      .then(setRows)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const options = { enableOnFormTags: true, preventDefault: true };
  useHotkeys("down", () => setChosen((old) => Math.min(old + 1, rows.length - 1)), options);
  useHotkeys("up", () => setChosen((old) => Math.max(old - 1, 0)), options);
  useHotkeys(
    "enter",
    () => {
      const row = rows[chosen];
      if (row) {
        onResume(row.cart_id);
      }
    },
    options,
  );
  useHotkeys("escape", onClose, { enableOnFormTags: true });

  return (
    <Dialog title={t("heldList.title")} colour="amber" width="wide">
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {rows.length === 0 ? <p>{t("heldList.empty")}</p> : null}
      {rows.length > 0 ? (
        <table className="w-full">
          <thead className="text-left text-text-muted">
            <tr>
              <th className="p-2">{t("heldList.time")}</th>
              <th className="p-2">{t("heldList.cashier")}</th>
              <th className="p-2 text-right">{t("heldList.items")}</th>
              <th className="p-2 text-right">{t("heldList.total")}</th>
              <th className="p-2">{t("heldList.note")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row, index) => (
              <tr
                key={row.cart_id}
                aria-selected={index === chosen}
                className={index === chosen ? "bg-amber-soft text-amber" : ""}
              >
                <td className="p-2">
                  <Tip tip={t("heldList.resumeTip")}>
                    <button
                      type="button"
                      onClick={() => onResume(row.cart_id)}
                      className="font-semibold underline"
                    >
                      {formatDateTime(row.held_at)}
                    </button>
                  </Tip>
                </td>
                <td className="p-2">{row.cashier_name}</td>
                <td className="p-2 text-right tabular-nums">{row.item_count}</td>
                <td className="p-2 text-right tabular-nums">{formatMoney(row.total, currency)}</td>
                <td className="p-2">{row.note ?? ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("heldList.back.label")}
          tip={t("heldList.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ListRestart aria-hidden="true" className="text-amber" />
      </div>
    </Dialog>
  );
}
