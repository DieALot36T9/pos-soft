import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ReportTable } from "../../../generated/ReportTable";
import { alignRight, cellText, isNegative } from "../cells";

type ReportViewProps = {
  report: ReportTable;
  currency: CurrencyFormat;
  shopName: string;
  // When the report was made, as "2026-03-02 14:05", for the printed A4 page.
  generatedAt: string;
};

// Any report: the shop, title, period and filters, then the table and its sections.
// The same view is the A4 page: only this part prints (data-print-area, ADR-013).
export function ReportView({ report, currency, shopName, generatedAt }: ReportViewProps) {
  const { t } = useTranslation("reports");
  const { from, to, filters } = report.params;
  let period: string | null = null;
  if (from !== null && to !== null && from !== to) {
    period = t("view.period", { from, to });
  } else if (from !== null || to !== null) {
    period = t("view.day", { date: from ?? to });
  }
  return (
    <article
      data-print-area=""
      aria-label={report.title}
      className="report-page flex flex-col gap-3 rounded-ui border border-border bg-panel p-4"
    >
      <header className="flex flex-col gap-1">
        <p className="text-text-muted">{shopName}</p>
        <h3 className="text-[28px] font-semibold">{report.title}</h3>
        {period ? <p>{period}</p> : null}
        {filters.map((filter) => (
          <p key={filter.name}>{t("view.filter", { name: filter.name, value: filter.value })}</p>
        ))}
        <p className="text-sm text-text-muted">{t("view.generated", { time: generatedAt })}</p>
      </header>
      <TableBody report={report} currency={currency} />
      {report.sections.map((section, index) => (
        <Section key={`${section.title}-${index}`} report={section} currency={currency} />
      ))}
    </article>
  );
}

// A smaller table inside the report, with its own title. Sections can hold sections.
function Section({ report, currency }: { report: ReportTable; currency: CurrencyFormat }) {
  return (
    <section className="flex flex-col gap-2 pt-2">
      <h4 className="text-xl font-semibold">{report.title}</h4>
      <TableBody report={report} currency={currency} />
      {report.sections.map((section, index) => (
        <Section key={`${section.title}-${index}`} report={section} currency={currency} />
      ))}
    </section>
  );
}

// The column titles, the rows, and the totals in bold. Numbers line up on the right.
function TableBody({ report, currency }: { report: ReportTable; currency: CurrencyFormat }) {
  const { t } = useTranslation("reports");
  if (report.columns.length === 0) {
    return null;
  }
  const cellClass = (index: number, negative: boolean) =>
    "p-2 " +
    (report.columns[index] && alignRight(report.columns[index].kind)
      ? "text-right tabular-nums "
      : "") +
    (negative ? "text-red" : "");
  return (
    <table className="w-full border-collapse">
      <thead>
        <tr className="border-b border-border">
          {report.columns.map((column) => (
            <th
              key={column.key}
              scope="col"
              className={"p-2 " + (alignRight(column.kind) ? "text-right" : "text-left")}
            >
              {column.title}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {report.rows.length === 0 ? (
          <tr>
            <td className="p-2 text-text-muted" colSpan={report.columns.length}>
              {t("view.empty")}
            </td>
          </tr>
        ) : null}
        {report.rows.map((row, rowIndex) => (
          <tr key={rowIndex} className="border-b border-border">
            {row.map((cell, index) => (
              <td
                key={report.columns[index]?.key ?? index}
                className={cellClass(index, isNegative(cell))}
              >
                {cellText(cell, currency)}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
      {report.totals ? (
        <tfoot>
          <tr className="border-t-2 border-text font-semibold">
            {report.totals.map((cell, index) => (
              <td
                key={report.columns[index]?.key ?? index}
                className={cellClass(index, isNegative(cell))}
              >
                {index === 0 && cell.kind === "empty" ? t("view.total") : cellText(cell, currency)}
              </td>
            ))}
          </tr>
        </tfoot>
      ) : null}
    </table>
  );
}
