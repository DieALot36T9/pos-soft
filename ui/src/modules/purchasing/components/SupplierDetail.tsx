import { Archive, ArrowLeft } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { SupplierPage } from "../../../generated/SupplierPage";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { archiveSupplier, getSupplier } from "../api";
import { PaymentForm } from "./PaymentForm";
import { StatementPanel } from "./StatementPanel";
import { SupplierForm } from "./SupplierForm";

type SupplierDetailProps = {
  id: string;
  canPay: boolean;
  onBack: () => void;
};

// One supplier: what the shop owes, the details, a payment, the statement and the
// deliveries (PUR-01, PUR-05, PUR-06). The balance comes from Rust.
export function SupplierDetail({ id, canPay, onBack }: SupplierDetailProps) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const [page, setPage] = useState<SupplierPage | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(() => {
    getSupplier(id).then(setPage).catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load again after a change.
  }, [id, reload]);

  if (page === null) {
    return problem ? <Alert kind="problem">{problem}</Alert> : <p>{t("common.loading")}</p>;
  }
  const money = (minor: number) => formatMoney(minor, page.currency);
  const archive = () => {
    archiveSupplier(id)
      .then(() => {
        setNotice(t("suppliers.detail.archivedDone"));
        setReload((old) => old + 1);
      })
      .catch(fail);
  };
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("common.back.label")}
          tip={t("common.back.tip")}
          onClick={onBack}
        />
        {page.supplier.archived_at === null ? (
          <ActionButton
            colour="red"
            icon={Archive}
            label={t("suppliers.detail.archive.label")}
            tip={t("suppliers.detail.archive.tip")}
            onClick={archive}
          />
        ) : null}
      </div>
      <h3 className="text-2xl font-semibold">{page.supplier.name}</h3>
      <p className="text-xl font-semibold">
        {page.balance >= 0
          ? t("suppliers.detail.balance", { amount: money(page.balance) })
          : t("suppliers.detail.credit", { amount: money(-page.balance) })}
      </p>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}
      {canPay ? <PaymentForm page={page} onPaid={() => setReload((old) => old + 1)} /> : null}
      <StatementPanel supplierId={id} currency={page.currency} />
      <section className="flex flex-col gap-2">
        <h4 className="text-xl font-semibold">{t("suppliers.detail.grns")}</h4>
        {page.grns.length === 0 ? <p>{t("suppliers.detail.noGrns")}</p> : null}
        {page.grns.length > 0 ? (
          <table className="w-full border-collapse rounded-ui border border-border bg-panel">
            <thead>
              <tr className="border-b border-border text-left">
                <th className="p-2">{t("suppliers.detail.grnNumber")}</th>
                <th className="p-2">{t("suppliers.detail.date")}</th>
                <th className="p-2">{t("suppliers.detail.invoice")}</th>
                <th className="p-2 text-right">{t("suppliers.detail.total")}</th>
              </tr>
            </thead>
            <tbody>
              {page.grns.map((grn) => (
                <tr key={grn.id} className="border-b border-border">
                  <td className="p-2">{grn.number}</td>
                  <td className="p-2">{formatDateTime(grn.received_at)}</td>
                  <td className="p-2">{grn.supplier_invoice_no ?? ""}</td>
                  <td className="p-2 text-right tabular-nums">{money(grn.invoice_total)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : null}
      </section>
      <SupplierForm
        key={`${page.supplier.id}-${page.supplier.updated_at}`}
        supplier={page.supplier}
        heading={page.supplier.name}
        saveLabel={t("suppliers.form.save.label")}
        saveTip={t("suppliers.form.save.tip")}
        onSaved={() => setReload((old) => old + 1)}
      />
    </div>
  );
}
