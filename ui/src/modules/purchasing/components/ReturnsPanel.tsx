import { PackageX, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { GrnRow } from "../../../generated/GrnRow";
import type { ReturnRow } from "../../../generated/ReturnRow";
import type { SupplierList } from "../../../generated/SupplierList";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { parseQty } from "../../../shared/format/qty";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { Tip } from "../../../shared/ui/Tip";
import { getDelivery, listDeliveries, listReturns, listSuppliers, sendBack } from "../api";
import { plain, qtyText } from "./OrderEditor";
import { ProductPicker } from "./ProductPicker";

type BackLine = {
  productId: string;
  name: string;
  batchId: string | null;
  qty: string;
  cost: string;
};

export function ReturnsPanel() {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const save = useSave();
  const [suppliers, setSuppliers] = useState<SupplierList | null>(null);
  const [deliveries, setDeliveries] = useState<GrnRow[]>([]);
  const [returns, setReturns] = useState<ReturnRow[]>([]);
  const [supplierId, setSupplierId] = useState("");
  const [grnId, setGrnId] = useState("");
  const [reason, setReason] = useState("");
  const [lines, setLines] = useState<BackLine[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(() => {
    listSuppliers(true).then(setSuppliers).catch(fail);
    listDeliveries().then(setDeliveries).catch(fail);
    listReturns().then(setReturns).catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load again after a return.
  }, [reload]);

  if (suppliers === null) {
    return problem ? <Alert kind="problem">{problem}</Alert> : <p>{t("common.loading")}</p>;
  }
  const currency = suppliers.currency;
  const money = (minor: number) => formatMoney(minor, currency);
  const chosenSupplier = supplierId || suppliers.rows[0]?.supplier.id || "";
  const theirDeliveries = deliveries.filter((grn) => grn.supplier_id === chosenSupplier);
  const typed = lines.map((line) => ({
    line,
    qty: parseQty(line.qty, 3),
    cost: line.cost.trim() === "" ? null : parseMoney(line.cost, currency.minor_digits),
  }));
  const typingOk = typed.every((each) => each.qty.ok && (each.cost === null || each.cost.ok));
  const change = (index: number, patch: Partial<BackLine>) =>
    setLines((old) => old.map((line, at) => (at === index ? { ...line, ...patch } : line)));

  const pickDelivery = (id: string) => {
    setGrnId(id);
    if (id === "") {
      return;
    }
    getDelivery(id)
      .then((grn) =>
        setLines(
          grn.lines.map((line) => ({
            productId: line.product_id,
            name: line.product_name,
            batchId: line.inventory_batch_id,
            qty: qtyText(line.qty),
            cost: plain(line.unit_cost, currency),
          })),
        ),
      )
      .catch(fail);
  };
  const submit = () => {
    if (!typingOk || lines.length === 0) {
      return;
    }
    save.run(
      () =>
        sendBack({
          supplier_id: chosenSupplier,
          grn_id: grnId === "" ? null : grnId,
          reason: reason.trim() === "" ? null : reason.trim(),
          lines: typed.map(({ line, qty, cost }) => ({
            product_id: line.productId,
            qty: qty.ok ? qty.milli : 0,
            unit_cost: cost?.ok ? cost.minor : null,
            batch_id: line.batchId,
          })),
        }),
      (saved) => {
        setNotice(t("returns.done", { number: saved.number, amount: money(saved.total) }));
        setLines([]);
        setGrnId("");
        setReason("");
        setReload((old) => old + 1);
      },
    );
  };
  return (
    <section className="flex flex-col gap-4">
      <div className="rounded-ui border border-border bg-panel p-4 grid grid-cols-1 md:grid-cols-3 gap-3">
        <SelectField
          label={t("common.supplier.label")}
          tip={t("common.supplier.tip")}
          value={chosenSupplier}
          options={suppliers.rows.map(({ supplier }) => ({
            value: supplier.id,
            label: supplier.name,
          }))}
          onChange={(value) => {
            setSupplierId(value);
            setGrnId("");
          }}
        />
        <SelectField
          label={t("returns.grn.label")}
          tip={t("returns.grn.tip")}
          value={grnId}
          options={[
            { value: "", label: t("returns.grn.none") },
            ...theirDeliveries.map((grn) => ({ value: grn.id, label: grn.number })),
          ]}
          onChange={pickDelivery}
        />
        <Field
          label={t("returns.reason.label")}
          tip={t("returns.reason.tip")}
          value={reason}
          onChange={setReason}
        />
      </div>

      {lines.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <tbody>
            {lines.map((line, index) => (
              <tr key={`${line.productId}-${index}`} className="border-b border-border">
                <td className="p-2.5 font-semibold">{line.name}</td>
                <td className="p-2.5">
                  <Tip tip={t("returns.qty.tip")}>
                    <input
                      aria-label={t("returns.qty.label", { name: line.name })}
                      value={line.qty}
                      onChange={(event) => change(index, { qty: event.target.value })}
                      className="min-h-10 w-28 rounded-ui border border-border bg-panel px-2 tabular-nums"
                    />
                  </Tip>
                </td>
                <td className="p-2.5">
                  <Tip tip={t("returns.cost.tip")}>
                    <input
                      aria-label={t("returns.cost.label", { name: line.name })}
                      value={line.cost}
                      onChange={(event) => change(index, { cost: event.target.value })}
                      className="min-h-10 w-32 rounded-ui border border-border bg-panel px-2 tabular-nums"
                    />
                  </Tip>
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
            ))}
          </tbody>
        </table>
      ) : (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("returns.noLines")}
        </p>
      )}

      <div className="rounded-ui border border-border bg-panel p-4">
        <ProductPicker
          onPick={(hit) =>
            setLines((old) => [
              ...old,
              { productId: hit.product_id, name: hit.name, batchId: null, qty: "1", cost: "" },
            ])
          }
        />
      </div>

      {typingOk ? null : <Alert kind="problem">{t("returns.typingProblem")}</Alert>}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}
      <div className="flex justify-end">
        <ActionButton
          colour="red"
          icon={PackageX}
          label={t("returns.save.label")}
          tip={t("returns.save.tip")}
          disabledReason={lines.length === 0 ? t("returns.noLines") : undefined}
          onClick={submit}
        />
      </div>

      <h3 className="text-xl font-semibold pt-2">{t("returns.list")}</h3>
      {returns.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("returns.empty")}
        </p>
      ) : null}
      {returns.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("returns.number")}</th>
              <th className="p-2.5">{t("common.supplier.label")}</th>
              <th className="p-2.5">{t("returns.grn.label")}</th>
              <th className="p-2.5">{t("suppliers.detail.date")}</th>
              <th className="p-2.5">{t("returns.reason.label")}</th>
              <th className="p-2.5 text-right">{t("returns.total")}</th>
            </tr>
          </thead>
          <tbody>
            {returns.map((row) => (
              <tr key={row.id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="p-2.5 font-mono text-sm">{row.number}</td>
                <td className="p-2.5 font-semibold">{row.supplier_name}</td>
                <td className="p-2.5 font-mono text-sm">{row.grn_number ?? ""}</td>
                <td className="p-2.5 tabular-nums">{formatDateTime(row.at)}</td>
                <td className="p-2.5">{row.reason ?? ""}</td>
                <td className="p-2.5 text-right font-semibold tabular-nums">{money(row.total)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </section>
  );
}
