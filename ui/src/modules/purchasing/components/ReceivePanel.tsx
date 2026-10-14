import { PackageCheck, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { GrnDone } from "../../../generated/GrnDone";
import type { GrnInput } from "../../../generated/GrnInput";
import type { GrnPreview } from "../../../generated/GrnPreview";
import type { OrderRow } from "../../../generated/OrderRow";
import type { SupplierList } from "../../../generated/SupplierList";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { parseQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { Tip } from "../../../shared/ui/Tip";
import { listOrders, listSuppliers, previewDelivery, saveDelivery, startReceiving } from "../api";
import { plain, qtyText } from "./OrderEditor";
import { ProductPicker } from "./ProductPicker";

type ReceiveLine = {
  orderLineId: string | null;
  productId: string;
  name: string;
  remaining: number | null;
  qty: string;
  cost: string;
  batch: string;
  expiry: string;
};

const DATE = /^\d{4}-\d{2}-\d{2}$/;

export function ReceivePanel({ startOrder }: { startOrder: string | null }) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const save = useSave();
  const [suppliers, setSuppliers] = useState<SupplierList | null>(null);
  const [orders, setOrders] = useState<OrderRow[]>([]);
  const [orderId, setOrderId] = useState<string>(startOrder ?? "");
  const [supplierId, setSupplierId] = useState("");
  const [lines, setLines] = useState<ReceiveLine[]>([]);
  const [invoiceNo, setInvoiceNo] = useState("");
  const [invoiceTotal, setInvoiceTotal] = useState("");
  const [notes, setNotes] = useState("");
  const [preview, setPreview] = useState<GrnPreview | null>(null);
  const [done, setDone] = useState<GrnDone | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(() => {
    listSuppliers(false).then(setSuppliers).catch(fail);
    listOrders(null)
      .then((list) =>
        setOrders(list.rows.filter((row) => ["draft", "sent", "partial"].includes(row.status))),
      )
      .catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, []);

  useEffect(() => {
    if (orderId === "") {
      return;
    }
    startReceiving(orderId)
      .then((start) => {
        setSupplierId(start.supplier_id);
        setLines(
          start.lines.map((line) => ({
            orderLineId: line.order_line_id,
            productId: line.product_id,
            name: line.name,
            remaining: line.remaining,
            qty: qtyText(line.remaining),
            cost: plain(line.unit_cost, start.currency),
            batch: "",
            expiry: "",
          })),
        );
      })
      .catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- fill again when the order changes.
  }, [orderId]);

  const currency = suppliers?.currency ?? null;
  const chosenSupplier = supplierId || suppliers?.rows[0]?.supplier.id || "";
  const input = (): GrnInput | null => {
    if (currency === null || chosenSupplier === "") {
      return null;
    }
    const total = parseMoney(invoiceTotal, currency.minor_digits);
    if (!total.ok) {
      return null;
    }
    const built = [];
    for (const line of lines) {
      const qty = parseQty(line.qty, 3);
      const cost = parseMoney(line.cost, currency.minor_digits);
      const expiry = line.expiry.trim();
      if (!qty.ok || !cost.ok || (expiry !== "" && !DATE.test(expiry))) {
        return null;
      }
      built.push({
        order_line_id: line.orderLineId,
        product_id: line.productId,
        qty: qty.milli,
        unit_cost: cost.minor,
        batch_no: line.batch.trim() === "" ? null : line.batch.trim(),
        expiry_date: expiry === "" ? null : expiry,
      });
    }
    return {
      supplier_id: chosenSupplier,
      order_id: orderId === "" ? null : orderId,
      supplier_invoice_no: invoiceNo.trim() === "" ? null : invoiceNo.trim(),
      invoice_total: total.minor,
      notes: notes.trim() === "" ? null : notes.trim(),
      lines: built,
    };
  };
  const typed = input();
  const key = JSON.stringify(typed);
  useEffect(() => {
    if (typed === null || typed.lines.length === 0) {
      return;
    }
    previewDelivery(typed)
      .then((answer) => {
        setPreview(answer);
        setProblem(null);
      })
      .catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- ask Rust again only when the typing changes.
  }, [key]);

  if (suppliers === null || currency === null) {
    return problem ? <Alert kind="problem">{problem}</Alert> : <p>{t("common.loading")}</p>;
  }
  const money = (minor: number) => formatMoney(minor, currency);
  const change = (index: number, patch: Partial<ReceiveLine>) =>
    setLines((old) => old.map((line, at) => (at === index ? { ...line, ...patch } : line)));
  const shown = typed !== null && typed.lines.length > 0 ? preview : null;
  const submit = () => {
    if (typed === null || typed.lines.length === 0) {
      return;
    }
    save.run(
      () => saveDelivery(typed),
      (saved) => {
        setDone(saved);
        setLines([]);
        setOrderId("");
        setInvoiceNo("");
        setInvoiceTotal("");
        setNotes("");
        setPreview(null);
      },
    );
  };
  const costChanges = (saved: GrnDone) =>
    saved.cost_changes
      .map((each) => `${each.name} ${money(each.old_cost)} → ${money(each.new_cost)}`)
      .join(", ");

  return (
    <section className="flex flex-col gap-4">
      <div className="rounded-ui border border-border bg-panel p-4 grid grid-cols-1 md:grid-cols-2 gap-3">
        <SelectField
          label={t("receive.order.label")}
          tip={t("receive.order.tip")}
          value={orderId}
          options={[
            { value: "", label: t("receive.order.none") },
            ...orders.map((row) => ({
              value: row.id,
              label: `${row.number} · ${row.supplier_name}`,
            })),
          ]}
          onChange={(value) => {
            setOrderId(value);
            setDone(null);
            if (value === "") {
              setLines([]);
            }
          }}
        />
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
          label={t("receive.invoiceNo.label")}
          tip={t("receive.invoiceNo.tip")}
          value={invoiceNo}
          onChange={setInvoiceNo}
        />
        <Field
          label={t("receive.invoiceTotal.label")}
          tip={t("receive.invoiceTotal.tip")}
          value={invoiceTotal}
          onChange={setInvoiceTotal}
        />
        <div className="md:col-span-2">
          <Field
            label={t("receive.notes.label")}
            tip={t("receive.notes.tip")}
            value={notes}
            onChange={setNotes}
          />
        </div>
      </div>

      {done ? (
        <Alert kind="done">
          {t("receive.done", { number: done.number })}{" "}
          {done.total_differs ? t("receive.differs") : ""}{" "}
          {done.over_received.length > 0
            ? t("receive.over", { names: done.over_received.join(", ") })
            : ""}{" "}
          {done.cost_changes.length > 0 ? t("receive.costs", { changes: costChanges(done) }) : ""}
        </Alert>
      ) : null}

      {lines.length === 0 && !done ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("receive.empty")}
        </p>
      ) : null}

      {lines.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("receive.product")}</th>
              <th className="p-2.5">{t("orders.detail.received")}</th>
              <th className="p-2.5">{t("orders.detail.cost")}</th>
              <th className="p-2.5">{t("receive.batchColumn")}</th>
              <th className="p-2.5">{t("receive.expiryColumn")}</th>
              <th className="p-2.5 text-right">{t("receive.value")}</th>
              <th className="p-2.5" />
            </tr>
          </thead>
          <tbody>
            {lines.map((line, index) => {
              const seen = shown?.lines[index];
              return (
                <tr
                  key={`${line.productId}-${line.orderLineId ?? index}`}
                  className="border-b border-border"
                >
                  <td className="p-2.5 font-semibold">
                    {line.name}
                    {line.remaining !== null ? (
                      <span className="block text-xs font-normal text-text-muted">
                        {t("receive.remaining", { qty: qtyText(line.remaining) })}
                      </span>
                    ) : null}
                    {seen && seen.new_cost !== seen.old_cost ? (
                      <Tip tip={t("receive.costChangeTip")}>
                        <span className="block text-xs font-semibold text-purple">
                          {t("receive.costChange", {
                            old: money(seen.old_cost),
                            new: money(seen.new_cost),
                          })}
                        </span>
                      </Tip>
                    ) : null}
                  </td>
                  {(["qty", "cost", "batch", "expiry"] as const).map((field) => (
                    <td key={field} className="p-2.5">
                      <Tip tip={t(`receive.${field}.tip`)}>
                        <input
                          aria-label={t(`receive.${field}.label`, { name: line.name })}
                          value={line[field]}
                          placeholder={
                            field === "expiry" && seen?.tracks_expiry
                              ? t("receive.needsExpiry")
                              : undefined
                          }
                          onChange={(event) => change(index, { [field]: event.target.value })}
                          className="min-h-10 w-28 rounded-ui border border-border bg-panel px-2 tabular-nums"
                        />
                      </Tip>
                    </td>
                  ))}
                  <td className="p-2.5 text-right font-semibold tabular-nums">
                    {seen ? money(seen.value) : ""}
                  </td>
                  <td className="p-2.5">
                    <ActionButton
                      colour="red"
                      icon={Trash2}
                      label={t("common.remove.label")}
                      tip={t("common.remove.tip")}
                      onClick={() => setLines((old) => old.filter((_, at) => at !== index))}
                    />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : null}

      <div className="rounded-ui border border-border bg-panel p-4">
        <ProductPicker
          onPick={(hit) => {
            setDone(null);
            setLines((old) => [
              ...old,
              {
                orderLineId: null,
                productId: hit.product_id,
                name: hit.name,
                remaining: null,
                qty: "1",
                cost: "",
                batch: "",
                expiry: "",
              },
            ]);
          }}
        />
      </div>

      {shown ? (
        <p className="text-right text-xl font-bold tabular-nums">
          {t("receive.linesTotal", { amount: money(shown.lines_total) })}
        </p>
      ) : null}
      {shown && shown.difference !== 0 ? (
        <Alert kind="warning">{t("receive.difference", { amount: money(shown.difference) })}</Alert>
      ) : null}
      {lines.length > 0 && typed === null ? (
        <Alert kind="problem">{t("receive.typingProblem")}</Alert>
      ) : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={PackageCheck}
          label={t("receive.save.label")}
          tip={t("receive.save.tip")}
          disabledReason={lines.length === 0 ? t("receive.empty") : undefined}
          onClick={submit}
        />
      </div>
    </section>
  );
}
