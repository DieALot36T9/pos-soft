import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CartView } from "../../generated/CartView";
import type { CheckoutView } from "../../generated/CheckoutView";
import type { SaleDone } from "../../generated/SaleDone";
import type { SaleHit } from "../../generated/SaleHit";
import type { ScanResult } from "../../generated/ScanResult";
import type { WeighProduct } from "../../generated/WeighProduct";
import { errorMessage } from "../../shared/bridge";
import { addProduct, loadCheckout, removeLine, scanInput, setQty } from "./api";
import { beepGood, beepProblem } from "./sounds";

// How long a new line's background flashes (Design System: 0.3 s, the only movement).
const FLASH_MS = 300;
// How long the red message bar stays when no key is pressed (Checkout Screen: 3 s).
const MESSAGE_MS = 3000;

// The checkout screen's state: the whole view from Rust, the selected line, and the
// last problem. Every cart action sends the change to Rust and shows the cart it returns.
export function useTill(reloadKey: number, justLoggedIn: boolean) {
  const { t: tErrors } = useTranslation("errors");
  const [view, setView] = useState<CheckoutView | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [flash, setFlash] = useState<string | null>(null);
  const [priceChanged, setPriceChanged] = useState<string[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [hits, setHits] = useState<SaleHit[]>([]);
  const [weighing, setWeighing] = useState<WeighProduct | null>(null);
  const [lastSale, setLastSale] = useState<SaleDone | null>(null);
  const [loadFailed, setLoadFailed] = useState<string | null>(null);
  // "Your previous sale was restored" (CHK-19): a cart with items right after login.
  const [restored, setRestored] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    loadCheckout()
      .then((loaded) => {
        setView(loaded);
        setLoadFailed(null);
        const lines = loaded.cart?.lines ?? [];
        setSelected(lines.at(-1)?.line_id ?? null);
        setRestored(justLoggedIn && lines.length > 0);
      })
      .catch((error: unknown) => setLoadFailed(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load again only when asked.
  }, [reloadKey]);

  const sounds = view?.sounds ?? false;

  const fail = (error: unknown) => {
    if (sounds) {
      beepProblem();
    }
    setProblem(errorMessage(error, tErrors));
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setProblem(null), MESSAGE_MS);
  };

  // Shows a new cart from Rust. `lineId` is the line just added or changed.
  const showCart = (cart: CartView, lineId: string | null = null) => {
    setView((old) => (old ? { ...old, cart } : old));
    setProblem(null);
    setLastSale(null);
    const ids = cart.lines.map((line) => line.line_id);
    const keep = lineId ?? selected;
    setSelected(keep !== null && ids.includes(keep) ? keep : (ids.at(-1) ?? null));
    if (lineId !== null) {
      setFlash(lineId);
      window.setTimeout(() => setFlash(null), FLASH_MS);
    }
  };

  const handleScan = (result: ScanResult) => {
    setHits([]);
    switch (result.kind) {
      case "added":
        if (sounds) {
          beepGood();
        }
        showCart(result.cart, result.line_id);
        break;
      case "needs_weight":
        setWeighing(result.product);
        break;
      case "search":
        setHits(result.hits);
        break;
    }
  };

  const scan = (input: string) => scanInput(input).then(handleScan).catch(fail);
  const pick = (productId: string) => addProduct(productId).then(handleScan).catch(fail);
  const changeQty = (lineId: string, qty: number) =>
    setQty(lineId, qty)
      .then((cart) => showCart(cart, qty > 0 ? lineId : null))
      .catch(fail);
  const remove = (lineId: string) =>
    removeLine(lineId)
      .then((cart) => showCart(cart))
      .catch(fail);

  // Moves the blue selection up or down the cart (arrow keys).
  const moveSelection = (step: 1 | -1) => {
    const lines = view?.cart?.lines ?? [];
    const index = lines.findIndex((line) => line.line_id === selected);
    const next = lines[Math.min(Math.max(index + step, 0), lines.length - 1)];
    if (next) {
      setSelected(next.line_id);
    }
  };

  const saleDone = (done: SaleDone) => {
    setView((old) => (old ? { ...old, cart: done.cart } : old));
    setSelected(null);
    setPriceChanged([]);
    setLastSale(done);
  };

  return {
    view,
    setView,
    restored,
    setRestored,
    loadFailed,
    selected,
    setSelected,
    flash,
    priceChanged,
    setPriceChanged,
    problem,
    clearProblem: () => setProblem(null),
    fail,
    hits,
    setHits,
    weighing,
    setWeighing,
    lastSale,
    showCart,
    scan,
    pick,
    changeQty,
    remove,
    moveSelection,
    saleDone,
  };
}

export type Till = ReturnType<typeof useTill>;
