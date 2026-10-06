import {
  Hash,
  ListRestart,
  Lock,
  PauseCircle,
  Search,
  Trash2,
  Wallet,
  XCircle,
  LayoutGrid,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";

type ButtonBarProps = {
  hasLines: boolean;
  hasSelection: boolean;
  heldCount: number;
  onSearch: () => void;
  onQty: () => void;
  onHold: () => void;
  onHeld: () => void;
  onRemove: () => void;
  onCancel: () => void;
  onShift: () => void;
  onLock: () => void;
  onBackOffice: () => void;
};

// The buttons under the cart, each with its icon, word, colour and key (Checkout Screen).
// Discounts, price changes, customers and returns come in later milestones.
export function ButtonBar(props: ButtonBarProps) {
  const { t } = useTranslation("checkout");
  const noItems = props.hasLines ? undefined : t("pay.empty");
  const noSelection = props.hasSelection ? undefined : t("buttons.qty.none");
  return (
    <nav className="flex flex-wrap gap-2 border-t border-border bg-panel p-3">
      <ActionButton
        colour="blue"
        icon={Search}
        size="till"
        label={t("buttons.search.label")}
        tip={t("buttons.search.tip")}
        shortcut={shortcuts.checkoutSearch.label}
        onClick={props.onSearch}
      />
      <ActionButton
        colour="grey"
        icon={Hash}
        size="till"
        label={t("buttons.qty.label")}
        tip={t("buttons.qty.tip")}
        shortcut={shortcuts.checkoutQty.label}
        disabledReason={noSelection}
        onClick={props.onQty}
      />
      <ActionButton
        colour="amber"
        icon={PauseCircle}
        size="till"
        label={t("buttons.hold.label")}
        tip={t("buttons.hold.tip")}
        shortcut={shortcuts.checkoutHold.label}
        disabledReason={noItems}
        onClick={props.onHold}
      />
      <ActionButton
        colour="amber"
        icon={ListRestart}
        size="till"
        label={t("buttons.held.label", { count: props.heldCount })}
        tip={t("buttons.held.tip", { count: props.heldCount })}
        shortcut={shortcuts.checkoutHeld.label}
        disabledReason={props.heldCount === 0 ? t("buttons.held.none") : undefined}
        onClick={props.onHeld}
      />
      <ActionButton
        colour="red"
        icon={Trash2}
        size="till"
        label={t("buttons.remove.label")}
        tip={t("buttons.remove.tip")}
        shortcut={shortcuts.checkoutRemove.label}
        disabledReason={noSelection}
        onClick={props.onRemove}
      />
      <ActionButton
        colour="red"
        icon={XCircle}
        size="till"
        label={t("buttons.cancel.label")}
        tip={t("buttons.cancel.tip")}
        shortcut={shortcuts.checkoutCancel.label}
        disabledReason={noItems}
        onClick={props.onCancel}
      />
      <ActionButton
        colour="grey"
        icon={Wallet}
        size="till"
        label={t("buttons.shift.label")}
        tip={t("buttons.shift.tip")}
        shortcut={shortcuts.checkoutShift.label}
        onClick={props.onShift}
      />
      <ActionButton
        colour="grey"
        icon={Lock}
        size="till"
        label={t("buttons.lock.label")}
        tip={t("buttons.lock.tip")}
        shortcut={shortcuts.lock.label}
        onClick={props.onLock}
      />
      <ActionButton
        colour="grey"
        icon={LayoutGrid}
        size="till"
        label={t("backOffice.label")}
        tip={t("backOffice.tip")}
        onClick={props.onBackOffice}
      />
    </nav>
  );
}
