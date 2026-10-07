import { FileSpreadsheet, ListChecks, Printer, ReceiptText } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { Permission } from "../../../generated/Permission";
import type { PrintResult } from "../../../generated/PrintResult";
import type { ReportChoices } from "../../../generated/ReportChoices";
import type { ReportRequest } from "../../../generated/ReportRequest";
import type { ReportTable } from "../../../generated/ReportTable";
import type { ShiftChoice } from "../../../generated/ShiftChoice";
import { errorMessage } from "../../../shared/bridge";
import { pickSaveFile } from "../../../shared/bridge/pickers";
import { formatDateTime } from "../../../shared/format/time";
import { PrintProblem } from "../../../shared/print/PrintProblem";
import { printWindow } from "../../../shared/print/api";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { exportReport, loadChoices, loadShifts, printReport, runReport } from "../api";
import { ReportList } from "../components/ReportList";
import { ReportParams } from "../components/ReportParams";
import { ReportView } from "../components/ReportView";
import { buildRequest, type ReportForm, type ReportKind } from "../request";

type Shown = { report: ReportTable; request: ReportRequest; generatedAt: string };

// Reports (Req - Reports): pick a report, choose the days and filters, show it, then
// print it on receipt paper or A4 (or PDF), or save it as Excel. Every number comes
// from Rust. Without "all reports" a user sees only their own shifts' X and Z (RPT-10).
export function ReportsPage({ permissions }: { permissions: Permission[] }) {
  const { t } = useTranslation("reports");
  const { t: tErrors } = useTranslation("errors");
  const seesAll = permissions.includes("reports_view_all");
  const [choices, setChoices] = useState<ReportChoices | null>(null);
  const [shifts, setShifts] = useState<ShiftChoice[]>([]);
  const [currency, setCurrency] = useState<CurrencyFormat | null>(null);
  const [shopName, setShopName] = useState("");
  const [form, setForm] = useState<ReportForm>(() => ({
    kind: seesAll ? "daily_summary" : "shift",
    from: "",
    to: "",
    tillId: "",
    shiftId: "",
    salesBy: "product",
    group: "product",
    categoryId: "",
    brandId: "",
    cashierId: "",
    top: "",
    days: "30",
    supplierId: "",
    customerId: "",
  }));
  const [shown, setShown] = useState<Shown | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [printed, setPrinted] = useState<PrintResult | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));
  const today = choices?.today ?? new Date().toISOString().slice(0, 10);

  useEffect(() => {
    loadShifts()
      .then((list) => {
        setShifts(list.shifts);
        setCurrency(list.currency);
        setShopName(list.shop_name);
        setForm((current) => ({ ...current, shiftId: list.shifts[0]?.id ?? "" }));
      })
      .catch(fail);
    if (seesAll) {
      loadChoices()
        .then((found) => {
          setChoices(found);
          setForm((current) => ({ ...current, from: found.today, to: found.today }));
        })
        .catch(fail);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, []);

  const pick = (kind: ReportKind) => {
    setShown(null);
    setProblem(null);
    setNotice(null);
    setForm({ ...form, kind });
  };
  // Builds the request from the form, or shows why it can't yet.
  const request = (): ReportRequest | null => {
    const built = buildRequest(form);
    if (!built.ok) {
      setProblem(t(`problems.${built.problem}`));
      return null;
    }
    return built.request;
  };
  const clear = () => {
    setProblem(null);
    setNotice(null);
    setPrinted(null);
  };
  const show = () => {
    clear();
    const wanted = request();
    if (wanted === null) {
      return;
    }
    runReport(wanted)
      .then((report) =>
        setShown({ report, request: wanted, generatedAt: formatDateTime(Date.now()) }),
      )
      .catch(fail);
  };
  const printReceipt = () => {
    clear();
    if (shown === null) {
      return;
    }
    printReport(shown.request)
      .then((result) => (result.printed ? setNotice(t("done.printed")) : setPrinted(result)))
      .catch(fail);
  };
  const printA4 = () => {
    clear();
    printWindow().catch(fail);
  };
  const saveExcel = () => {
    clear();
    if (shown === null) {
      return;
    }
    const name = `${shown.report.title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}.xlsx`;
    pickSaveFile(t("actions.excel.saveTitle"), name)
      .then((file) =>
        file === null
          ? null
          : exportReport(shown.request, file).then(() => setNotice(t("done.exported"))),
      )
      .catch(fail);
  };
  const needReport = shown === null ? t("actions.needReport") : undefined;

  return (
    <section aria-labelledby="reports-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="reports-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>
      <div className="flex gap-6">
        <ReportList
          selected={form.kind}
          allowed={(kind) => seesAll || kind === "shift"}
          onPick={pick}
        />
        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <h3 className="text-xl font-semibold">{t(`reports.${form.kind}.label`)}</h3>
          <ReportParams
            key={form.kind}
            form={form}
            onChange={setForm}
            choices={choices}
            shifts={shifts}
            today={today}
          />
          <div className="flex flex-wrap gap-2">
            <ActionButton
              colour="blue"
              icon={ListChecks}
              label={t("actions.show.label")}
              tip={t("actions.show.tip")}
              onClick={show}
            />
            <ActionButton
              colour="blue"
              icon={ReceiptText}
              label={t("actions.print80.label")}
              tip={t("actions.print80.tip")}
              disabledReason={needReport}
              onClick={printReceipt}
            />
            <ActionButton
              colour="blue"
              icon={Printer}
              label={t("actions.printA4.label")}
              tip={t("actions.printA4.tip")}
              disabledReason={needReport}
              onClick={printA4}
            />
            <ActionButton
              colour="blue"
              icon={FileSpreadsheet}
              label={t("actions.excel.label")}
              tip={t("actions.excel.tip")}
              disabledReason={needReport}
              onClick={saveExcel}
            />
          </div>
          {problem ? <Alert kind="problem">{problem}</Alert> : null}
          {notice ? <Alert kind="done">{notice}</Alert> : null}
          {printed ? <PrintProblem result={printed} onGone={() => setPrinted(null)} /> : null}
          {shown && currency ? (
            <ReportView
              report={shown.report}
              currency={currency}
              shopName={choices?.shop_name ?? shopName}
              generatedAt={shown.generatedAt}
            />
          ) : null}
        </div>
      </div>
    </section>
  );
}
