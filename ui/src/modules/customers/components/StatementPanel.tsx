import { FileText } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { Statement } from "../../../generated/Statement";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { loadStatement } from "../api";

// Today as year-month-day, and the first day of this month.
function today(): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${String(now.getFullYear())}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

// The credit statement: every change in the chosen days with the running balance (CUS-05).
export function StatementPanel(props: { customerId: string; currency: CurrencyFormat }) {
  const { t } = useTranslation("customers");
  const money = (minor: number) => formatMoney(minor, props.currency);
  const [from, setFrom] = useState(`${today().slice(0, 8)}01`);
  const [to, setTo] = useState(today());
  const [statement, setStatement] = useState<Statement | null>(null);
  const load = useSave();

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">{t("statement.heading")}</h3>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("statement.from.label")}
          tip={t("statement.from.tip")}
          value={from}
          onChange={setFrom}
          width="short"
        />
        <Field
          label={t("statement.to.label")}
          tip={t("statement.to.tip")}
          value={to}
          onChange={setTo}
          width="short"
        />
        <ActionButton
          colour="blue"
          icon={FileText}
          label={t("statement.show.label")}
          tip={t("statement.show.tip")}
          onClick={() => load.run(() => loadStatement(props.customerId, from, to), setStatement)}
        />
      </div>
      {load.problem ? <Alert kind="problem">{load.problem}</Alert> : null}
      {statement ? (
        <>
          <p>{t("statement.opening", { amount: money(statement.opening) })}</p>
          {statement.lines.length === 0 ? (
            <p>{t("statement.empty")}</p>
          ) : (
            <table className="w-full border-collapse">
              <thead>
                <tr className="border-b border-border text-left">
                  <th className="p-2">{t("statement.date")}</th>
                  <th className="p-2">{t("statement.kind")}</th>
                  <th className="p-2 text-right">{t("statement.amount")}</th>
                  <th className="p-2 text-right">{t("statement.balance")}</th>
                </tr>
              </thead>
              <tbody>
                {statement.lines.map((line) => (
                  <tr key={line.id} className="border-b border-border">
                    <td className="p-2">{line.business_date}</td>
                    <td className="p-2">{t(`statement.kinds.${line.kind}`)}</td>
                    <td className="p-2 text-right tabular-nums">{money(line.amount)}</td>
                    <td className="p-2 text-right tabular-nums">{money(line.balance_after)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <p className="font-semibold">
            {t("statement.closing", { amount: money(statement.closing) })}
          </p>
        </>
      ) : null}
    </section>
  );
}
