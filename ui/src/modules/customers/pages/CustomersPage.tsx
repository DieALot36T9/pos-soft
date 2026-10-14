import { Archive } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CustomerList } from "../../../generated/CustomerList";
import type { Permission } from "../../../generated/Permission";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { Tip } from "../../../shared/ui/Tip";
import { listCustomers } from "../api";
import { CustomerDetail } from "../components/CustomerDetail";
import { CustomerForm } from "../components/CustomerForm";

export function CustomersPage({ permissions }: { permissions: Permission[] }) {
  const { t } = useTranslation("customers");
  const { t: tErrors } = useTranslation("errors");
  const [text, setText] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [list, setList] = useState<CustomerList | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [reload, setReload] = useState(0);

  useEffect(() => {
    listCustomers(text, showArchived)
      .then((found) => {
        setList(found);
        setProblem(null);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- search again when the filter changes.
  }, [text, showArchived, reload]);

  if (opened !== null) {
    return (
      <CustomerDetail
        id={opened}
        canManage={permissions.includes("customer_manage")}
        canTakePayment={permissions.includes("customer_receive_payment")}
        canAdjustPoints={permissions.includes("loyalty_adjust")}
        onBack={() => {
          setOpened(null);
          setReload((old) => old + 1);
        }}
      />
    );
  }
  const money = (minor: number) => (list ? formatMoney(minor, list.currency) : "");

  return (
    <section aria-labelledby="customers-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="customers-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>

      <div className="rounded-ui border border-border bg-panel p-4 flex flex-wrap items-end justify-between gap-4">
        <div className="flex-1 min-w-[260px]">
          <Field label={t("search.label")} tip={t("search.tip")} value={text} onChange={setText} />
        </div>
        <CheckBox
          label={t("showArchived.label")}
          tip={t("showArchived.tip")}
          checked={showArchived}
          onChange={setShowArchived}
        />
      </div>

      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {list && list.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("table.empty")}
        </p>
      ) : null}
      {list && list.rows.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("table.name")}</th>
              <th className="p-2.5">{t("table.phone")}</th>
              <th className="p-2.5">{t("table.tier")}</th>
              <th className="p-2.5 text-right">{t("table.points")}</th>
              <th className="p-2.5 text-right">{t("table.credit")}</th>
            </tr>
          </thead>
          <tbody>
            {list.rows.map((row) => (
              <tr key={row.id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="p-2.5">
                  <Tip tip={t("table.open", { name: row.name })}>
                    <button
                      type="button"
                      onClick={() => setOpened(row.id)}
                      className="font-semibold text-blue underline cursor-pointer"
                    >
                      {row.name}
                    </button>
                  </Tip>{" "}
                  {row.archived ? (
                    <Badge
                      colour="grey"
                      icon={Archive}
                      word={t("table.archived")}
                      tip={t("table.archivedTip")}
                    />
                  ) : null}
                </td>
                <td className="p-2.5 font-mono text-sm">{row.phone}</td>
                <td className="p-2.5">{row.tier_name ?? ""}</td>
                <td className="p-2.5 text-right font-semibold tabular-nums">{row.points}</td>
                <td className="p-2.5 text-right font-semibold tabular-nums">
                  {row.credit >= 0
                    ? money(row.credit)
                    : t("table.storeCredit", { amount: money(-row.credit) })}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      {list ? (
        <CustomerForm
          customer={null}
          currency={list.currency}
          tiers={null}
          heading={t("add.heading")}
          saveLabel={t("add.save.label")}
          saveTip={t("add.save.tip")}
          doneText={t("add.saved")}
          onSaved={() => setReload((old) => old + 1)}
        />
      ) : null}
    </section>
  );
}
