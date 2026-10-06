import { Archive } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { SupplierList } from "../../../generated/SupplierList";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Tip } from "../../../shared/ui/Tip";
import { listSuppliers } from "../api";
import { SupplierDetail } from "./SupplierDetail";
import { SupplierForm } from "./SupplierForm";

// The supplier list, adding a supplier, and each supplier's page (PUR-01, PUR-05, PUR-06).
export function SuppliersPanel({ canPay }: { canPay: boolean }) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const [showArchived, setShowArchived] = useState(false);
  const [list, setList] = useState<SupplierList | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [reload, setReload] = useState(0);

  useEffect(() => {
    listSuppliers(showArchived)
      .then(setList)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load again when the filter changes.
  }, [showArchived, reload]);

  if (opened !== null) {
    return (
      <SupplierDetail
        id={opened}
        canPay={canPay}
        onBack={() => {
          setOpened(null);
          setReload((old) => old + 1);
        }}
      />
    );
  }
  const money = (minor: number) => (list ? formatMoney(minor, list.currency) : "");
  return (
    <div className="flex flex-col gap-4">
      <CheckBox
        label={t("suppliers.showArchived.label")}
        tip={t("suppliers.showArchived.tip")}
        checked={showArchived}
        onChange={setShowArchived}
      />
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {list && list.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("suppliers.empty")}</p>
      ) : null}
      {list && list.rows.length > 0 ? (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("suppliers.name")}</th>
              <th className="p-2">{t("suppliers.phone")}</th>
              <th className="p-2">{t("suppliers.terms")}</th>
              <th className="p-2 text-right">{t("suppliers.balance")}</th>
            </tr>
          </thead>
          <tbody>
            {list.rows.map(({ supplier, balance }) => (
              <tr key={supplier.id} className="border-b border-border">
                <td className="p-2">
                  <Tip tip={t("suppliers.open", { name: supplier.name })}>
                    <button
                      type="button"
                      onClick={() => setOpened(supplier.id)}
                      className="font-semibold text-blue underline"
                    >
                      {supplier.name}
                    </button>
                  </Tip>{" "}
                  {supplier.archived_at !== null ? (
                    <Badge
                      colour="grey"
                      icon={Archive}
                      word={t("suppliers.archived")}
                      tip={t("suppliers.archivedTip")}
                    />
                  ) : null}
                </td>
                <td className="p-2">{supplier.phone ?? ""}</td>
                <td className="p-2">
                  {t("suppliers.termsDays", { days: supplier.payment_terms_days })}
                </td>
                <td className="p-2 text-right tabular-nums">{money(balance)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
      <SupplierForm
        supplier={null}
        heading={t("suppliers.add.heading")}
        saveLabel={t("suppliers.add.save.label")}
        saveTip={t("suppliers.add.save.tip")}
        onSaved={() => setReload((old) => old + 1)}
      />
    </div>
  );
}
