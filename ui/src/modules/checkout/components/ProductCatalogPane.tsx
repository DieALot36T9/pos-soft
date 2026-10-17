import React, { useState, useEffect, memo } from "react";
import { Grid, Package, Search } from "lucide-react";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SaleHit } from "../../../generated/SaleHit";
import { formatMoney } from "../../../shared/format/money";
import { formatQty } from "../../../shared/format/qty";
import { searchProducts } from "../api";

type CategoryTab = "HIDDEN" | "ALL" | "WEIGHED" | "LOW_STOCK";

const quickTouchCatalogTitle = "Quick Touch Catalog";
const quickTouchCatalogIntro =
  "Scan any barcode above, or select a category tab (`All Product Tiles`, `⚖ Weighed / Fresh`) to browse touch tiles.";
const showProductTilesLabel = "Show Product Tiles";
const noMatchingTilesTitle = "No matching product tiles";
const noMatchingTilesIntro = "Scan any barcode above or clear the tile filter.";
const scaleIcon = "⚖";

export const ProductCatalogPane = memo(function ProductCatalogPane({
  currency,
  onPickProduct,
  scanBoxSlot,
  actionBarSlot,
}: {
  currency: CurrencyFormat;
  onPickProduct: (productId: string) => void;
  scanBoxSlot: React.ReactNode;
  actionBarSlot: React.ReactNode;
}) {
  const [quickFilter, setQuickFilter] = useState("");
  const [items, setItems] = useState<SaleHit[]>([]);
  const [activeCategory, setActiveCategory] = useState<CategoryTab>("HIDDEN");

  const isHidden = activeCategory === "HIDDEN" && quickFilter.trim() === "";

  useEffect(() => {
    if (isHidden) {
      return;
    }
    let cancelled = false;
    searchProducts(quickFilter.trim())
      .then((res) => {
        if (!cancelled) setItems(res);
      })
      .catch(() => {
        if (!cancelled) setItems([]);
      });
    return () => {
      cancelled = true;
    };
  }, [quickFilter, isHidden]);

  const displayedItems = isHidden ? [] : items;
  const visibleItems = displayedItems.filter((item) => {
    const isWeighed =
      item.unit_symbol.toLowerCase().includes("kg") || item.unit_symbol.toLowerCase().includes("g");
    if (activeCategory === "WEIGHED") return isWeighed;
    if (activeCategory === "LOW_STOCK")
      return Number(item.on_hand) <= 5000 && Number(item.on_hand) > 0;
    return true;
  });

  return (
    <section
      aria-label="Product catalog and barcode scanner"
      className="flex flex-col h-full bg-bg border-r border-border overflow-hidden"
    >
      {/* Top: Persistent Scan & Search Box */}
      <div className="p-3 bg-panel border-b border-border">{scanBoxSlot}</div>

      {/* Category & Quick Filter Pills */}
      <div className="px-3 py-2 bg-panel border-b border-border flex items-center justify-between gap-2 overflow-x-auto shrink-0">
        <div className="flex items-center gap-1.5">
          {(
            [
              { id: "ALL", label: "All Product Tiles" },
              { id: "WEIGHED", label: "⚖ Weighed / Fresh" },
              { id: "LOW_STOCK", label: "Low Stock" },
            ] as const
          ).map((tab) => {
            const active = activeCategory === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveCategory(active ? "HIDDEN" : tab.id)}
                title={`Show ${tab.label}`}
                className={`px-3 py-1.5 rounded-ui text-xs font-bold border cursor-pointer ${
                  active
                    ? "bg-blue text-white border-blue"
                    : "bg-bg text-text border-border hover:bg-grey-soft"
                }`}
              >
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>

        <div className="relative w-48 shrink-0">
          <Search className="w-3.5 h-3.5 text-text-muted absolute left-2.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={quickFilter}
            onChange={(e) => {
              setQuickFilter(e.target.value);
              if (activeCategory === "HIDDEN" && e.target.value.trim() !== "") {
                setActiveCategory("ALL");
              }
            }}
            placeholder="Filter tiles…"
            aria-label="Filter product tiles"
            className="w-full h-8 pl-8 pr-2 text-xs rounded-ui border border-border bg-bg text-text"
          />
        </div>
      </div>

      {/* Center: Touch-Friendly Product Tile Grid */}
      <div className="flex-1 p-3 overflow-y-auto">
        {isHidden ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-6 text-text-muted">
            <Grid className="w-8 h-8 mb-2 text-blue" />
            <p className="text-sm font-bold text-text">{quickTouchCatalogTitle}</p>
            <p className="text-xs mt-1 max-w-sm">{quickTouchCatalogIntro}</p>
            <button
              type="button"
              onClick={() => setActiveCategory("ALL")}
              className="mt-3 px-4 py-2 rounded-ui border border-blue bg-blue-soft text-blue text-xs font-bold cursor-pointer"
            >
              <span>{showProductTilesLabel}</span>
            </button>
          </div>
        ) : visibleItems.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-6 text-text-muted">
            <Package className="w-8 h-8 mb-2 opacity-60" />
            <p className="text-sm font-semibold">{noMatchingTilesTitle}</p>
            <p className="text-xs mt-0.5">{noMatchingTilesIntro}</p>
          </div>
        ) : (
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2.5">
            {visibleItems.map((hit) => {
              const stockRaw = Number(hit.on_hand);
              const isNegative = stockRaw <= 0;
              const isLow = stockRaw > 0 && stockRaw <= 5000;
              const isWeighed =
                hit.unit_symbol.toLowerCase().includes("kg") ||
                hit.unit_symbol.toLowerCase().includes("g");

              return (
                <button
                  key={hit.product_id}
                  type="button"
                  onClick={() => onPickProduct(hit.product_id)}
                  title={`Add ${hit.name} (${hit.sku}) to order`}
                  className="min-h-[96px] p-3 rounded-ui border border-border bg-panel hover:border-blue hover:bg-blue-soft/40 text-left flex flex-col justify-between cursor-pointer"
                >
                  <div>
                    <div className="flex items-start justify-between gap-1">
                      <span className="text-sm font-bold text-text line-clamp-2 leading-tight">
                        {hit.name}
                      </span>
                      {isWeighed && (
                        <span
                          title="Sold by weight"
                          className="px-1.5 py-0.5 rounded text-[11px] font-bold bg-blue-soft text-blue shrink-0"
                        >
                          {scaleIcon}
                        </span>
                      )}
                    </div>
                    <span className="text-[11px] font-mono text-text-muted mt-0.5 block">
                      {hit.sku}
                    </span>
                  </div>

                  <div className="flex items-end justify-between gap-2 mt-2 pt-1.5 border-t border-border/50">
                    <span className="text-sm font-extrabold text-text tabular-nums">
                      {formatMoney(hit.sale_price, currency)}
                    </span>
                    <span
                      className={`text-[11px] font-semibold px-1.5 py-0.5 rounded tabular-nums ${
                        isNegative
                          ? "bg-red-soft text-red"
                          : isLow
                            ? "bg-amber-soft text-amber"
                            : "bg-grey-soft text-text-muted"
                      }`}
                    >
                      {formatQty(hit.on_hand, isWeighed ? 3 : 0)} {hit.unit_symbol}
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>

      {/* Bottom: Compact Action Bar */}
      <div className="p-2.5 bg-panel border-t border-border shrink-0">{actionBarSlot}</div>
    </section>
  );
});
