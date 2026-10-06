import { ArrowLeft, FileSpreadsheet, PackageCheck, PenLine, Send, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { OrderView } from "../../../generated/OrderView";
import { errorMessage } from "../../../shared/bridge";
import { pickSaveFile } from "../../../shared/bridge/pickers";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { cancelOrder, exportOrder, getOrder, markSent } from "../api";
import { qtyText } from "./OrderEditor";

type OrderDetailProps = {
  id: string;
  onBack: () => void;
  onEdit: () => void;
  onReceive: () => void;
};

// One purchase order with its lines, what has arrived, and what can be done with it
// in its status (PUR-02). Values come from Rust.
export function OrderDetail({ id, onBack, onEdit, onReceive }: OrderDetailProps) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const [view, setView] = useState<OrderView | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(() => {
    getOrder(id).then(setView).catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load again after a change.
  }, [id, reload]);

  if (view === null) {
    return problem ? <Alert kind="problem">{problem}</Alert> : <p>{t("common.loading")}</p>;
  }
  const { order } = view;
  const money = (minor: number) => formatMoney(minor, view.currency);
  const after = (work: Promise<unknown>, message: string | null = null) => {
    setProblem(null);
    work
      .then(() => {
        setNotice(message);
        setReload((old) => old + 1);
      })
      .catch(fail);
  };
  const exportIt = () => {
    pickSaveFile(t("orders.detail.saveTitle"), `${order.number}.xlsx`)
      .then((file) =>
        file === null ? null : after(exportOrder(id, file), t("orders.detail.exported")),
      )
      .catch(fail);
  };
  const open = order.status === "draft" || order.status === "sent" || order.status === "partial";
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <ActionButton
        colour="grey"
        icon={ArrowLeft}
        label={t("common.back.label")}
        tip={t("common.back.tip")}
        onClick={onBack}
      />
      <h3 className="text-xl font-semibold">{order.number}</h3>
      <p>{t("orders.detail.supplier", { name: view.supplier_name })}</p>
      <p>{t("orders.detail.status", { status: t(`orders.status.${order.status}`) })}</p>
      {order.expected_date ? (
        <p>{t("orders.detail.expected", { date: order.expected_date })}</p>
      ) : null}
      {order.notes ? <p>{order.notes}</p> : null}
      <table className="w-full border-collapse">
        <thead>
          <tr className="border-b border-border text-left">
            <th className="p-2">{t("orders.editor.product")}</th>
            <th className="p-2 text-right">{t("orders.detail.ordered")}</th>
            <th className="p-2 text-right">{t("orders.detail.received")}</th>
            <th className="p-2 text-right">{t("orders.detail.cost")}</th>
            <th className="p-2 text-right">{t("orders.detail.value")}</th>
          </tr>
        </thead>
        <tbody>
          {order.lines.map((line, index) => (
            <tr key={line.id} className="border-b border-border">
              <td className="p-2">{line.product_name}</td>
              <td className="p-2 text-right tabular-nums">{qtyText(line.qty)}</td>
              <td className="p-2 text-right tabular-nums">{qtyText(line.received)}</td>
              <td className="p-2 text-right tabular-nums">{money(line.unit_cost)}</td>
              <td className="p-2 text-right tabular-nums">{money(view.line_values[index] ?? 0)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="text-right text-xl font-semibold">
        {t("orders.detail.total", { amount: money(view.total) })}
      </p>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}
      <div className="flex flex-wrap gap-2">
        {order.status === "draft" ? (
          <ActionButton
            colour="grey"
            icon={PenLine}
            label={t("orders.detail.edit.label")}
            tip={t("orders.detail.edit.tip")}
            onClick={onEdit}
          />
        ) : null}
        <ActionButton
          colour="blue"
          icon={FileSpreadsheet}
          label={t("orders.detail.export.label")}
          tip={t("orders.detail.export.tip")}
          onClick={exportIt}
        />
        {order.status === "draft" ? (
          <ActionButton
            colour="blue"
            icon={Send}
            label={t("orders.detail.markSent.label")}
            tip={t("orders.detail.markSent.tip")}
            onClick={() => after(markSent(id))}
          />
        ) : null}
        {open ? (
          <ActionButton
            colour="green"
            icon={PackageCheck}
            label={t("orders.detail.receive.label")}
            tip={t("orders.detail.receive.tip")}
            onClick={onReceive}
          />
        ) : null}
        {order.status === "draft" || order.status === "sent" ? (
          <ActionButton
            colour="red"
            icon={XCircle}
            label={t("orders.detail.cancel.label")}
            tip={t("orders.detail.cancel.tip")}
            onClick={() => after(cancelOrder(id))}
          />
        ) : null}
      </div>
    </section>
  );
}
