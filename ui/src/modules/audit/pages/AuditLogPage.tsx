import { ChevronLeft, ChevronRight, FileSpreadsheet, Search } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { AuditFilter } from "../../../generated/AuditFilter";
import type { AuditList } from "../../../generated/AuditList";
import type { AuditRow } from "../../../generated/AuditRow";
import { errorMessage } from "../../../shared/bridge";
import { pickSaveFile } from "../../../shared/bridge/pickers";
import { useSave } from "../../../shared/bridge/useSave";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { KitDrawer } from "../../../shared/ui/PosKit";
import { SelectField } from "../../../shared/ui/SelectField";
import { exportAudit, queryAudit } from "../api";
import { en } from "../text/en";

const PAGE_SIZE = 50;
const ANY = "";

type ActionKey = keyof typeof en.actions;

function actionKey(action: string): ActionKey | null {
  const key = action.replace(/\./g, "_");
  return Object.hasOwn(en.actions, key) ? (key as ActionKey) : null;
}

const looksLikeDate = (text: string) => text === "" || /^\d{4}-\d{2}-\d{2}$/.test(text);

export function AuditLogPage() {
  const { t } = useTranslation("audit");
  const { t: tErrors } = useTranslation("errors");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [userId, setUserId] = useState(ANY);
  const [action, setAction] = useState(ANY);
  const [filter, setFilter] = useState<AuditFilter>({
    from: null,
    to: null,
    user_id: null,
    action: null,
  });
  const [offset, setOffset] = useState(0);
  const [list, setList] = useState<AuditList | null>(null);
  const [selectedRow, setSelectedRow] = useState<AuditRow | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [dateProblem, setDateProblem] = useState<string | null>(null);
  const [exported, setExported] = useState<number | null>(null);
  const exporting = useSave();
  const colonSuffix = ": ";

  useEffect(() => {
    queryAudit(filter, { limit: PAGE_SIZE, offset })
      .then((found) => {
        setList(found);
        setProblem(null);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [filter, offset, tErrors]);

  const show = () => {
    if (!looksLikeDate(from.trim()) || !looksLikeDate(to.trim())) {
      setDateProblem(t("filter.dateInvalid"));
      return;
    }
    setDateProblem(null);
    setOffset(0);
    setFilter({
      from: from.trim() === "" ? null : from.trim(),
      to: to.trim() === "" ? null : to.trim(),
      user_id: userId === ANY ? null : userId,
      action: action === ANY ? null : action,
    });
  };

  const saveToExcel = () => {
    setExported(null);
    exporting.run(
      async () => {
        const file = await pickSaveFile(t("export.title"), "audit-log.xlsx");
        return file === null ? null : exportAudit(filter, file);
      },
      (count) => setExported(count),
    );
  };

  const nameOf = (id: string | null) => {
    if (id === null) {
      return "";
    }
    return list?.people.find((person) => person.id === id)?.display_name ?? id;
  };
  const actionName = (name: string) => {
    const key = actionKey(name);
    return key ? t(`actions.${key}`) : name;
  };
  const rows = list?.rows ?? [];

  return (
    <section aria-labelledby="audit-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between">
        <div>
          <h2 id="audit-heading" className="text-[28px] font-semibold">
            {t("page.title")}
          </h2>
          <p className="text-text-muted">{t("page.intro")}</p>
        </div>
        <ActionButton
          colour="blue"
          icon={FileSpreadsheet}
          label={t("export.label")}
          tip={t("export.tip")}
          onClick={saveToExcel}
        />
      </div>
      <div className="flex flex-wrap items-end gap-3 rounded-ui border border-border bg-panel p-4">
        <Field
          label={t("filter.from.label")}
          tip={t("filter.from.tip")}
          value={from}
          onChange={setFrom}
          width="short"
        />
        <Field
          label={t("filter.to.label")}
          tip={t("filter.to.tip")}
          value={to}
          onChange={setTo}
          width="short"
        />
        <SelectField
          label={t("filter.user.label")}
          tip={t("filter.user.tip")}
          value={userId}
          options={[
            { value: ANY, label: t("filter.anyone") },
            ...(list?.people ?? []).map((person) => ({
              value: person.id,
              label: person.display_name,
            })),
          ]}
          onChange={setUserId}
        />
        <SelectField
          label={t("filter.action.label")}
          tip={t("filter.action.tip")}
          value={action}
          options={[
            { value: ANY, label: t("filter.anything") },
            ...(list?.actions ?? []).map((name) => ({ value: name, label: actionName(name) })),
          ]}
          onChange={setAction}
        />
        <ActionButton
          colour="blue"
          icon={Search}
          label={t("filter.show.label")}
          tip={t("filter.show.tip")}
          onClick={show}
        />
      </div>
      {dateProblem ? <Alert kind="problem">{dateProblem}</Alert> : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {exporting.problem ? <Alert kind="problem">{exporting.problem}</Alert> : null}
      {exported !== null ? (
        <Alert kind="done">{t("export.done", { count: exported })}</Alert>
      ) : null}
      {rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("table.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("table.time")}</th>
              <th className="p-2.5">{t("table.user")}</th>
              <th className="p-2.5">{t("table.approvedBy")}</th>
              <th className="p-2.5">{t("table.action")}</th>
              <th className="p-2.5">{t("table.details")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr
                key={row.id}
                onClick={() => setSelectedRow(row)}
                className="border-b border-border align-top hover:bg-blue-soft/30 cursor-pointer"
              >
                <td className="p-2.5 tabular-nums whitespace-nowrap">{formatDateTime(row.at)}</td>
                <td className="p-2.5 font-semibold">
                  {row.user_id ? nameOf(row.user_id) : t("table.system")}
                </td>
                <td className="p-2.5">{nameOf(row.approved_by)}</td>
                <td className="p-2.5 font-medium">{actionName(row.action)}</td>
                <td className="p-2.5">
                  <ul className="text-sm">
                    {Object.entries(row.details).map(([key, value]) => (
                      <li key={key}>
                        <span className="text-text-muted">{t("table.detailKey", { key })}</span>{" "}
                        {typeof value === "string" ? value : JSON.stringify(value)}
                      </li>
                    ))}
                  </ul>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <div className="flex items-center justify-between">
        <ActionButton
          colour="grey"
          icon={ChevronLeft}
          label={t("pages.previous.label")}
          tip={t("pages.previous.tip")}
          disabledReason={offset === 0 ? t("pages.newest") : undefined}
          onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
        />
        {rows.length > 0 ? (
          <span className="text-text-muted">
            {t("pages.showing", { from: offset + 1, to: offset + rows.length })}
          </span>
        ) : null}
        <ActionButton
          colour="grey"
          icon={ChevronRight}
          label={t("pages.next.label")}
          tip={t("pages.next.tip")}
          disabledReason={rows.length < PAGE_SIZE ? t("pages.oldest") : undefined}
          onClick={() => setOffset(offset + PAGE_SIZE)}
        />
      </div>

      <KitDrawer
        open={selectedRow !== null}
        onClose={() => setSelectedRow(null)}
        title={selectedRow ? actionName(selectedRow.action) : ""}
        description={
          selectedRow ? `${formatDateTime(selectedRow.at)} · Day ${selectedRow.business_date}` : ""
        }
      >
        {selectedRow ? (
          <div className="space-y-3">
            <div className="rounded-ui border border-border bg-bg p-3 text-sm space-y-1">
              <div>
                <span className="font-bold">
                  {t("table.user")}
                  {colonSuffix}
                </span>
                <span>{selectedRow.user_id ? nameOf(selectedRow.user_id) : t("table.system")}</span>
              </div>
              {selectedRow.approved_by ? (
                <div>
                  <span className="font-bold">
                    {t("table.approvedBy")}
                    {colonSuffix}
                  </span>
                  <span>{nameOf(selectedRow.approved_by)}</span>
                </div>
              ) : null}
            </div>
            <div className="rounded-ui border border-border bg-panel p-3">
              <h3 className="text-xs font-bold uppercase text-text-muted mb-2">
                {t("table.details")}
              </h3>
              <dl className="space-y-1.5 text-sm">
                {Object.entries(selectedRow.details).map(([key, value]) => (
                  <div
                    key={key}
                    className="flex justify-between gap-4 border-b border-border/50 pb-1"
                  >
                    <dt className="text-text-muted font-medium">{key}</dt>
                    <dd className="font-mono text-right">
                      {typeof value === "string" ? value : JSON.stringify(value)}
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
          </div>
        ) : null}
      </KitDrawer>
    </section>
  );
}
