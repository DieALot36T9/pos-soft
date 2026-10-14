import {
  AlertTriangle,
  CheckCircle,
  ChevronDown,
  ChevronLeft,
  Clock,
  PlusCircle,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { BatchView } from "../../../generated/BatchView";
import type { HistoryRow } from "../../../generated/HistoryRow";
import type { MoveKind } from "../../../generated/MoveKind";
import type { StockProduct } from "../../../generated/StockProduct";
import { errorMessage } from "../../../shared/bridge";
import { formatQty } from "../../../shared/format/qty";
import { formatDateTime } from "../../../shared/format/time";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { getStock, stockHistory } from "../api";
import { AdjustForm } from "../components/AdjustForm";

const HISTORY_PAGE = 50;

const allKinds: MoveKind[] = [
  "sale",
  "sale_void",
  "return",
  "receive",
  "supplier_return",
  "adjust",
  "count",
  "opening",
];

type StockProductPageProps = {
  productId: string;
  canAdjust: boolean;
  onBack: () => void;
};

export function StockProductPage({ productId, canAdjust, onBack }: StockProductPageProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [stock, setStock] = useState<StockProduct | null>(null);
  const [history, setHistory] = useState<HistoryRow[]>([]);
  const [kind, setKind] = useState<MoveKind | "">("");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [limit, setLimit] = useState(HISTORY_PAGE);
  const [adjusting, setAdjusting] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [historyProblem, setHistoryProblem] = useState<string | null>(null);

  useHotkeys(shortcuts.back.keys, () => (adjusting ? setAdjusting(false) : onBack()), {
    enableOnFormTags: true,
  });

  const loadStock = () => {
    getStock(productId)
      .then(setStock)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once for this product.
  useEffect(loadStock, [productId]);

  const loadHistory = () => {
    const date = (text: string) => (/^\d{4}-\d{2}-\d{2}$/.test(text.trim()) ? text.trim() : null);
    stockHistory({
      product_id: productId,
      from: date(from),
      to: date(to),
      kinds: kind === "" ? [] : [kind],
      page: { offset: 0, limit },
    })
      .then((rows) => {
        setHistory(rows);
        setHistoryProblem(null);
      })
      .catch((error: unknown) => setHistoryProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when a filter or the stock changes.
  useEffect(loadHistory, [productId, kind, from, to, limit, stock]);

  if (!stock) {
    return problem ? <Alert kind="problem">{problem}</Alert> : null;
  }
  const qty = (milli: number) => formatQty(milli, stock.unit_decimals);

  return (
    <section aria-labelledby="stock-product-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="stock-product-heading" className="text-[28px] font-semibold">
            {stock.name}
          </h2>
          <p className="font-mono text-sm text-text-muted">{stock.sku}</p>
        </div>
        <div className="flex gap-2">
          {canAdjust && !adjusting ? (
            <ActionButton
              colour="blue"
              icon={PlusCircle}
              label={t("product.adjust.label")}
              tip={t("product.adjust.tip")}
              onClick={() => {
                setDone(null);
                setAdjusting(true);
              }}
            />
          ) : null}
          <ActionButton
            colour="grey"
            icon={ChevronLeft}
            label={t("product.back.label")}
            tip={t("product.back.tip")}
            shortcut={shortcuts.back.label}
            onClick={onBack}
          />
        </div>
      </div>

      {/* Product Stock Summary Strip */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <div className="rounded-ui border border-border bg-panel p-4">
          <p
            className={"text-xl font-bold " + (stock.on_hand < 0 ? "text-red" : "text-text")}
            data-testid="on-hand"
          >
            {t("product.onHand", { qty: qty(stock.on_hand), unit: stock.unit_symbol })}
          </p>
        </div>
        <div className="rounded-ui border border-border bg-panel p-4 flex items-center">
          <p className="text-sm text-text-muted font-medium">
            {stock.reorder_level === null
              ? t("product.noReorder")
              : t("product.reorder", {
                  level: qty(stock.reorder_level),
                  qty: qty(stock.reorder_qty ?? 0),
                  unit: stock.unit_symbol,
                })}
          </p>
        </div>
      </div>

      {done ? <Alert kind="done">{done}</Alert> : null}
      {adjusting ? (
        <AdjustForm
          stock={stock}
          onCancel={() => setAdjusting(false)}
          onSaved={(onHand) => {
            setAdjusting(false);
            setDone(t("adjust.done", { qty: qty(onHand), unit: stock.unit_symbol }));
            loadStock();
          }}
        />
      ) : null}

      <div className="flex flex-col gap-2">
        <h3 className="text-xl font-semibold">{t("product.batches.title")}</h3>
        {stock.batches.length === 0 ? (
          <p className="rounded-ui border border-border bg-panel p-4 text-text-muted">
            {t("product.batches.none")}
          </p>
        ) : (
          <table className="w-full border-collapse rounded-ui border border-border bg-panel">
            <thead>
              <tr className="border-b border-border bg-bg text-left text-sm font-bold">
                <th className="p-2.5">{t("product.batches.number")}</th>
                <th className="p-2.5">{t("product.batches.expiry")}</th>
                <th className="p-2.5 text-right">{t("product.batches.onHand")}</th>
                <th className="p-2.5">{t("product.batches.status")}</th>
              </tr>
            </thead>
            <tbody>
              {stock.batches.map((batch) => (
                <tr key={batch.id} className="border-b border-border">
                  <td className="p-2.5 font-mono text-sm">
                    {batch.batch_no ?? t("product.batches.noNumber")}
                  </td>
                  <td className="p-2.5 tabular-nums">
                    {batch.expiry_date ?? t("product.batches.noExpiry")}
                  </td>
                  <td className="p-2.5 text-right font-semibold tabular-nums">
                    {qty(batch.on_hand)}
                  </td>
                  <td className="p-2.5">
                    <BatchBadge batch={batch} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="flex flex-col gap-3">
        <h3 className="text-xl font-semibold">{t("history.title")}</h3>
        <div className="rounded-ui border border-border bg-panel p-4 grid grid-cols-1 md:grid-cols-3 gap-3">
          <SelectField<MoveKind | "">
            label={t("history.kind.label")}
            tip={t("history.kind.tip")}
            value={kind}
            options={[
              { value: "", label: t("history.kind.all") },
              ...allKinds.map((each) => ({ value: each, label: t(`kinds.${each}`) })),
            ]}
            onChange={setKind}
          />
          <Field
            label={t("history.from.label")}
            tip={t("history.from.tip")}
            value={from}
            onChange={setFrom}
          />
          <Field
            label={t("history.to.label")}
            tip={t("history.to.tip")}
            value={to}
            onChange={setTo}
          />
        </div>
        {historyProblem ? <Alert kind="problem">{historyProblem}</Alert> : null}
        {history.length === 0 ? (
          <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
            {t("history.table.empty")}
          </p>
        ) : (
          <table className="w-full border-collapse rounded-ui border border-border bg-panel">
            <thead>
              <tr className="border-b border-border bg-bg text-left text-sm font-bold">
                <th className="p-2.5">{t("history.table.when")}</th>
                <th className="p-2.5">{t("history.table.kind")}</th>
                <th className="p-2.5 text-right">{t("history.table.change")}</th>
                <th className="p-2.5 text-right">{t("history.table.balance")}</th>
                <th className="p-2.5">{t("history.table.batch")}</th>
                <th className="p-2.5">{t("history.table.reason")}</th>
                <th className="p-2.5">{t("history.table.document")}</th>
                <th className="p-2.5">{t("history.table.user")}</th>
              </tr>
            </thead>
            <tbody>
              {history.map((row) => (
                <tr key={row.id} className="border-b border-border hover:bg-blue-soft/20">
                  <td className="p-2.5 tabular-nums whitespace-nowrap">{formatDateTime(row.at)}</td>
                  <td className="p-2.5 font-medium">{t(`kinds.${row.kind}`)}</td>
                  <td
                    className={
                      "p-2.5 text-right font-bold tabular-nums " +
                      (row.qty_delta < 0 ? "text-red" : "text-green")
                    }
                  >
                    {(row.qty_delta > 0 ? "+" : "") + qty(row.qty_delta)}
                  </td>
                  <td className="p-2.5 text-right font-semibold tabular-nums">
                    {qty(row.balance_after)}
                  </td>
                  <td className="p-2.5 font-mono text-sm">
                    {row.batch_no ?? row.batch_expiry ?? ""}
                  </td>
                  <td className="p-2.5">
                    {[row.reason_name, row.note].filter((part) => part).join(": ")}
                  </td>
                  <td className="p-2.5 font-mono text-sm">{row.ref_label ?? ""}</td>
                  <td className="p-2.5">{row.user_name ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {history.length >= limit ? (
          <div>
            <ActionButton
              colour="grey"
              icon={ChevronDown}
              label={t("history.more.label")}
              tip={t("history.more.tip")}
              onClick={() => setLimit(limit + HISTORY_PAGE)}
            />
          </div>
        ) : null}
      </div>
    </section>
  );
}

function BatchBadge({ batch }: { batch: BatchView }) {
  const { t } = useTranslation("inventory");
  if (batch.expired) {
    return (
      <Badge
        colour="red"
        icon={AlertTriangle}
        word={t("product.batches.expired")}
        tip={t("product.batches.expiredTip")}
      />
    );
  }
  if (batch.near_expiry) {
    return (
      <Badge
        colour="amber"
        icon={Clock}
        word={t("product.batches.soon")}
        tip={t("product.batches.soonTip")}
      />
    );
  }
  return (
    <Badge
      colour="green"
      icon={CheckCircle}
      word={t("product.batches.good")}
      tip={t("product.batches.goodTip")}
    />
  );
}
