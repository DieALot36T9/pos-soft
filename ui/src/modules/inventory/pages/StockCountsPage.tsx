import { FolderOpen, Play } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Category } from "../../../generated/Category";
import type { CountScope } from "../../../generated/CountScope";
import type { CountSummary } from "../../../generated/CountSummary";
import { errorField, errorMessage } from "../../../shared/bridge";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { createCount, listCategories, listCounts } from "../api";
import { CountStatusBadge } from "../components/CountStatusBadge";
import { CountingScreen } from "../components/CountingScreen";

type StockCountsPageProps = {
  // The user may start, review, post and cancel counts (StockCountManage).
  canManage: boolean;
};

// The list of stock counts, "Start a count", and the count screen when one is open (INV-06).
export function StockCountsPage({ canManage }: StockCountsPageProps) {
  const { t } = useTranslation("inventory");
  const { t: tErrors } = useTranslation("errors");
  const [counts, setCounts] = useState<CountSummary[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [open, setOpen] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [scope, setScope] = useState<CountScope>("full");
  const [categoryId, setCategoryId] = useState("");
  const [location, setLocation] = useState("");
  const [problem, setProblem] = useState<{ field: string | null; message: string } | null>(null);

  const load = () => {
    listCounts({ offset: 0, limit: 100 })
      .then(setCounts)
      .catch((error: unknown) =>
        setProblem({ field: null, message: errorMessage(error, tErrors) }),
      );
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when a count closes.
  useEffect(load, [open]);
  const loadOnce = () => {
    if (canManage) {
      listCategories()
        .then(setCategories)
        .catch(() => setCategories([]));
    }
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the page opens.
  useEffect(loadOnce, []);

  const start = () => {
    createCount({
      name,
      scope,
      category_id: scope === "partial" && categoryId !== "" ? categoryId : null,
      brand_id: null,
      location: scope === "partial" && location.trim() !== "" ? location : null,
    })
      .then((id) => {
        setName("");
        setLocation("");
        setProblem(null);
        setOpen(id);
      })
      .catch((error: unknown) =>
        setProblem({ field: errorField(error), message: errorMessage(error, tErrors) }),
      );
  };

  if (open !== null) {
    return <CountingScreen countId={open} canManage={canManage} onBack={() => setOpen(null)} />;
  }

  return (
    <section aria-labelledby="counts-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="counts-heading" className="text-[28px] font-semibold">
          {t("counts.title")}
        </h2>
        <p className="text-text-muted">{t("counts.intro")}</p>
      </div>
      {canManage ? (
        <form
          aria-label={t("counts.start.title")}
          className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4"
          onSubmit={(event) => {
            event.preventDefault();
            start();
          }}
        >
          <h3 className="text-xl font-semibold">{t("counts.start.title")}</h3>
          <div className="grid grid-cols-2 gap-3">
            <Field
              label={t("counts.name.label")}
              tip={t("counts.name.tip")}
              value={name}
              onChange={setName}
              error={problem && problem.field !== null ? problem.message : null}
            />
            <SelectField<CountScope>
              label={t("counts.scope.label")}
              tip={t("counts.scope.tip")}
              value={scope}
              options={[
                { value: "full", label: t("counts.scope.full") },
                { value: "partial", label: t("counts.scope.partial") },
              ]}
              onChange={setScope}
            />
            {scope === "partial" ? (
              <>
                <SelectField
                  label={t("counts.category.label")}
                  tip={t("counts.category.tip")}
                  value={categoryId}
                  options={[
                    { value: "", label: t("counts.category.none") },
                    ...categories.map((each) => ({ value: each.id, label: each.name })),
                  ]}
                  onChange={setCategoryId}
                />
                <Field
                  label={t("counts.location.label")}
                  tip={t("counts.location.tip")}
                  value={location}
                  onChange={setLocation}
                />
              </>
            ) : null}
          </div>
          <div>
            <ActionButton
              colour="green"
              icon={Play}
              label={t("counts.start.label")}
              tip={t("counts.start.tip")}
              onClick={start}
            />
          </div>
        </form>
      ) : null}
      {problem && problem.field === null ? <Alert kind="problem">{problem.message}</Alert> : null}
      {counts.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("counts.table.empty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("counts.table.number")}</th>
              <th className="p-2">{t("counts.table.name")}</th>
              <th className="p-2">{t("counts.table.covers")}</th>
              <th className="p-2">{t("counts.table.status")}</th>
              <th className="p-2">{t("counts.table.started")}</th>
              <th className="p-2">{t("counts.table.by")}</th>
              <th className="p-2">{t("counts.table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {counts.map((count) => (
              <tr key={count.id} className="border-b border-border">
                <td className="p-2 tabular-nums">{count.number}</td>
                <td className="p-2">{count.name}</td>
                <td className="p-2">
                  {count.scope === "full" ? t("counts.table.full") : count.covers}
                </td>
                <td className="p-2">
                  <CountStatusBadge status={count.status} />
                </td>
                <td className="p-2 tabular-nums">{formatDateTime(count.created_at)}</td>
                <td className="p-2">{count.created_by_name ?? ""}</td>
                <td className="p-2">
                  <IconButton
                    icon={FolderOpen}
                    colour="blue"
                    tip={`${t("counts.open.tip")} ${count.number}`}
                    onClick={() => setOpen(count.id)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
