import { Archive, Lock, Pencil, Save, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Unit } from "../../../generated/Unit";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { archiveUnit, listUnits, saveUnit } from "../api";

export function UnitsPage() {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [units, setUnits] = useState<Unit[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<Unit | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    listUnits(showArchived)
      .then(setUnits)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when the filter changes.
  useEffect(load, [showArchived]);

  const archive = (id: string) => {
    setProblem(null);
    archiveUnit(id)
      .then(load)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <section aria-labelledby="units-heading" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 id="units-heading" className="text-[28px] font-semibold">
            {t("units.title")}
          </h2>
          <p className="text-text-muted">{t("units.intro")}</p>
        </div>
        <CheckBox
          label={t("units.showArchived.label")}
          tip={t("units.showArchived.tip")}
          checked={showArchived}
          onChange={setShowArchived}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <table className="w-full border-collapse rounded-ui border border-border bg-panel">
        <thead>
          <tr className="border-b border-border bg-bg text-left text-sm font-bold">
            <th className="p-2.5">{t("units.table.name")}</th>
            <th className="p-2.5">{t("units.table.symbol")}</th>
            <th className="p-2.5 text-right">{t("units.table.decimals")}</th>
            <th className="p-2.5">{t("units.table.code")}</th>
            <th className="p-2.5">{t("units.table.kind")}</th>
            <th className="p-2.5">{t("units.table.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {units.map((unit) => (
            <tr key={unit.id} className="border-b border-border hover:bg-blue-soft/30">
              <td className="p-2.5 font-medium">{unit.name}</td>
              <td className="p-2.5 font-mono text-sm">{unit.symbol}</td>
              <td className="p-2.5 text-right tabular-nums">{unit.decimals}</td>
              <td className="p-2.5 font-mono text-sm">{unit.unit_code ?? ""}</td>
              <td className="flex flex-wrap gap-2 p-2.5">
                {unit.is_system ? (
                  <Badge
                    colour="grey"
                    icon={Lock}
                    word={t("units.table.builtIn")}
                    tip={t("units.table.builtInTip")}
                  />
                ) : null}
                {unit.archived_at !== null ? (
                  <Badge
                    colour="red"
                    icon={Archive}
                    word={t("units.table.archived")}
                    tip={t("units.table.archivedTip")}
                  />
                ) : null}
              </td>
              <td className="p-2.5">
                <span className="flex gap-2">
                  <IconButton
                    icon={Pencil}
                    tip={`${t("units.edit.tip")} ${unit.name}`}
                    onClick={() => setEditing(unit)}
                  />
                  {!unit.is_system && unit.archived_at === null ? (
                    <IconButton
                      icon={Archive}
                      colour="red"
                      tip={`${t("units.archive.tip")} ${unit.name}`}
                      onClick={() => archive(unit.id)}
                    />
                  ) : null}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <UnitForm
        key={editing?.id ?? "new"}
        unit={editing}
        onSaved={() => {
          setEditing(null);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </section>
  );
}

type UnitFormProps = { unit: Unit | null; onSaved: () => void; onCancel: () => void };

function UnitForm({ unit, onSaved, onCancel }: UnitFormProps) {
  const { t } = useTranslation("catalog");
  const [name, setName] = useState(unit?.name ?? "");
  const [symbol, setSymbol] = useState(unit?.symbol ?? "");
  const [decimals, setDecimals] = useState(String(unit?.decimals ?? 0));
  const [code, setCode] = useState(unit?.unit_code ?? "");
  const save = useSave();

  const submit = () =>
    save.run(
      () =>
        saveUnit({
          id: unit?.id ?? null,
          fields: {
            name,
            symbol,
            unit_code: code.trim() === "" ? null : code,
            decimals: Number(decimals),
          },
        }),
      () => {
        setName("");
        setSymbol("");
        setDecimals("0");
        setCode("");
        onSaved();
      },
    );

  const fieldProblem = ["name", "symbol", "decimals"].some(save.errorFor);
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {unit ? t("units.form.editHeading", { name: unit.name }) : t("units.form.addHeading")}
      </h3>
      <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
        <Field
          label={t("units.form.name.label")}
          tip={t("units.form.name.tip")}
          value={name}
          onChange={setName}
          error={save.errorFor("name")}
        />
        <Field
          label={t("units.form.symbol.label")}
          tip={t("units.form.symbol.tip")}
          value={symbol}
          onChange={setSymbol}
          error={save.errorFor("symbol")}
        />
        <SelectField
          label={t("units.form.decimals.label")}
          tip={t("units.form.decimals.tip")}
          value={decimals}
          options={["0", "1", "2", "3"].map((each) => ({ value: each, label: each }))}
          onChange={setDecimals}
          error={save.errorFor("decimals")}
        />
        <Field
          label={t("units.form.code.label")}
          tip={t("units.form.code.tip")}
          value={code}
          onChange={setCode}
        />
      </div>
      {save.problem && !fieldProblem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("units.form.saved")}</Alert> : null}
      <div className="flex justify-between">
        {unit ? (
          <ActionButton
            colour="grey"
            icon={X}
            label={t("units.form.cancel.label")}
            tip={t("units.form.cancel.tip")}
            onClick={onCancel}
          />
        ) : (
          <span />
        )}
        <ActionButton
          colour="green"
          icon={Save}
          label={t("units.form.save.label")}
          tip={t("units.form.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
