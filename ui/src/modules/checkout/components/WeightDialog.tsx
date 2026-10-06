import { CheckCircle, Keyboard, Scale, X } from "lucide-react";
import { useEffect, useState, type KeyboardEvent } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CartView } from "../../../generated/CartView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ScaleView } from "../../../generated/ScaleView";
import type { WeighProduct } from "../../../generated/WeighProduct";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { addWeighed, readScale } from "../api";

// How often the dialog asks the scale for the live weight.
const READ_EVERY_MS = 500;

type WeightDialogProps = {
  product: WeighProduct;
  currency: CurrencyFormat;
  // The user may type a weight (CheckoutManualWeight). Rust checks it again.
  canType: boolean;
  onAdded: (cart: CartView) => void;
  onClose: () => void;
};

// The weight dialog (CHK-06, CHK-07): the live weight with a green "Stable" badge, or
// a typed weight. Enter adds only a stable weight above zero; M switches to typing.
export function WeightDialog({ product, currency, canType, onAdded, onClose }: WeightDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const [scale, setScale] = useState<ScaleView | null>(null);
  const [typing, setTyping] = useState(false);
  const [text, setText] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    if (typing) {
      return;
    }
    let stopped = false;
    const read = () => {
      readScale(product.product_id)
        .then((view) => {
          if (!stopped) {
            setScale(view);
            if (!view.has_scale) {
              setTyping(canType);
            }
          }
        })
        .catch(() => undefined);
    };
    read();
    const timer = window.setInterval(read, READ_EVERY_MS);
    return () => {
      stopped = true;
      window.clearInterval(timer);
    };
  }, [typing, product.product_id, canType]);

  const add = () => {
    const weight = typing ? ({ kind: "manual", text } as const) : ({ kind: "scale" } as const);
    addWeighed(product.product_id, weight)
      .then(onAdded)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const ready = typing ? text.trim() !== "" : scale?.stable === true && (scale.qty ?? 0) > 0;
  const options = { enableOnFormTags: true, preventDefault: true };
  useHotkeys("enter", () => (ready ? add() : undefined), options, [ready, text, typing]);
  useHotkeys("escape", onClose, { enableOnFormTags: true });
  useHotkeys("m", () => (canType ? setTyping(true) : undefined), {
    enabled: !typing,
    enableOnFormTags: true,
    preventDefault: true,
  });
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      onClose();
    }
  };

  return (
    <Dialog title={t("weight.title", { name: product.name })} colour="blue">
      <p className="text-text-muted">
        {t("weight.priceTimesWeight", {
          price: formatMoney(product.unit_price, currency),
          unit: product.unit_symbol,
        })}
      </p>
      {typing ? (
        <Field
          label={t("weight.field.label")}
          tip={t("weight.field.tip", { unit: product.unit_symbol })}
          value={text}
          onChange={setText}
          onKeyDown={onKeyDown}
          width="short"
          autoFocus
        />
      ) : (
        <div className="flex items-center justify-between rounded-ui border border-border p-3">
          <span className="text-text-muted">{t("weight.live")}</span>
          <span className="text-[40px] font-semibold tabular-nums">
            {scale?.qty != null ? `${formatQty(scale.qty, 3)} ${product.unit_symbol}` : "—"}
          </span>
          {scale?.stable ? (
            <Badge
              colour="green"
              icon={Scale}
              word={t("weight.stable")}
              tip={t("weight.stableTip")}
            />
          ) : (
            <Badge
              colour="amber"
              icon={Scale}
              word={t("weight.moving")}
              tip={t("weight.movingTip")}
            />
          )}
        </div>
      )}
      {scale?.has_scale === false ? <Alert kind="info">{t("weight.noScale")}</Alert> : null}
      {scale?.problem && scale.has_scale ? (
        <Alert kind="warning">
          {errorMessage({ kind: "conflict", code: scale.problem, params: {} }, tErrors)}
        </Alert>
      ) : null}
      {!canType && (typing || scale?.has_scale === false) ? (
        <Alert kind="warning">{t("weight.noManual")}</Alert>
      ) : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex justify-between gap-2">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("weight.back.label")}
          tip={t("weight.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        {!typing && canType ? (
          <ActionButton
            colour="grey"
            icon={Keyboard}
            label={t("weight.typeIt.label")}
            tip={t("weight.typeIt.tip")}
            shortcut="M"
            onClick={() => setTyping(true)}
          />
        ) : null}
        <ActionButton
          colour="green"
          icon={CheckCircle}
          label={typing ? t("weight.add.label") : t("weight.addScale.label")}
          tip={typing ? t("weight.add.tip") : t("weight.addScale.tip")}
          shortcut="Enter"
          disabledReason={ready ? undefined : t("weight.movingTip")}
          onClick={add}
        />
      </div>
    </Dialog>
  );
}
