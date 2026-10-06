import { Check, Trash2, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ManualDiscount } from "../../../generated/ManualDiscount";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { formatPercent, parsePercent } from "../../../shared/format/percent";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";

type DiscountDialogProps = {
  // The line's product name, or null for the whole sale.
  lineName: string | null;
  current: ManualDiscount | null;
  // The user's own limit, to say when an admin is needed. Rust checks it (PRC-07).
  limit: number;
  currency: CurrencyFormat;
  onSave: (discount: ManualDiscount | null) => void;
  onClose: () => void;
};

type Kind = ManualDiscount["kind"];

// F7 / Shift+F7: a discount as a percentage or an amount (CHK-12). Rust works out what it
// takes off and asks for an admin when it is above the user's limit.
export function DiscountDialog(props: DiscountDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const plain = (minor: number) =>
    formatMoney(minor, { ...props.currency, symbol: "", group_sep: "", decimal_sep: "." });
  const [kind, setKind] = useState<Kind>(props.current?.kind ?? "percent");
  const [text, setText] = useState(() => {
    if (!props.current) {
      return "";
    }
    return props.current.kind === "percent"
      ? formatPercent(props.current.value).replace("%", "")
      : plain(props.current.value);
  });
  const [problem, setProblem] = useState<string | null>(null);
  const limit = formatPercent(Math.min(props.limit, 10000));

  const save = () => {
    if (kind === "percent") {
      const parsed = parsePercent(text);
      if (!parsed.ok) {
        setProblem(tErrors("promotion_percent_invalid"));
        return;
      }
      props.onSave({ kind: "percent", value: parsed.bps });
      return;
    }
    const parsed = parseMoney(text, props.currency.minor_digits);
    if (!parsed.ok) {
      setProblem(tErrors("money_not_a_number"));
      return;
    }
    props.onSave({ kind: "amount", value: parsed.minor });
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      save();
    } else if (event.key === "Escape") {
      props.onClose();
    }
  };

  return (
    <Dialog
      title={
        props.lineName === null
          ? t("discount.invoiceTitle")
          : t("discount.lineTitle", { name: props.lineName })
      }
      colour="purple"
    >
      <SelectField<Kind>
        label={t("discount.kind.label")}
        tip={t("discount.kind.tip")}
        value={kind}
        options={[
          { value: "percent", label: t("discount.kind.percent") },
          { value: "amount", label: t("discount.kind.amount") },
        ]}
        onChange={(value) => {
          setKind(value);
          setText("");
          setProblem(null);
        }}
      />
      <Field
        label={kind === "percent" ? t("discount.percent.label") : t("discount.amount.label")}
        tip={kind === "percent" ? t("discount.percent.tip", { limit }) : t("discount.amount.tip")}
        value={text}
        onChange={setText}
        onKeyDown={onKeyDown}
        error={problem}
        width="short"
        autoFocus
      />
      <p className="text-text-muted">{t("discount.limitNote", { limit })}</p>
      <div className="flex justify-between gap-2">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("discount.back.label")}
          tip={t("discount.back.tip")}
          shortcut="Esc"
          onClick={props.onClose}
        />
        {props.current ? (
          <ActionButton
            colour="red"
            icon={Trash2}
            label={t("discount.remove.label")}
            tip={t("discount.remove.tip")}
            onClick={() => props.onSave(null)}
          />
        ) : null}
        <ActionButton
          colour="purple"
          icon={Check}
          label={t("discount.save.label")}
          tip={t("discount.save.tip")}
          shortcut="Enter"
          onClick={save}
        />
      </div>
    </Dialog>
  );
}
