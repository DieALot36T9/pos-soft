import {
  Banknote,
  BookOpen,
  CreditCard,
  Smartphone,
  Star,
  Wallet,
  type LucideIcon,
} from "lucide-react";
import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { MethodKind } from "../../../generated/MethodKind";
import type { PaymentMethodView } from "../../../generated/PaymentMethodView";
import { cn } from "../../../shared/lib/utils";
import type { ButtonColour } from "../../../shared/ui/colours";
import { Tip } from "../../../shared/ui/Tip";

// Each kind of payment has its own colour and icon everywhere (Design System).
const looks: Record<MethodKind, { colour: ButtonColour; icon: LucideIcon }> = {
  cash: { colour: "green", icon: Banknote },
  card: { colour: "blue", icon: CreditCard },
  wallet: { colour: "teal", icon: Smartphone },
  other: { colour: "grey", icon: Wallet },
  credit: { colour: "amber", icon: BookOpen },
  points: { colour: "purple", icon: Star },
  exchange_credit: { colour: "grey", icon: Wallet },
};

const softText: Record<ButtonColour, string> = {
  green: "bg-green-soft text-green",
  blue: "bg-blue-soft text-blue",
  teal: "bg-teal-soft text-teal",
  grey: "bg-grey-soft text-grey",
  amber: "bg-amber-soft text-amber",
  purple: "bg-purple-soft text-purple",
  red: "bg-red-soft text-red",
};

type PaymentRowProps = {
  method: PaymentMethodView;
  amount: string;
  reference: string;
  onAmount: (text: string) => void;
  onReference: (text: string) => void;
  // ↑/↓ move to the next row (Keyboard Shortcuts, "Payment dialog").
  onArrow: (step: 1 | -1) => void;
  inputId: string;
  // What the customer's account allows on this row (credit room, points), with a button
  // that fills in the most points can pay.
  note?: { text: string; tip: string; use?: { label: string; tip: string; amount: number } };
  onUse?: (amount: number) => void;
};

// One payment method in the payment dialog: its colour and icon, the amount, and a
// reference box when the method takes one (PAY-02).
export function PaymentRow(props: PaymentRowProps) {
  const { method } = props;
  const { t } = useTranslation("checkout");
  const look = looks[method.kind];
  const Icon = look.icon;
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      props.onArrow(event.key === "ArrowDown" ? 1 : -1);
    }
  };
  return (
    <div className="flex items-center gap-3">
      <span
        className={cn(
          "flex w-36 items-center gap-2 rounded-ui px-2 py-2 font-semibold",
          softText[look.colour],
        )}
      >
        <Icon aria-hidden="true" size={20} />
        {method.name}
      </span>
      <Tip
        tip={
          method.kind === "cash"
            ? t("payment.amount.cashTip")
            : t("payment.amount.tip", { method: method.name })
        }
      >
        <input
          id={props.inputId}
          aria-label={t("payment.amount.label", { method: method.name })}
          value={props.amount}
          onChange={(event) => props.onAmount(event.target.value)}
          onKeyDown={onKeyDown}
          onFocus={(event) => event.target.select()}
          className="min-h-10 w-40 rounded-ui border border-border px-3 text-right text-lg tabular-nums"
        />
      </Tip>
      {method.requires_reference ? (
        <Tip tip={t("payment.reference.tip")}>
          <input
            aria-label={`${method.name} ${t("payment.reference.label")}`}
            placeholder={t("payment.reference.label")}
            value={props.reference}
            onChange={(event) => props.onReference(event.target.value)}
            className="min-h-10 flex-1 rounded-ui border border-border px-3"
          />
        </Tip>
      ) : null}
      {props.note ? (
        <Tip tip={props.note.tip}>
          <span className="text-text-muted">{props.note.text}</span>
        </Tip>
      ) : null}
      {props.note?.use && props.onUse ? (
        <Tip tip={props.note.use.tip}>
          <button
            type="button"
            onClick={() => props.onUse?.(props.note?.use?.amount ?? 0)}
            className="min-h-10 rounded-ui border border-purple bg-purple-soft px-3 font-semibold text-purple"
          >
            {props.note.use.label}
          </button>
        </Tip>
      ) : null}
    </div>
  );
}
