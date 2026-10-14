import { useState, useEffect } from "react";
import { Hash, Percent, Tag, Trash2 } from "lucide-react";
import type { CartLineView } from "../../../generated/CartLineView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { formatMoney } from "../../../shared/format/money";
import { KitDrawer, KitButton, KitNumpad } from "../../../shared/ui/PosKit";

export type LineEditTab = "qty" | "discount" | "price";

const removeLineLabel = "Remove Line";
const applyChangeLabel = "Apply Change";
const qtyTabLabel = "Quantity (F4)";
const discountTabLabel = "Discount (F7)";
const priceTabLabel = "Price (F8)";
const percentageLabel = "Percentage (%)";
const fixedAmountLabel = "Fixed Amount";

export function LineEditDrawer({
  open,
  line,
  initialTab = "qty",
  currency,
  onClose,
  onApplyQty,
  onApplyPercentDiscount,
  onApplyAmountDiscount,
  onApplyPriceOverride,
  onRemoveLine,
}: {
  open: boolean;
  line: CartLineView | null;
  initialTab?: LineEditTab;
  currency: CurrencyFormat;
  onClose: () => void;
  onApplyQty: (qtyText: string) => void;
  onApplyPercentDiscount: (pctText: string) => void;
  onApplyAmountDiscount: (amountMinorText: string) => void;
  onApplyPriceOverride: (priceMinorText: string) => void;
  onRemoveLine: () => void;
}) {
  const [tab, setTab] = useState<LineEditTab>(initialTab);
  const [discountKind, setDiscountKind] = useState<"pct" | "amt">("pct");
  const [inputVal, setInputVal] = useState("");

  useEffect(() => {
    setTimeout(() => {
      setTab(initialTab);
      setInputVal("");
    }, 0);
  }, [initialTab, line, open]);

  if (!line) return null;

  const handleApply = () => {
    if (!inputVal.trim()) return;
    if (tab === "qty") {
      onApplyQty(inputVal);
    } else if (tab === "discount") {
      if (discountKind === "pct") onApplyPercentDiscount(inputVal);
      else onApplyAmountDiscount(inputVal);
    } else if (tab === "price") {
      onApplyPriceOverride(inputVal);
    }
    onClose();
  };

  return (
    <KitDrawer
      open={open}
      onClose={onClose}
      widthClass="max-w-md"
      title={line.name}
      description={`SKU: ${line.sku} · Unit price: ${formatMoney(line.unit_price, currency)}`}
      footer={
        <>
          <KitButton
            tone="red"
            icon={Trash2}
            shortcut="Del"
            tip="Remove this item from the current sale"
            onClick={() => {
              onRemoveLine();
              onClose();
            }}
          >
            <span>{removeLineLabel}</span>
          </KitButton>
          <KitButton
            tone="green"
            shortcut="Enter"
            tip="Apply change to selected cart line"
            onClick={handleApply}
            disabled={!inputVal.trim()}
          >
            <span>{applyChangeLabel}</span>
          </KitButton>
        </>
      }
    >
      {/* Mode Switcher Tabs */}
      <div className="grid grid-cols-3 gap-1.5 p-1 bg-bg rounded-[6px] border border-border">
        <button
          type="button"
          onClick={() => {
            setTab("qty");
            setInputVal("");
          }}
          className={`py-2 px-2 rounded text-xs font-bold flex items-center justify-center gap-1.5 cursor-pointer ${
            tab === "qty" ? "bg-blue text-white" : "text-text hover:bg-grey-soft"
          }`}
        >
          <Hash className="w-3.5 h-3.5" /> <span>{qtyTabLabel}</span>
        </button>
        <button
          type="button"
          onClick={() => {
            setTab("discount");
            setInputVal("");
          }}
          className={`py-2 px-2 rounded text-xs font-bold flex items-center justify-center gap-1.5 cursor-pointer ${
            tab === "discount" ? "bg-purple text-white" : "text-text hover:bg-grey-soft"
          }`}
        >
          <Percent className="w-3.5 h-3.5" /> <span>{discountTabLabel}</span>
        </button>
        <button
          type="button"
          onClick={() => {
            setTab("price");
            setInputVal("");
          }}
          className={`py-2 px-2 rounded text-xs font-bold flex items-center justify-center gap-1.5 cursor-pointer ${
            tab === "price" ? "bg-purple text-white" : "text-text hover:bg-grey-soft"
          }`}
        >
          <Tag className="w-3.5 h-3.5" /> <span>{priceTabLabel}</span>
        </button>
      </div>

      {tab === "discount" && (
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => setDiscountKind("pct")}
            className={`py-1.5 text-xs font-bold rounded border cursor-pointer ${
              discountKind === "pct"
                ? "bg-purple-soft text-purple border-purple"
                : "border-border text-text-muted hover:bg-grey-soft"
            }`}
          >
            <span>{percentageLabel}</span>
          </button>
          <button
            type="button"
            onClick={() => setDiscountKind("amt")}
            className={`py-1.5 text-xs font-bold rounded border cursor-pointer ${
              discountKind === "amt"
                ? "bg-purple-soft text-purple border-purple"
                : "border-border text-text-muted hover:bg-grey-soft"
            }`}
          >
            <span>{fixedAmountLabel}</span>
          </button>
        </div>
      )}

      {/* Active Value Display */}
      <div className="p-3 rounded-[6px] border border-border bg-bg flex items-center justify-between">
        <span className="text-xs font-semibold text-text-muted">
          {tab === "qty"
            ? `New Quantity (${line.unit_symbol})`
            : tab === "discount"
              ? discountKind === "pct"
                ? "Discount Percentage (%)"
                : "Discount Amount"
              : "New Unit Price"}
        </span>
        <input
          type="text"
          value={inputVal}
          onChange={(e) => setInputVal(e.target.value)}
          placeholder="0"
          autoFocus
          className="w-36 text-right text-2xl font-extrabold text-text bg-transparent tabular-nums focus:outline-none"
        />
      </div>

      {/* Integrated Touch Numpad */}
      <KitNumpad
        value={inputVal}
        onChange={setInputVal}
        onEnter={handleApply}
        allowDecimal={true}
        quickValues={
          tab === "qty"
            ? [
                { label: "1", value: "1" },
                { label: "2", value: "2" },
                { label: "5", value: "5" },
                { label: "10", value: "10" },
              ]
            : tab === "discount" && discountKind === "pct"
              ? [
                  { label: "5%", value: "5" },
                  { label: "10%", value: "10" },
                  { label: "15%", value: "15" },
                  { label: "20%", value: "20" },
                ]
              : undefined
        }
      />
    </KitDrawer>
  );
}
