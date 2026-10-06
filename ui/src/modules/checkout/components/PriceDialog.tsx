import { Check, RotateCcw, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { CartLineView } from "../../../generated/CartLineView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";

type PriceDialogProps = {
  line: CartLineView;
  currency: CurrencyFormat;
  // A price for one unit, or null to go back to the normal price.
  onSave: (price: number | null) => void;
  onClose: () => void;
};

// F8: a typed unit price for the selected line (CHK-13). Without `PriceOverride` an
// admin approves; the old and new price go into the audit log.
export function PriceDialog({ line, currency, onSave, onClose }: PriceDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const [text, setText] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const save = () => {
    const parsed = parseMoney(text, currency.minor_digits);
    if (!parsed.ok) {
      setProblem(tErrors("money_not_a_number"));
      return;
    }
    onSave(parsed.minor);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      save();
    } else if (event.key === "Escape") {
      onClose();
    }
  };

  return (
    <Dialog title={t("price.title", { name: line.name })} colour="purple">
      <p>{t("price.normal", { price: formatMoney(line.unit_price, currency) })}</p>
      <Field
        label={t("price.field.label")}
        tip={t("price.field.tip")}
        value={text}
        onChange={setText}
        onKeyDown={onKeyDown}
        error={problem}
        width="short"
        autoFocus
      />
      <div className="flex justify-between gap-2">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("price.back.label")}
          tip={t("price.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        {line.price_overridden ? (
          <ActionButton
            colour="grey"
            icon={RotateCcw}
            label={t("price.reset.label")}
            tip={t("price.reset.tip")}
            onClick={() => onSave(null)}
          />
        ) : null}
        <ActionButton
          colour="purple"
          icon={Check}
          label={t("price.save.label")}
          tip={t("price.save.tip")}
          shortcut="Enter"
          onClick={save}
        />
      </div>
    </Dialog>
  );
}
