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

// The customer list with search, adding a customer, and each customer's page
// (Req - Customers and Loyalty). The page shows what the user's role may do.
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
      <div className="flex flex-wrap items-end gap-4">
        <Field label={t("search.label")} tip={t("search.tip")} value={text} onChange={setText} />
        <CheckBox
          label={t("showArchived.label")}
          tip={t("showArchived.tip")}
          checked={showArchived}
          onChange={setShowArchived}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {list && list.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("table.empty")}</p>
      ) : null}
      {list && list.rows.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("table.name")}</th>
              <th className="p-2">{t("table.phone")}</th>
              <th className="p-2">{t("table.tier")}</th>
              <th className="p-2 text-right">{t("table.points")}</th>
              <th className="p-2 text-right">{t("table.credit")}</th>
            </tr>
          </thead>
          <tbody>
            {list.rows.map((row) => (
              <tr key={row.id} className="border-b border-border">
                <td className="p-2">
                  <Tip tip={t("table.open", { name: row.name })}>
                    <button
                      type="button"
                      onClick={() => setOpened(row.id)}
                      className="font-semibold text-blue underline"
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
                <td className="p-2">{row.phone}</td>
                <td className="p-2">{row.tier_name ?? ""}</td>
                <td className="p-2 text-right tabular-nums">{row.points}</td>
                <td className="p-2 text-right tabular-nums">
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
