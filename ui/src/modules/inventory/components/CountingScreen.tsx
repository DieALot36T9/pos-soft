import { ChevronLeft, ListChecks, Plus } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CountView } from "../../../generated/CountView";
import type { StockProduct } from "../../../generated/StockProduct";
import { errorMessage } from "../../../shared/bridge";
import { formatQty, parseQty } from "../../../shared/format/qty";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { countLine, getCount, lookupStock } from "../api";
import { CountStatusBadge } from "./CountStatusBadge";
import { VarianceReview } from "./VarianceReview";

type CountingScreenProps = {
  countId: string;
  canManage: boolean;
  onBack: () => void;
};

export function CountingScreen({ countId, canManage, onBack }: CountingScreenProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [view, setView] = useState<CountView | null>(null);
  const [code, setCode] = useState("");
  const [qtyText, setQtyText] = useState("1");
  const [takeBack, setTakeBack] = useState(false);
  const [waiting, setWaiting] = useState<StockProduct | null>(null);
  const [batch, setBatch] = useState("");
  const [reviewing, setReviewing] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const codeBox = useRef<HTMLDivElement>(null);

  useHotkeys(shortcuts.back.keys, () => (reviewing ? setReviewing(false) : onBack()), {
    enableOnFormTags: true,
  });

  const load = () => {
    getCount(countId)
      .then(setView)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once for this count.
  useEffect(load, [countId]);

  const focusCode = () => codeBox.current?.querySelector("input")?.focus();

  const add = (product: StockProduct, batchId: string | null) => {
    const parsed = parseQty(qtyText, product.unit_decimals);
    if (!parsed.ok) {
      setProblem(tErrors(parsed.code as "qty_invalid"));
      return;
    }
    countLine(countId, {
      product_id: product.product_id,
      batch_id: batchId,
      qty: takeBack ? -parsed.milli : parsed.milli,
    })
      .then((answer) => {
        setView(answer);
        setDone(
          t("counting.added", {
            qty: (takeBack ? "−" : "") + formatQty(parsed.milli, product.unit_decimals),
            unit: product.unit_symbol,
            name: product.name,
          }),
        );
        setProblem(null);
        setWaiting(null);
        setCode("");
        setQtyText("1");
        setTakeBack(false);
        focusCode();
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  const scan = () => {
    if (code.trim() === "") {
      return;
    }
    lookupStock(code)
      .then((product) => {
        if (product.batches.length === 0) {
          add(product, null);
        } else {
          setWaiting(product);
          setBatch("");
          setDone(null);
          setProblem(null);
        }
      })
      .catch((error: unknown) => {
        setDone(null);
        setProblem(errorMessage(error, tErrors));
      });
  };

  if (!view) {
    return problem ? <Alert kind="problem">{problem}</Alert> : null;
  }
  if (reviewing) {
    return (
      <VarianceReview
        countId={countId}
        onBack={() => {
          setReviewing(false);
          load();
        }}
      />
    );
  }
  const isOpen = view.count.status === "open";

  return (
    <section aria-labelledby="counting-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3">
          <h2 id="counting-heading" className="text-[28px] font-semibold">
            {t("counting.heading", { number: view.count.number, name: view.count.name })}
          </h2>
          <CountStatusBadge status={view.count.status} />
        </div>
        <div className="flex gap-2">
          {canManage ? (
            <ActionButton
              colour="blue"
              icon={ListChecks}
              label={t("counting.review.label")}
              tip={t("counting.review.tip")}
              onClick={() => setReviewing(true)}
            />
          ) : null}
          <ActionButton
            colour="grey"
            icon={ChevronLeft}
            label={t("counting.back.label")}
            tip={t("counting.back.tip")}
            shortcut={shortcuts.back.label}
            onClick={onBack}
          />
        </div>
      </div>
      {isOpen ? (
        <div className="flex flex-col gap-4 rounded-ui border border-border bg-panel p-5">
          <div className="grid grid-cols-1 md:grid-cols-[2fr_1fr_auto] items-end gap-4">
            <div ref={codeBox}>
              <Field
                label={t("counting.code.label")}
                tip={t("counting.code.tip")}
                value={code}
                onChange={setCode}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    scan();
                  }
                }}
              />
            </div>
            <Field
              label={t("counting.qty.label")}
              tip={t("counting.qty.tip")}
              value={qtyText}
              onChange={setQtyText}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  scan();
                }
              }}
            />
            <div className="pb-2">
              <CheckBox
                label={t("counting.takeBack.label")}
                tip={t("counting.takeBack.tip")}
                checked={takeBack}
                onChange={setTakeBack}
              />
            </div>
          </div>
          {waiting ? (
            <div className="flex flex-col gap-3 rounded-ui border border-blue bg-blue-soft/30 p-3">
              <p className="font-semibold text-blue">
                {t("counting.found", { name: waiting.name, sku: waiting.sku })}
              </p>
              <div className="grid grid-cols-1 md:grid-cols-[2fr_auto] items-end gap-3">
                <SelectField
                  label={t("counting.batch.label")}
                  tip={t("counting.batch.tip")}
                  value={batch}
                  options={[
                    { value: "", label: t("counting.batch.whole") },
                    ...waiting.batches.map((each) => ({
                      value: each.id,
                      label: t("counting.batch.one", {
                        batch: each.batch_no ?? "—",
                        expiry: each.expiry_date ?? "—",
                      }),
                    })),
                  ]}
                  onChange={setBatch}
                />
                <ActionButton
                  colour="green"
                  icon={Plus}
                  label={t("counting.add.label")}
                  tip={t("counting.add.tip")}
                  onClick={() => add(waiting, batch === "" ? null : batch)}
                />
              </div>
            </div>
          ) : null}
        </div>
      ) : (
        <Alert kind="info">{t("counting.closed")}</Alert>
      )}
      {done ? <Alert kind="done">{done}</Alert> : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {view.lines.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("counting.table.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("counting.table.name")}</th>
              <th className="p-2.5">{t("counting.table.sku")}</th>
              <th className="p-2.5">{t("counting.table.batch")}</th>
              <th className="p-2.5 text-right">{t("counting.table.counted")}</th>
            </tr>
          </thead>
          <tbody>
            {view.lines.map((line) => (
              <tr
                key={`${line.product_id}-${line.batch_id ?? ""}`}
                className="border-b border-border hover:bg-blue-soft/30"
              >
                <td className="p-2.5 font-semibold">{line.name}</td>
                <td className="p-2.5 font-mono text-sm text-text-muted">{line.sku}</td>
                <td className="p-2.5 font-mono text-sm">
                  {line.batch_no ?? (line.batch_id ? "—" : "")}
                </td>
                <td className="p-2.5 text-right font-bold tabular-nums">
                  {t("qtyUnit", {
                    qty: formatQty(line.counted, line.unit_decimals),
                    unit: line.unit_symbol,
                  })}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
