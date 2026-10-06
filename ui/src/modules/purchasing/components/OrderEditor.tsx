import { ArrowLeft, Lightbulb, Save, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SupplierList } from "../../../generated/SupplierList";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { formatQty, parseQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { Tip } from "../../../shared/ui/Tip";
import { getOrder, listSuppliers, saveOrder, suggestLines } from "../api";
import { ProductPicker } from "./ProductPicker";

// One line as typed: the product and the quantity and cost as text.
type EditLine = { productId: string; name: string; qty: string; cost: string };

type OrderEditorProps = {
  // None: a new order.
  orderId: string | null;
  onSaved: (id: string) => void;
  onBack: () => void;
};

// A draft purchase order: supplier, expected day, notes and lines, with
// "Suggest from low stock" (PUR-02). Rust saves it and copies names from the catalog.
export function OrderEditor({ orderId, onSaved, onBack }: OrderEditorProps) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const save = useSave();
  const [suppliers, setSuppliers] = useState<SupplierList | null>(null);
  const [number, setNumber] = useState<string | null>(null);
  const [supplierId, setSupplierId] = useState("");
  const [expected, setExpected] = useState("");
  const [notes, setNotes] = useState("");
  const [lines, setLines] = useState<EditLine[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(() => {
    listSuppliers(false).then(setSuppliers).catch(fail);
    if (orderId !== null) {
      getOrder(orderId)
        .then((view) => {
          setNumber(view.order.number);
          setSupplierId(view.order.supplier_id);
          setExpected(view.order.expected_date ?? "");
          setNotes(view.order.notes ?? "");
          setLines(
            view.order.lines.map((line) => ({
              productId: line.product_id,
              name: line.product_name,
              qty: qtyText(line.qty),
              cost: plain(line.unit_cost, view.currency),
            })),
          );
        })
        .catch(fail);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, [orderId]);

  if (suppliers === null) {
    return problem ? <Alert kind="problem">{problem}</Alert> : <p>{t("common.loading")}</p>;
  }
  const currency = suppliers.currency;
  const chosenSupplier = supplierId || suppliers.rows[0]?.supplier.id || "";
  const typed = lines.map((line) => ({
    product_id: line.productId,
    qty: parseQty(line.qty, 3),
    cost: parseMoney(line.cost, currency.minor_digits),
  }));
  const typingOk = typed.every((line) => line.qty.ok && line.cost.ok);
  const change = (index: number, patch: Partial<EditLine>) =>
    setLines((old) => old.map((line, at) => (at === index ? { ...line, ...patch } : line)));

  const suggest = () => {
    setNotice(null);
    suggestLines(chosenSupplier)
      .then((found) => {
        if (found.length === 0) {
          setNotice(t("orders.editor.suggestNone"));
          return;
        }
        setLines((old) => [
          ...old,
          ...found
            .filter((each) => !old.some((line) => line.productId === each.product_id))
            .map((each) => ({
              productId: each.product_id,
              name: each.product_name,
              qty: qtyText(each.qty),
              cost: plain(each.unit_cost, currency),
            })),
        ]);
      })
      .catch(fail);
  };
  const submit = () => {
    if (!typingOk) {
      return;
    }
    save.run(
      () =>
        saveOrder({
          id: orderId,
          supplier_id: chosenSupplier,
          expected_date: expected.trim() === "" ? null : expected.trim(),
          notes: notes.trim() === "" ? null : notes.trim(),
          lines: typed.map((line) => ({
            product_id: line.product_id,
            qty: line.qty.ok ? line.qty.milli : 0,
            unit_cost: line.cost.ok ? line.cost.minor : 0,
          })),
        }),
      onSaved,
    );
  };
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <ActionButton
        colour="grey"
        icon={ArrowLeft}
        label={t("common.back.label")}
        tip={t("common.back.tip")}
        onClick={onBack}
      />
      <h3 className="text-xl font-semibold">
        {number === null
          ? t("orders.editor.newHeading")
          : t("orders.editor.editHeading", { number })}
      </h3>
      <div className="grid grid-cols-3 gap-3">
        <SelectField
          label={t("common.supplier.label")}
          tip={t("common.supplier.tip")}
          value={chosenSupplier}
          options={suppliers.rows.map(({ supplier }) => ({
            value: supplier.id,
            label: supplier.name,
          }))}
          onChange={setSupplierId}
        />
        <Field
          label={t("orders.editor.expected.label")}
          tip={t("orders.editor.expected.tip")}
          value={expected}
          onChange={setExpected}
        />
        <Field
          label={t("orders.editor.notes.label")}
          tip={t("orders.editor.notes.tip")}
          value={notes}
          onChange={setNotes}
        />
      </div>
      <div>
        <ActionButton
          colour="blue"
          icon={Lightbulb}
          label={t("orders.editor.suggest.label")}
          tip={t("orders.editor.suggest.tip")}
          disabledReason={chosenSupplier === "" ? t("common.supplier.choose") : undefined}
          onClick={suggest}
        />
      </div>
      {notice ? <Alert kind="info">{notice}</Alert> : null}
      {lines.length > 0 ? (
        <table className="w-full border-collapse">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("orders.editor.product")}</th>
              <th className="p-2">{t("orders.detail.ordered")}</th>
              <th className="p-2">{t("orders.detail.cost")}</th>
              <th className="p-2" />
            </tr>
          </thead>
          <tbody>
            {lines.map((line, index) => (
              <tr key={line.productId} className="border-b border-border">
                <td className="p-2">{line.name}</td>
                <td className="p-2">
                  <Tip tip={t("orders.editor.qty.tip")}>
                    <input
                      aria-label={t("orders.editor.qty.label", { name: line.name })}
                      value={line.qty}
                      onChange={(event) => change(index, { qty: event.target.value })}
                      className="min-h-10 w-28 rounded-ui border border-border bg-panel px-3 tabular-nums"
                    />
                  </Tip>
                </td>
                <td className="p-2">
                  <Tip tip={t("orders.editor.cost.tip")}>
                    <input
                      aria-label={t("orders.editor.cost.label", { name: line.name })}
                      value={line.cost}
                      onChange={(event) => change(index, { cost: event.target.value })}
                      className="min-h-10 w-32 rounded-ui border border-border bg-panel px-3 tabular-nums"
                    />
                  </Tip>
                </td>
                <td className="p-2">
                  <ActionButton
                    colour="red"
                    icon={Trash2}
                    label={t("common.remove.label")}
                    tip={t("common.remove.tip")}
                    onClick={() => setLines((old) => old.filter((_, at) => at !== index))}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      <ProductPicker
        onPick={(hit) =>
          setLines((old) =>
            old.some((line) => line.productId === hit.product_id)
              ? old
              : [...old, { productId: hit.product_id, name: hit.name, qty: "1", cost: "" }],
          )
        }
      />
      {typingOk ? null : <Alert kind="problem">{t("orders.editor.typingProblem")}</Alert>}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      <div>
        <ActionButton
          colour="green"
          icon={Save}
          label={t("orders.editor.save.label")}
          tip={t("orders.editor.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}

// An amount as plain text for typing: "50.00", no symbol or separators.
export function plain(minor: number, currency: CurrencyFormat): string {
  return formatMoney(minor, { ...currency, symbol: "", group_sep: "", decimal_sep: "." });
}

// A quantity for typing and reading, without trailing zeros: 24000 → "24", 1250 → "1.25".
export function qtyText(milli: number): string {
  const text = formatQty(milli, 3);
  return text.includes(".") ? text.replace(/0+$/, "").replace(/\.$/, "") : text;
}
