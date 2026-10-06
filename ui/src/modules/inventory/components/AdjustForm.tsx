import { Save, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { AdjustBatch } from "../../../generated/AdjustBatch";
import type { Reason } from "../../../generated/Reason";
import type { StockProduct } from "../../../generated/StockProduct";
import { errorField, errorMessage } from "../../../shared/bridge";
import { formatQty, parseQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { adjustStock, stockReasons } from "../api";

type Way = "in" | "out";

type AdjustFormProps = {
  stock: StockProduct;
  onCancel: () => void;
  // Called with the stock on hand after the change.
  onSaved: (onHand: number) => void;
};

// A stock adjustment: way, quantity, reason, note and batch (INV-02, INV-05).
// Rust checks every rule; this form only collects what was typed.
export function AdjustForm({ stock, onCancel, onSaved }: AdjustFormProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [reasons, setReasons] = useState<Reason[]>([]);
  const [way, setWay] = useState<Way>("in");
  const [qtyText, setQtyText] = useState("");
  const [reasonCode, setReasonCode] = useState("");
  const [note, setNote] = useState("");
  // "auto", "none", "new" or a batch id.
  const [batch, setBatch] = useState(stock.track_expiry ? "new" : "none");
  const [batchNo, setBatchNo] = useState("");
  const [expiry, setExpiry] = useState("");
  const [problem, setProblem] = useState<{ field: string | null; message: string } | null>(null);

  const loadOnce = () => {
    stockReasons()
      .then(setReasons)
      .catch((error: unknown) =>
        setProblem({ field: null, message: errorMessage(error, tErrors) }),
      );
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the form opens.
  useEffect(loadOnce, []);

  const fits = (reason: Reason) => reason.direction === "both" || reason.direction === way;
  const choices = reasons.filter(fits);

  const changeWay = (picked: Way) => {
    setWay(picked);
    setBatch(picked === "in" ? (stock.track_expiry ? "new" : "none") : "auto");
    setReasonCode("");
  };

  const batchOptions = [
    way === "out"
      ? { value: "auto", label: t("adjust.batch.auto") }
      : { value: "none", label: t("adjust.batch.none") },
    ...(way === "in" ? [{ value: "new", label: t("adjust.batch.new") }] : []),
    ...stock.batches.map((each) => ({
      value: each.id,
      label: t("adjust.batch.existing", {
        batch: each.batch_no ?? "—",
        expiry: each.expiry_date ?? "—",
        qty: formatQty(each.on_hand, stock.unit_decimals),
      }),
    })),
  ];

  const save = () => {
    const parsed = parseQty(qtyText, stock.unit_decimals);
    if (!parsed.ok) {
      setProblem({ field: "qty", message: tErrors(parsed.code as "qty_invalid") });
      return;
    }
    const expiryText = expiry.trim();
    if (batch === "new" && expiryText !== "" && !/^\d{4}-\d{2}-\d{2}$/.test(expiryText)) {
      setProblem({ field: "expiry_date", message: tErrors("date_invalid") });
      return;
    }
    let chosen: AdjustBatch;
    if (batch === "new") {
      chosen = {
        kind: "new",
        batch_no: batchNo.trim() === "" ? null : batchNo.trim(),
        expiry_date: expiryText === "" ? null : expiryText,
      };
    } else if (batch === "auto" || batch === "none") {
      chosen = { kind: "auto" };
    } else {
      chosen = { kind: "existing", id: batch };
    }
    adjustStock({
      product_id: stock.product_id,
      qty_delta: way === "in" ? parsed.milli : -parsed.milli,
      reason_code: reasonCode,
      note: note.trim() === "" ? null : note,
      batch: chosen,
    })
      .then(onSaved)
      .catch((error: unknown) => {
        const field = errorField(error);
        const available = notEnoughAvailable(error);
        setProblem({
          field,
          message:
            available === null
              ? errorMessage(error, tErrors)
              : t("adjust.notEnough", {
                  qty: formatQty(available, stock.unit_decimals),
                  unit: stock.unit_symbol,
                }),
        });
      });
  };

  const errorFor = (field: string) => (problem?.field === field ? problem.message : null);

  return (
    <form
      aria-label={t("adjust.title", { name: stock.name })}
      className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
    >
      <h3 className="text-xl font-semibold">{t("adjust.title", { name: stock.name })}</h3>
      <div className="grid grid-cols-3 gap-3">
        <SelectField<Way>
          label={t("adjust.direction.label")}
          tip={t("adjust.direction.tip")}
          value={way}
          options={[
            { value: "in", label: t("adjust.direction.in") },
            { value: "out", label: t("adjust.direction.out") },
          ]}
          onChange={changeWay}
        />
        <Field
          label={t("adjust.qty.label")}
          tip={t("adjust.qty.tip")}
          value={qtyText}
          onChange={setQtyText}
          error={errorFor("qty") ?? errorFor("qty_delta")}
        />
        <SelectField
          label={t("adjust.reason.label")}
          tip={t("adjust.reason.tip")}
          value={reasonCode}
          options={[
            { value: "", label: "—" },
            ...choices.map((reason) => ({ value: reason.code, label: reason.name })),
          ]}
          onChange={setReasonCode}
        />
      </div>
      <Field
        label={t("adjust.note.label")}
        tip={t("adjust.note.tip")}
        value={note}
        onChange={setNote}
      />
      <div className="grid grid-cols-3 gap-3">
        <SelectField
          label={t("adjust.batch.label")}
          tip={t("adjust.batch.tip")}
          value={batch}
          options={batchOptions}
          onChange={setBatch}
          error={errorFor("batch")}
        />
        {batch === "new" ? (
          <>
            <Field
              label={t("adjust.batchNo.label")}
              tip={t("adjust.batchNo.tip")}
              value={batchNo}
              onChange={setBatchNo}
            />
            <Field
              label={t("adjust.expiry.label")}
              tip={t("adjust.expiry.tip")}
              value={expiry}
              onChange={setExpiry}
              error={errorFor("expiry_date")}
            />
          </>
        ) : null}
      </div>
      {problem && !["qty", "qty_delta", "batch", "expiry_date"].includes(problem.field ?? "") ? (
        <Alert kind="problem">{problem.message}</Alert>
      ) : null}
      <div className="flex gap-2">
        <ActionButton
          colour="green"
          icon={Save}
          label={t("adjust.save.label")}
          tip={t("adjust.save.tip")}
          onClick={save}
        />
        <ActionButton
          colour="grey"
          icon={X}
          label={t("adjust.cancel.label")}
          tip={t("adjust.cancel.tip")}
          onClick={onCancel}
        />
      </div>
    </form>
  );
}

// The quantity Rust says is available, when it refused for not enough stock.
function notEnoughAvailable(error: unknown): number | null {
  if (
    typeof error === "object" &&
    error !== null &&
    "kind" in error &&
    error.kind === "conflict" &&
    "code" in error &&
    String(error.code).startsWith("not_enough_stock") &&
    "params" in error
  ) {
    const params = error.params as Record<string, string>;
    const available = Number(params.available_milli);
    return Number.isFinite(available) ? available : null;
  }
  return null;
}
