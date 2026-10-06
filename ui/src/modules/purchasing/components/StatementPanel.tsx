import { FileSpreadsheet, ListChecks } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SupplierStatement } from "../../../generated/SupplierStatement";
import { errorMessage } from "../../../shared/bridge";
import { pickSaveFile } from "../../../shared/bridge/pickers";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { exportStatement, loadStatement } from "../api";

// Today as "YYYY-MM-DD", and the first day of this month, to start the date fields.
const today = () => new Date().toISOString().slice(0, 10);
const monthStart = () => `${today().slice(0, 8)}01`;

// A supplier's statement for chosen days, with opening and closing balances (PUR-06).
export function StatementPanel({
  supplierId,
  currency,
}: {
  supplierId: string;
  currency: CurrencyFormat;
}) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const [from, setFrom] = useState(monthStart());
  const [to, setTo] = useState(today());
  const [statement, setStatement] = useState<SupplierStatement | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));
  const money = (minor: number) => formatMoney(minor, currency);

  const show = () => {
    setProblem(null);
    loadStatement(supplierId, from.trim(), to.trim()).then(setStatement).catch(fail);
  };
  const save = () => {
    setProblem(null);
    pickSaveFile(t("suppliers.statement.saveTitle"), `statement-${from}-${to}.xlsx`)
      .then((file) =>
        file === null
          ? null
          : exportStatement(supplierId, from.trim(), to.trim(), file).then((count) =>
              setNotice(t("suppliers.statement.exported", { count })),
            ),
      )
      .catch(fail);
  };
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h4 className="text-xl font-semibold">{t("suppliers.statement.heading")}</h4>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("suppliers.statement.from.label")}
          tip={t("suppliers.statement.from.tip")}
          value={from}
          onChange={setFrom}
          width="short"
        />
        <Field
          label={t("suppliers.statement.to.label")}
          tip={t("suppliers.statement.to.tip")}
          value={to}
          onChange={setTo}
          width="short"
        />
        <ActionButton
          colour="blue"
          icon={ListChecks}
          label={t("suppliers.statement.show.label")}
          tip={t("suppliers.statement.show.tip")}
          onClick={show}
        />
        <ActionButton
          colour="blue"
          icon={FileSpreadsheet}
          label={t("suppliers.statement.export.label")}
          tip={t("suppliers.statement.export.tip")}
          onClick={save}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}
      {statement ? (
        <table className="w-full border-collapse">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("suppliers.statement.date")}</th>
              <th className="p-2">{t("suppliers.statement.kind")}</th>
              <th className="p-2">{t("suppliers.statement.reference")}</th>
              <th className="p-2 text-right">{t("suppliers.statement.amount")}</th>
              <th className="p-2 text-right">{t("suppliers.statement.balance")}</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-b border-border">
              <td className="p-2">{statement.from}</td>
              <td className="p-2 font-semibold" colSpan={3}>
                {t("suppliers.statement.opening")}
              </td>
              <td className="p-2 text-right tabular-nums">{money(statement.opening)}</td>
            </tr>
            {statement.lines.map((line) => (
              <tr key={line.id} className="border-b border-border">
                <td className="p-2">{line.business_date}</td>
                <td className="p-2">{t(`suppliers.statement.kinds.${line.kind}`)}</td>
                <td className="p-2">{line.reference ?? ""}</td>
                <td className="p-2 text-right tabular-nums">{money(line.amount)}</td>
                <td className="p-2 text-right tabular-nums">{money(line.balance_after)}</td>
              </tr>
            ))}
            <tr>
              <td className="p-2">{statement.to}</td>
              <td className="p-2 font-semibold" colSpan={3}>
                {t("suppliers.statement.closing")}
              </td>
              <td className="p-2 text-right tabular-nums">{money(statement.closing)}</td>
            </tr>
          </tbody>
        </table>
      ) : null}
    </section>
  );
}
