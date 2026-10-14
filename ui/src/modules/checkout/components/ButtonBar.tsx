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
  RotateCcw,
  Search,
  Trash2,
  Wallet,
  XCircle,
  LayoutGrid,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";

type ButtonBarProps = {
  hasLines: boolean;
  hasSelection: boolean;
  hasPriceLists: boolean;
  heldCount: number;
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
  onReturn: () => void;
  onShift: () => void;
  onLock: () => void;
  onBackOffice: () => void;
};

export function ButtonBar(props: ButtonBarProps) {
  const { t } = useTranslation("checkout");
  const [showMore, setShowMore] = useState(true);

  const noItems = props.hasLines ? undefined : t("pay.empty");
  const noSelection = props.hasSelection ? undefined : t("buttons.qty.none");
  const moreLabel = "More";

  return (
    <nav aria-label="Till actions" className="flex flex-col gap-2 bg-panel">
      {/* Primary 5 Buttons + More Toggle */}
      <div className="flex flex-wrap items-center gap-2">
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
          colour="red"
          icon={RotateCcw}
          size="till"
          label={t("buttons.returns.label")}
          tip={t("buttons.returns.tip")}
          shortcut={shortcuts.checkoutReturn.label}
          onClick={props.onReturn}
        />

        <button
          type="button"
          onClick={() => setShowMore((prev) => !prev)}
          title="Toggle secondary actions (More actions menu)"
          className="min-h-12 px-3 py-2 rounded-[6px] border border-border bg-bg hover:bg-grey-soft text-text text-sm font-semibold flex items-center gap-1.5 cursor-pointer ml-auto"
        >
          <span>{moreLabel}</span>
          {showMore ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
        </button>
      </div>

      {/* Secondary Actions Row / Drawer */}
      {showMore && (
        <div className="flex flex-wrap items-center gap-1.5 pt-2 border-t border-border/60">
          <ActionButton
            colour="blue"
            icon={Search}
            size="office"
            label={t("buttons.search.label")}
            tip={t("buttons.search.tip")}
            shortcut={shortcuts.checkoutSearch.label}
            onClick={props.onSearch}
          />
          <ActionButton
            colour="grey"
            icon={Hash}
            size="office"
            label={t("buttons.qty.label")}
            tip={t("buttons.qty.tip")}
            shortcut={shortcuts.checkoutQty.label}
            disabledReason={noSelection}
            onClick={props.onQty}
          />
          <ActionButton
            colour="purple"
            icon={BadgePercent}
            size="office"
            label={t("buttons.invoiceDiscount.label")}
            tip={t("buttons.invoiceDiscount.tip")}
            shortcut={shortcuts.checkoutInvoiceDiscount.label}
            disabledReason={noItems}
            onClick={props.onInvoiceDiscount}
          />
          <ActionButton
            colour="purple"
            icon={PenLine}
            size="office"
            label={t("buttons.price.label")}
            tip={t("buttons.price.tip")}
            shortcut={shortcuts.checkoutPrice.label}
            disabledReason={noSelection}
            onClick={props.onPrice}
          />
          <ActionButton
            colour="purple"
            icon={Tags}
            size="office"
            label={t("buttons.priceList.label")}
            tip={t("buttons.priceList.tip")}
            disabledReason={props.hasPriceLists ? undefined : t("buttons.priceList.none")}
            onClick={props.onPriceList}
          />
          <ActionButton
            colour="red"
            icon={Trash2}
            size="office"
            label={t("buttons.remove.label")}
            tip={t("buttons.remove.tip")}
            shortcut={shortcuts.checkoutRemove.label}
            disabledReason={noSelection}
            onClick={props.onRemove}
          />
          <ActionButton
            colour="red"
            icon={XCircle}
            size="office"
            label={t("buttons.cancel.label")}
            tip={t("buttons.cancel.tip")}
            shortcut={shortcuts.checkoutCancel.label}
            disabledReason={noItems}
            onClick={props.onCancel}
          />
          <ActionButton
            colour="blue"
            icon={Printer}
            size="office"
            label={t("buttons.reprint.label")}
            tip={t("buttons.reprint.tip")}
            shortcut={shortcuts.checkoutReprint.label}
            onClick={props.onReprint}
          />
          {props.canReprintAny ? (
            <ActionButton
              colour="blue"
              icon={ReceiptText}
              size="office"
              label={t("buttons.oldReceipt.label")}
              tip={t("buttons.oldReceipt.tip")}
              onClick={props.onOldReceipt}
            />
          ) : null}
          <ActionButton
            colour="grey"
            icon={Wallet}
            size="office"
            label={t("buttons.shift.label")}
            tip={t("buttons.shift.tip")}
            shortcut={shortcuts.checkoutShift.label}
            onClick={props.onShift}
          />
          <ActionButton
            colour="grey"
            icon={Lock}
            size="office"
            label={t("buttons.lock.label")}
            tip={t("buttons.lock.tip")}
            shortcut={shortcuts.lock.label}
            onClick={props.onLock}
          />
          <ActionButton
            colour="grey"
            icon={LayoutGrid}
            size="office"
            label={t("backOffice.label")}
            tip={t("backOffice.tip")}
            onClick={props.onBackOffice}
          />
        </div>
      )}
    </nav>
  );
}
