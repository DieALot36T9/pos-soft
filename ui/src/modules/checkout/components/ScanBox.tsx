import { Search } from "lucide-react";
import { useEffect, useState, type KeyboardEvent, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SaleHit } from "../../../generated/SaleHit";
import { formatMoney } from "../../../shared/format/money";
import { formatQty } from "../../../shared/format/qty";
import { Tip } from "../../../shared/ui/Tip";
import { searchProducts } from "../api";

// Search-as-you-type waits this long after the last key (Frontend Architecture).
const SEARCH_WAIT_MS = 80;

type ScanBoxProps = {
  inputRef: RefObject<HTMLInputElement | null>;
  // Results from a scan of text that wasn't a code, shown in the same list.
  hits: SaleHit[];
  setHits: (hits: SaleHit[]) => void;
  currency: CurrencyFormat;
  // Enter with text: scan it. Enter on an empty box: open payment (PAY-01).
  onSubmit: (text: string) => void;
  onEmptyEnter: () => void;
  onPick: (productId: string) => void;
  // ↑/↓ and +/− for the cart while no list is open (Keyboard Shortcuts).
  onArrow: (step: 1 | -1) => void;
  onPlusMinus: (step: 1 | -1) => void;
  // Any key press clears the red message bar.
  onKey: () => void;
};

// The scan box at the top of the till, with the search list under it (CHK-01, CHK-03).
// It always has the focus when no dialog is open.
export function ScanBox(props: ScanBoxProps) {
  const { inputRef, hits, setHits, currency, onSubmit, onEmptyEnter, onPick } = props;
  const { t } = useTranslation("checkout");
  const [text, setText] = useState("");
  const [highlight, setHighlight] = useState(0);

  // Words are searched as they are typed; digits wait for Enter (a scanner types fast).
  useEffect(() => {
    const typed = text.trim();
    if (typed.length < 2 || /^[\d*]+$/.test(typed)) {
      return;
    }
    const timer = window.setTimeout(() => {
      searchProducts(typed)
        .then((found) => {
          setHits(found);
          setHighlight(0);
        })
        .catch(() => setHits([]));
    }, SEARCH_WAIT_MS);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- search again only when the text changes.
  }, [text]);

  const listOpen = hits.length > 0;
  const choose = (productId: string) => {
    setText("");
    setHits([]);
    onPick(productId);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    props.onKey();
    if (listOpen && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setHighlight((old) => Math.min(Math.max(old + step, 0), hits.length - 1));
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      props.onArrow(event.key === "ArrowDown" ? 1 : -1);
    } else if ((event.key === "+" || event.key === "-") && text === "") {
      event.preventDefault();
      props.onPlusMinus(event.key === "+" ? 1 : -1);
    } else if (event.key === "Escape") {
      setText("");
      setHits([]);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const chosen = hits[highlight];
      if (listOpen && chosen) {
        choose(chosen.product_id);
      } else if (text.trim() === "") {
        onEmptyEnter();
      } else {
        onSubmit(text);
        setText("");
      }
    }
  };

  return (
    <div className="relative">
      <label className="flex items-center gap-2 rounded-ui border border-border bg-panel px-3">
        <Search aria-hidden="true" size={24} className="text-blue" />
        <span className="sr-only">{t("scan.label")}</span>
        <Tip tip={t("scan.tip")}>
          <input
            ref={inputRef}
            value={text}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={onKeyDown}
            placeholder={t("scan.placeholder")}
            aria-label={t("scan.label")}
            autoComplete="off"
            className="min-h-14 w-full bg-transparent text-xl outline-none"
          />
        </Tip>
      </label>
      {listOpen ? (
        <ul
          aria-label={t("search.title")}
          className="absolute z-30 mt-1 w-full rounded-ui border border-border bg-panel"
        >
          {hits.map((hit, index) => (
            <li key={hit.product_id}>
              <Tip tip={t("search.pick", { name: hit.name })}>
                <button
                  type="button"
                  aria-current={index === highlight ? "true" : undefined}
                  onClick={() => choose(hit.product_id)}
                  className={
                    "flex w-full items-center gap-3 px-3 py-2 text-left " +
                    (index === highlight ? "bg-blue-soft text-blue" : "")
                  }
                >
                  {hit.thumbnail ? (
                    <img src={hit.thumbnail} alt="" className="h-10 w-10 rounded-ui border" />
                  ) : null}
                  <span className="flex-1 font-semibold">{hit.name}</span>
                  <span className="text-text-muted">
                    {t("search.onHand", {
                      qty: formatQty(hit.on_hand, hit.on_hand % 1000 === 0 ? 0 : 3),
                    })}
                  </span>
                  <span className="w-28 text-right tabular-nums">
                    {formatMoney(hit.sale_price, currency)}
                  </span>
                </button>
              </Tip>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
