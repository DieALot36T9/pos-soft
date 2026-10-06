import { Archive, ArchiveRestore, ArrowLeft, Ban } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CustomerPage } from "../../../generated/CustomerPage";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { archiveCustomer, getCustomer } from "../api";
import { DebtPaymentForm, PointsAdjustForm } from "./AccountForms";
import { CustomerForm } from "./CustomerForm";
import { StatementPanel } from "./StatementPanel";

type CustomerDetailProps = {
  id: string;
  canManage: boolean;
  canTakePayment: boolean;
  canAdjustPoints: boolean;
  onBack: () => void;
};

// One customer's page: balances, details, purchases, points, payments and the
// statement (CUS-01, CUS-03 to CUS-05). Every number comes from Rust.
export function CustomerDetail(props: CustomerDetailProps) {
  const { t } = useTranslation("customers");
  const { t: tErrors } = useTranslation("errors");
  const [page, setPage] = useState<CustomerPage | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  const load = () => {
    getCustomer(props.id).then(setPage).catch(fail);
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load again only for another customer.
  useEffect(load, [props.id]);

  const back = (
    <ActionButton
      colour="grey"
      icon={ArrowLeft}
      label={t("detail.back.label")}
      tip={t("detail.back.tip")}
      onClick={props.onBack}
    />
  );
  if (page === null) {
    return (
      <div className="flex flex-col gap-3">
        {back}
        {problem ? <Alert kind="problem">{problem}</Alert> : null}
      </div>
    );
  }
  const money = (minor: number) => formatMoney(minor, page.currency);
  const archived = page.customer.archived_at !== null;
  const facts: [string, string][] = [
    [t("detail.points"), String(page.points)],
    page.credit >= 0
      ? [t("detail.credit"), money(page.credit)]
      : [t("detail.storeCredit"), money(-page.credit)],
    [t("detail.limit"), money(page.customer.credit_limit)],
    [t("detail.spend"), money(page.total_spend)],
    [t("detail.visits"), String(page.visits)],
  ];

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        {back}
        {props.canManage ? (
          <ActionButton
            colour={archived ? "green" : "red"}
            icon={archived ? ArchiveRestore : Archive}
            label={archived ? t("detail.unarchive.label") : t("detail.archive.label")}
            tip={archived ? t("detail.unarchive.tip") : t("detail.archive.tip")}
            onClick={() => {
              archiveCustomer(page.customer.id, !archived).then(load).catch(fail);
            }}
          />
        ) : null}
      </div>
      <div>
        <h2 className="text-[28px] font-semibold">{page.customer.name}</h2>
        <p className="text-text-muted">
          {t("detail.phoneAndTier", {
            phone: page.customer.phone,
            tier: page.tier_name ? t("detail.tier", { tier: page.tier_name }) : t("detail.noTier"),
          })}
        </p>
      </div>
      {archived ? <Alert kind="warning">{t("detail.archivedNote")}</Alert> : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <dl className="grid grid-cols-5 gap-3">
        {facts.map(([label, value]) => (
          <div key={label} className="rounded-ui border border-border bg-panel p-3">
            <dt className="text-text-muted">{label}</dt>
            <dd className="text-xl font-semibold tabular-nums">{value}</dd>
          </div>
        ))}
      </dl>
      {page.oldest_unpaid_at !== null ? (
        <p className="text-amber">
          {t("detail.oldestUnpaid", { date: formatDateTime(page.oldest_unpaid_at).slice(0, 10) })}
        </p>
      ) : null}
      {props.canTakePayment ? <DebtPaymentForm page={page} onDone={load} /> : null}
      {props.canAdjustPoints ? <PointsAdjustForm page={page} onDone={load} /> : null}
      <Purchases page={page} />
      <PointsHistory page={page} />
      <StatementPanel customerId={page.customer.id} currency={page.currency} />
      {props.canManage ? (
        <CustomerForm
          key={page.customer.updated_at}
          customer={page.customer}
          currency={page.currency}
          tiers={page.tiers}
          heading={t("detail.edit.heading")}
          saveLabel={t("detail.save.label")}
          saveTip={t("detail.save.tip")}
          doneText={t("detail.edit.saved")}
          onSaved={load}
        />
      ) : null}
    </div>
  );
}

function Purchases({ page }: { page: CustomerPage }) {
  const { t } = useTranslation("customers");
  return (
    <section className="flex flex-col gap-2 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">{t("purchases.heading")}</h3>
      {page.purchases.length === 0 ? (
        <p>{t("purchases.empty")}</p>
      ) : (
        <table className="w-full border-collapse">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("purchases.at")}</th>
              <th className="p-2">{t("purchases.kind")}</th>
              <th className="p-2">{t("purchases.number")}</th>
              <th className="p-2 text-right">{t("purchases.total")}</th>
            </tr>
          </thead>
          <tbody>
            {page.purchases.map((row) => (
              <tr key={row.id} className="border-b border-border">
                <td className="p-2">{formatDateTime(row.at)}</td>
                <td className="p-2">
                  {row.kind === "sale" ? t("purchases.sale") : t("purchases.return")}{" "}
                  {row.voided ? (
                    <Badge
                      colour="red"
                      icon={Ban}
                      word={t("purchases.voided")}
                      tip={t("purchases.voidedTip")}
                    />
                  ) : null}
                </td>
                <td className="p-2">{row.number}</td>
                <td className="p-2 text-right tabular-nums">
                  {formatMoney(row.total, page.currency)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function PointsHistory({ page }: { page: CustomerPage }) {
  const { t } = useTranslation("customers");
  return (
    <section className="flex flex-col gap-2 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">{t("pointsHistory.heading")}</h3>
      {page.points_history.length === 0 ? (
        <p>{t("pointsHistory.empty")}</p>
      ) : (
        <table className="w-full border-collapse">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("pointsHistory.at")}</th>
              <th className="p-2">{t("pointsHistory.kind")}</th>
              <th className="p-2 text-right">{t("pointsHistory.points")}</th>
              <th className="p-2">{t("pointsHistory.reason")}</th>
            </tr>
          </thead>
          <tbody>
            {page.points_history.map((row, index) => (
              <tr key={`${String(row.at)}-${String(index)}`} className="border-b border-border">
                <td className="p-2">{formatDateTime(row.at)}</td>
                <td className="p-2">{t(`pointsHistory.kinds.${row.kind}`)}</td>
                <td className="p-2 text-right tabular-nums">{row.points}</td>
                <td className="p-2">{row.reason ?? ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
