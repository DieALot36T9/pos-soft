import {
  BadgePercent,
  Hash,
  PenLine,
  Percent,
  Tags,
  User,
  ListRestart,
  Lock,
  PauseCircle,
  Printer,
  ReceiptText,
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
  // True when at least one price list is switched on (CHK-17).
  hasPriceLists: boolean;
  heldCount: number;
  // Shows "Old receipt" only to users who may reprint any receipt (PAY-10).
  canReprintAny: boolean;
  onSearch: () => void;
  onCustomer: () => void;
  onQty: () => void;
  onDiscount: () => void;
  onInvoiceDiscount: () => void;
  onPrice: () => void;
  onPriceList: () => void;
  onHold: () => void;
  onHeld: () => void;
  onRemove: () => void;
  onCancel: () => void;
  onReprint: () => void;
  onOldReceipt: () => void;
  onShift: () => void;
  onLock: () => void;
  onBackOffice: () => void;
};

// The buttons under the cart, each with its icon, word, colour and key (Checkout Screen).
// Returns come in a later milestone (C-M9).
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
        colour="blue"
        icon={User}
        size="till"
        label={t("buttons.customer.label")}
        tip={t("buttons.customer.tip")}
        shortcut={shortcuts.checkoutCustomer.label}
        onClick={props.onCustomer}
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
        colour="purple"
        icon={Percent}
        size="till"
        label={t("buttons.discount.label")}
        tip={t("buttons.discount.tip")}
        shortcut={shortcuts.checkoutDiscount.label}
        disabledReason={noSelection}
        onClick={props.onDiscount}
      />
      <ActionButton
        colour="purple"
        icon={BadgePercent}
        size="till"
        label={t("buttons.invoiceDiscount.label")}
        tip={t("buttons.invoiceDiscount.tip")}
        shortcut={shortcuts.checkoutInvoiceDiscount.label}
        disabledReason={noItems}
        onClick={props.onInvoiceDiscount}
      />
      <ActionButton
        colour="purple"
        icon={PenLine}
        size="till"
        label={t("buttons.price.label")}
        tip={t("buttons.price.tip")}
        shortcut={shortcuts.checkoutPrice.label}
        disabledReason={noSelection}
        onClick={props.onPrice}
      />
      <ActionButton
        colour="purple"
        icon={Tags}
        size="till"
        label={t("buttons.priceList.label")}
        tip={t("buttons.priceList.tip")}
        disabledReason={props.hasPriceLists ? undefined : t("buttons.priceList.none")}
        onClick={props.onPriceList}
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
        colour="blue"
        icon={Printer}
        size="till"
        label={t("buttons.reprint.label")}
        tip={t("buttons.reprint.tip")}
        shortcut={shortcuts.checkoutReprint.label}
        onClick={props.onReprint}
      />
      {props.canReprintAny ? (
        <ActionButton
          colour="blue"
          icon={ReceiptText}
          size="till"
          label={t("buttons.oldReceipt.label")}
          tip={t("buttons.oldReceipt.tip")}
          onClick={props.onOldReceipt}
        />
      ) : null}
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
