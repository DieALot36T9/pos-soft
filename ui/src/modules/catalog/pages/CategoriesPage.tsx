import { Archive, Pencil, Save, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Category } from "../../../generated/Category";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { archiveCategory, listCategories, saveCategory } from "../api";
import { categoryPaths } from "../components/categoryPaths";

const UNCATEGORISED = "00000000000000000000000010";

export function CategoriesPage() {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [categories, setCategories] = useState<Category[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<Category | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    listCategories(showArchived)
      .then(setCategories)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when the filter changes.
  useEffect(load, [showArchived]);

  const archive = (id: string) => {
    setProblem(null);
    archiveCategory(id)
      .then(load)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  const byId = new Map(categories.map((each) => [each.id, each]));
  return (
    <section aria-labelledby="categories-heading" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 id="categories-heading" className="text-[28px] font-semibold">
            {t("categories.title")}
          </h2>
          <p className="text-text-muted">{t("categories.intro")}</p>
        </div>
        <CheckBox
          label={t("categories.showArchived.label")}
          tip={t("categories.showArchived.tip")}
          checked={showArchived}
          onChange={setShowArchived}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <table className="w-full border-collapse rounded-ui border border-border bg-panel">
        <thead>
          <tr className="border-b border-border bg-bg text-left text-sm font-bold">
            <th className="p-2.5">{t("categories.table.name")}</th>
            <th className="p-2.5 text-right">{t("categories.table.order")}</th>
            <th className="p-2.5">{t("categories.table.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {categoryPaths(categories).map(({ id, path }) => {
            const category = byId.get(id);
            if (!category) {
              return null;
            }
            return (
              <tr key={id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="p-2.5">
                  <span className="flex items-center gap-2 font-medium">
                    {path}
                    {category.archived_at !== null ? (
                      <Badge
                        colour="red"
                        icon={Archive}
                        word={t("categories.table.archived")}
                        tip={t("categories.table.archivedTip")}
                      />
                    ) : null}
                  </span>
                </td>
                <td className="p-2.5 text-right tabular-nums">{category.sort}</td>
                <td className="p-2.5">
                  <span className="flex gap-2">
                    <IconButton
                      icon={Pencil}
                      tip={`${t("categories.edit.tip")} ${category.name}`}
                      onClick={() => setEditing(category)}
                    />
                    {id !== UNCATEGORISED && category.archived_at === null ? (
                      <IconButton
                        icon={Archive}
                        colour="red"
                        tip={`${t("categories.archive.tip")} ${category.name}`}
                        onClick={() => archive(id)}
                      />
                    ) : null}
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <CategoryForm
        key={editing?.id ?? "new"}
        category={editing}
        all={categories.filter((each) => each.archived_at === null)}
        onSaved={() => {
          setEditing(null);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </section>
  );
}

type CategoryFormProps = {
  category: Category | null;
  all: Category[];
  onSaved: () => void;
  onCancel: () => void;
};

function CategoryForm({ category, all, onSaved, onCancel }: CategoryFormProps) {
  const { t } = useTranslation("catalog");
  const [name, setName] = useState(category?.name ?? "");
  const [parentId, setParentId] = useState(category?.parent_id ?? "");
  const [sort, setSort] = useState(String(category?.sort ?? 0));
  const [sortProblem, setSortProblem] = useState<string | null>(null);
  const save = useSave();

  const submit = () => {
    if (!/^-?\d+$/.test(sort.trim())) {
      setSortProblem(t("categories.form.sortInvalid"));
      return;
    }
    setSortProblem(null);
    save.run(
      () =>
        saveCategory({
          id: category?.id ?? null,
          fields: { name, parent_id: parentId === "" ? null : parentId, sort: Number(sort) },
        }),
      () => {
        setName("");
        setParentId("");
        setSort("0");
        onSaved();
      },
    );
  };

  const fieldProblem = ["name", "parent_id"].some(save.errorFor);
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {category
          ? t("categories.form.editHeading", { name: category.name })
          : t("categories.form.addHeading")}
      </h3>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
        <Field
          label={t("categories.form.name.label")}
          tip={t("categories.form.name.tip")}
          value={name}
          onChange={setName}
          error={save.errorFor("name")}
        />
        <SelectField
          label={t("categories.form.parent.label")}
          tip={t("categories.form.parent.tip")}
          value={parentId}
          options={[
            { value: "", label: t("categories.form.top") },
            ...categoryPaths(all)
              .filter((each) => each.id !== category?.id)
              .map((each) => ({ value: each.id, label: each.path })),
          ]}
          onChange={setParentId}
          error={save.errorFor("parent_id")}
        />
        <Field
          label={t("categories.form.sort.label")}
          tip={t("categories.form.sort.tip")}
          value={sort}
          width="short"
          onChange={setSort}
          error={sortProblem}
        />
      </div>
      {save.problem && !fieldProblem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("categories.form.saved")}</Alert> : null}
      <div className="flex justify-between">
        {category ? (
          <ActionButton
            colour="grey"
            icon={X}
            label={t("categories.form.cancel.label")}
            tip={t("categories.form.cancel.tip")}
            onClick={onCancel}
          />
        ) : (
          <span />
        )}
        <ActionButton
          colour="green"
          icon={Save}
          label={t("categories.form.save.label")}
          tip={t("categories.form.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
