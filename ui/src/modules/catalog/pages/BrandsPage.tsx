import { Archive, Pencil, Save, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Brand } from "../../../generated/Brand";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { archiveBrand, listBrands, saveBrand } from "../api";

export function BrandsPage() {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [brands, setBrands] = useState<Brand[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<Brand | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    listBrands(showArchived)
      .then(setBrands)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when the filter changes.
  useEffect(load, [showArchived]);

  const archive = (id: string) => {
    setProblem(null);
    archiveBrand(id)
      .then(load)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <section aria-labelledby="brands-heading" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 id="brands-heading" className="text-[28px] font-semibold">
            {t("brands.title")}
          </h2>
          <p className="text-text-muted">{t("brands.intro")}</p>
        </div>
        <CheckBox
          label={t("brands.showArchived.label")}
          tip={t("brands.showArchived.tip")}
          checked={showArchived}
          onChange={setShowArchived}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {brands.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("brands.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("brands.table.name")}</th>
              <th className="p-2.5">{t("brands.table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {brands.map((brand) => (
              <tr key={brand.id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="flex items-center gap-2 p-2.5 font-medium">
                  {brand.name}
                  {brand.archived_at !== null ? (
                    <Badge
                      colour="red"
                      icon={Archive}
                      word={t("brands.table.archived")}
                      tip={t("brands.table.archivedTip")}
                    />
                  ) : null}
                </td>
                <td className="p-2.5">
                  <span className="flex gap-2">
                    <IconButton
                      icon={Pencil}
                      tip={`${t("brands.edit.tip")} ${brand.name}`}
                      onClick={() => setEditing(brand)}
                    />
                    {brand.archived_at === null ? (
                      <IconButton
                        icon={Archive}
                        colour="red"
                        tip={`${t("brands.archive.tip")} ${brand.name}`}
                        onClick={() => archive(brand.id)}
                      />
                    ) : null}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <BrandForm
        key={editing?.id ?? "new"}
        brand={editing}
        onSaved={() => {
          setEditing(null);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </section>
  );
}

type BrandFormProps = { brand: Brand | null; onSaved: () => void; onCancel: () => void };

function BrandForm({ brand, onSaved, onCancel }: BrandFormProps) {
  const { t } = useTranslation("catalog");
  const [name, setName] = useState(brand?.name ?? "");
  const save = useSave();

  const submit = () =>
    save.run(
      () => saveBrand({ id: brand?.id ?? null, name }),
      () => {
        setName("");
        onSaved();
      },
    );

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {brand ? t("brands.form.editHeading", { name: brand.name }) : t("brands.form.addHeading")}
      </h3>
      <Field
        label={t("brands.form.name.label")}
        tip={t("brands.form.name.tip")}
        value={name}
        onChange={setName}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            submit();
          }
        }}
        error={save.errorFor("name")}
      />
      {save.problem && !save.errorFor("name") ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("brands.form.saved")}</Alert> : null}
      <div className="flex justify-between">
        {brand ? (
          <ActionButton
            colour="grey"
            icon={X}
            label={t("brands.form.cancel.label")}
            tip={t("brands.form.cancel.tip")}
            onClick={onCancel}
          />
        ) : (
          <span />
        )}
        <ActionButton
          colour="green"
          icon={Save}
          label={t("brands.form.save.label")}
          tip={t("brands.form.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
