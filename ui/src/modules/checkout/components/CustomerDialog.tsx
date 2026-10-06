import { UserMinus, UserPlus, X } from "lucide-react";
import { useEffect, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { CustomerRow } from "../../../generated/CustomerRow";
import { errorField, errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { Tip } from "../../../shared/ui/Tip";
import { findCustomers, quickAddCustomer } from "../api";

type CustomerDialogProps = {
  currency: CurrencyFormat;
  // True when the sale already has a customer, so "No customer" is offered.
  hasCustomer: boolean;
  onPick: (customerId: string | null) => void;
  onClose: () => void;
};

// F3: find the customer by phone or name, or add them with just a phone and a name
// (CUS-02). Picking one puts them on the sale; their tier applies at once (CHK-11).
export function CustomerDialog({ currency, hasCustomer, onPick, onClose }: CustomerDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const [text, setText] = useState("");
  const [found, setHits] = useState<CustomerRow[]>([]);
  const [chosen, setChosen] = useState(0);
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [badField, setBadField] = useState<string | null>(null);

  // Rust searches after every key; the list shows at most 20 matches.
  const searching = text.trim() !== "";
  useEffect(() => {
    if (!searching) {
      return;
    }
    findCustomers(text)
      .then((found) => {
        setHits(found);
        setChosen(0);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- search again only when the text changes.
  }, [text]);

  // Nothing typed: no list, whatever the last search found.
  const hits = searching ? found : [];
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setChosen((old) => Math.min(Math.max(old + step, 0), Math.max(hits.length - 1, 0)));
    } else if (event.key === "Enter") {
      const hit = hits[chosen];
      if (hit) {
        onPick(hit.id);
      }
    } else if (event.key === "Escape") {
      onClose();
    }
  };

  const add = () => {
    setProblem(null);
    setBadField(null);
    const input = {
      id: null,
      details: {
        phone,
        name,
        email: null,
        address: null,
        tier_id: null,
        credit_limit: 0,
        loyalty_opt_out: false,
        notes: null,
      },
    };
    quickAddCustomer(input)
      .then(onPick)
      .catch((error: unknown) => {
        setProblem(errorMessage(error, tErrors));
        setBadField(errorField(error));
      });
  };
  const fieldError = (field: string) => (badField === field ? problem : null);

  return (
    <Dialog title={t("customer.title")} colour="blue" width="wide">
      <Field
        label={t("customer.find.label")}
        tip={t("customer.find.tip")}
        value={text}
        onChange={setText}
        onKeyDown={onKeyDown}
        autoFocus
      />
      {text.trim() !== "" && hits.length === 0 ? (
        <p className="text-text-muted">{t("customer.none")}</p>
      ) : null}
      {hits.length > 0 ? (
        <ul role="listbox" aria-label={t("customer.title")} className="flex flex-col gap-1">
          {hits.map((hit, index) => (
            <li key={hit.id} role="option" aria-selected={index === chosen}>
              <Tip tip={t("customer.pickTip", { name: hit.name })}>
                <button
                  type="button"
                  onClick={() => onPick(hit.id)}
                  className={
                    "flex w-full justify-between gap-3 rounded-ui border px-3 py-2 text-left " +
                    (index === chosen ? "border-blue bg-blue-soft" : "border-border")
                  }
                >
                  <span>
                    {t("customer.hit", {
                      name: hit.name,
                      phone: hit.phone,
                      tier: hit.tier_name ?? "",
                    })}
                  </span>
                  <span className="text-text-muted tabular-nums">
                    {hit.credit > 0
                      ? t("customer.pointsOwes", {
                          points: hit.points,
                          amount: formatMoney(hit.credit, currency),
                        })
                      : t("customer.points", { points: hit.points })}
                  </span>
                </button>
              </Tip>
            </li>
          ))}
        </ul>
      ) : null}
      <section className="flex flex-col gap-2 border-t border-border pt-3">
        <h3 className="font-semibold">{t("customer.add.title")}</h3>
        <div className="grid grid-cols-2 gap-2">
          <Field
            label={t("customer.add.name.label")}
            tip={t("customer.add.name.tip")}
            value={name}
            onChange={setName}
            error={fieldError("name")}
          />
          <Field
            label={t("customer.add.phone.label")}
            tip={t("customer.add.phone.tip")}
            value={phone}
            onChange={setPhone}
            error={fieldError("phone")}
          />
        </div>
      </section>
      {problem && badField === null ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex justify-between gap-2">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("customer.back.label")}
          tip={t("customer.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        {hasCustomer ? (
          <ActionButton
            colour="red"
            icon={UserMinus}
            label={t("customer.remove.label")}
            tip={t("customer.remove.tip")}
            onClick={() => onPick(null)}
          />
        ) : null}
        <ActionButton
          colour="blue"
          icon={UserPlus}
          label={t("customer.add.save.label")}
          tip={t("customer.add.save.tip")}
          onClick={add}
        />
      </div>
    </Dialog>
  );
}
