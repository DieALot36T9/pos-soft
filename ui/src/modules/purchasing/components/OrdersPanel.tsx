import { FilePlus } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { OrderList } from "../../../generated/OrderList";
import type { OrderStatus } from "../../../generated/OrderStatus";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { SelectField } from "../../../shared/ui/SelectField";
import { Tip } from "../../../shared/ui/Tip";
import { listOrders } from "../api";
import { OrderDetail } from "./OrderDetail";
import { OrderEditor } from "./OrderEditor";

const NEW_ORDER = "new";
const STATUSES: OrderStatus[] = ["draft", "sent", "partial", "received", "cancelled"];

// Purchase orders by status, a new order, and each order's page (PUR-02).
export function OrdersPanel({ onReceive }: { onReceive: (orderId: string) => void }) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const [status, setStatus] = useState<OrderStatus | "all">("all");
  const [list, setList] = useState<OrderList | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  // An order id to change, NEW_ORDER for a new one, or null when not editing.
  const [editing, setEditing] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [reload, setReload] = useState(0);

  useEffect(() => {
    listOrders(status === "all" ? null : status)
      .then(setList)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load again when the filter changes.
  }, [status, reload]);

  const backToList = () => {
    setOpened(null);
    setEditing(null);
    setReload((old) => old + 1);
  };
  if (editing !== null) {
    return (
      <OrderEditor
        orderId={editing === NEW_ORDER ? null : editing}
        onSaved={(id) => {
          setEditing(null);
          setOpened(id);
        }}
        onBack={backToList}
      />
    );
  }
  if (opened !== null) {
    return (
      <OrderDetail
        id={opened}
        onBack={backToList}
        onEdit={() => setEditing(opened)}
        onReceive={() => onReceive(opened)}
      />
    );
  }
  const money = (minor: number) => (list ? formatMoney(minor, list.currency) : "");
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <SelectField
          label={t("orders.status.label")}
          tip={t("orders.status.tip")}
          value={status}
          options={[
            { value: "all" as const, label: t("orders.status.all") },
            ...STATUSES.map((each) => ({ value: each, label: t(`orders.status.${each}`) })),
          ]}
          onChange={setStatus}
        />
        <ActionButton
          colour="green"
          icon={FilePlus}
          label={t("orders.new.label")}
          tip={t("orders.new.tip")}
          onClick={() => setEditing(NEW_ORDER)}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {list && list.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("orders.empty")}</p>
      ) : null}
      {list && list.rows.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("orders.number")}</th>
              <th className="p-2">{t("common.supplier.label")}</th>
              <th className="p-2">{t("orders.status.label")}</th>
              <th className="p-2">{t("orders.created")}</th>
              <th className="p-2 text-right">{t("orders.lines")}</th>
              <th className="p-2 text-right">{t("orders.total")}</th>
            </tr>
          </thead>
          <tbody>
            {list.rows.map((row) => (
              <tr key={row.id} className="border-b border-border">
                <td className="p-2">
                  <Tip tip={t("orders.open", { number: row.number })}>
                    <button
                      type="button"
                      onClick={() => setOpened(row.id)}
                      className="font-semibold text-blue underline"
                    >
                      {row.number}
                    </button>
                  </Tip>
                </td>
                <td className="p-2">{row.supplier_name}</td>
                <td className="p-2">{t(`orders.status.${row.status}`)}</td>
                <td className="p-2">{formatDateTime(row.created_at)}</td>
                <td className="p-2 text-right tabular-nums">{row.line_count}</td>
                <td className="p-2 text-right tabular-nums">{money(row.total)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  );
}
