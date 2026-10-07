import { Search } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { Choice } from "../../../generated/Choice";
import type { ReportChoices } from "../../../generated/ReportChoices";
import type { ReportGroup } from "../../../generated/ReportGroup";
import type { SalesBy } from "../../../generated/SalesBy";
import type { ShiftChoice } from "../../../generated/ShiftChoice";
import { errorMessage } from "../../../shared/bridge";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { findCustomers } from "../api";
import { PRESETS, presetRange, type Preset } from "../dates";
import { NEEDS, type ReportForm } from "../request";

const SALES_BY: SalesBy[] = ["product", "category", "brand", "cashier", "hour", "weekday", "date"];
const GROUPS: ReportGroup[] = ["product", "category", "brand"];

type ReportParamsProps = {
  form: ReportForm;
  onChange: (form: ReportForm) => void;
  // Null for a user who sees only their own shifts.
  choices: ReportChoices | null;
  shifts: ShiftChoice[];
  today: string;
};

// The choices for the picked report: days, filters, shift, supplier or customer.
export function ReportParams({ form, onChange, choices, shifts, today }: ReportParamsProps) {
  const { t } = useTranslation("reports");
  const { t: tErrors } = useTranslation("errors");
  const [preset, setPreset] = useState<Preset>("today");
  const [customerText, setCustomerText] = useState("");
  const [customers, setCustomers] = useState<Choice[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const needs = NEEDS[form.kind];
  const set = (change: Partial<ReportForm>) => onChange({ ...form, ...change });
  const all = (list: Choice[] | undefined) => [
    { value: "", label: t("common.none") },
    ...(list ?? []).map((each) => ({ value: each.id, label: each.name })),
  ];

  const pickPreset = (next: Preset) => {
    setPreset(next);
    if (next !== "custom") {
      set(presetRange(next, today));
    }
  };
  const find = () => {
    setProblem(null);
    findCustomers(customerText.trim())
      .then((found) => {
        setCustomers(found);
        set({ customerId: found[0]?.id ?? "" });
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const presets = (
    needs.day ? PRESETS.filter((each) => each !== "this_week" && each !== "this_month") : PRESETS
  ).map((each) => ({ value: each, label: t(`params.preset.${each}`) }));

  return (
    <div className="flex flex-wrap items-end gap-3">
      {needs.range || needs.day ? (
        <SelectField
          label={t("params.preset.label")}
          tip={t("params.preset.tip")}
          value={preset}
          options={presets}
          onChange={pickPreset}
        />
      ) : null}
      {needs.day ? (
        <Field
          label={t("params.date.label")}
          tip={t("params.date.tip")}
          value={form.from}
          onChange={(from) => {
            setPreset("custom");
            set({ from, to: from });
          }}
          width="short"
        />
      ) : null}
      {needs.range ? (
        <>
          <Field
            label={t("params.from.label")}
            tip={t("params.from.tip")}
            value={form.from}
            onChange={(from) => {
              setPreset("custom");
              set({ from });
            }}
            width="short"
          />
          <Field
            label={t("params.to.label")}
            tip={t("params.to.tip")}
            value={form.to}
            onChange={(to) => {
              setPreset("custom");
              set({ to });
            }}
            width="short"
          />
        </>
      ) : null}
      {form.kind === "daily_summary" ? (
        <SelectField
          label={t("params.till.label")}
          tip={t("params.till.tip")}
          value={form.tillId}
          options={all(choices?.tills)}
          onChange={(tillId) => set({ tillId })}
        />
      ) : null}
      {form.kind === "shift" ? (
        shifts.length === 0 ? (
          <p>{t("params.shift.empty")}</p>
        ) : (
          <SelectField
            label={t("params.shift.label")}
            tip={t("params.shift.tip")}
            value={form.shiftId}
            options={shifts.map((shift) => ({
              value: shift.id,
              label: `${shift.business_date} · ${shift.till} · ${shift.cashier} · ${shift.opened} · ${
                shift.closed ? t("params.shift.closed") : t("params.shift.open")
              }`,
            }))}
            onChange={(shiftId) => set({ shiftId })}
          />
        )
      ) : null}
      {form.kind === "sales_by" ? (
        <>
          <SelectField
            label={t("params.by.label")}
            tip={t("params.by.tip")}
            value={form.salesBy}
            options={SALES_BY.map((each) => ({ value: each, label: t(`params.by.${each}`) }))}
            onChange={(salesBy) => set({ salesBy })}
          />
          <SelectField
            label={t("params.category.label")}
            tip={t("params.category.tip")}
            value={form.categoryId}
            options={all(choices?.categories)}
            onChange={(categoryId) => set({ categoryId })}
          />
          <SelectField
            label={t("params.brand.label")}
            tip={t("params.brand.tip")}
            value={form.brandId}
            options={all(choices?.brands)}
            onChange={(brandId) => set({ brandId })}
          />
          <SelectField
            label={t("params.cashier.label")}
            tip={t("params.cashier.tip")}
            value={form.cashierId}
            options={all(choices?.cashiers)}
            onChange={(cashierId) => set({ cashierId })}
          />
          <Field
            label={t("params.top.label")}
            tip={t("params.top.tip")}
            value={form.top}
            onChange={(top) => set({ top })}
            width="short"
          />
        </>
      ) : null}
      {needs.group ? (
        <SelectField
          label={t("params.by.label")}
          tip={t("params.by.tip")}
          value={form.group}
          options={GROUPS.map((each) => ({ value: each, label: t(`params.by.${each}`) }))}
          onChange={(group) => set({ group })}
        />
      ) : null}
      {form.kind === "near_expiry" ? (
        <Field
          label={t("params.days.label")}
          tip={t("params.days.tip")}
          value={form.days}
          onChange={(days) => set({ days })}
          width="short"
        />
      ) : null}
      {form.kind === "supplier_statement" ? (
        <SelectField
          label={t("params.supplier.label")}
          tip={t("params.supplier.tip")}
          value={form.supplierId}
          options={[
            { value: "", label: t("params.supplier.pick") },
            ...(choices?.suppliers ?? []).map((each) => ({ value: each.id, label: each.name })),
          ]}
          onChange={(supplierId) => set({ supplierId })}
        />
      ) : null}
      {form.kind === "customer_statement" ? (
        <>
          <Field
            label={t("params.customerSearch.label")}
            tip={t("params.customerSearch.tip")}
            value={customerText}
            onChange={setCustomerText}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                find();
              }
            }}
            width="short"
          />
          <ActionButton
            colour="blue"
            icon={Search}
            label={t("params.find.label")}
            tip={t("params.find.tip")}
            onClick={find}
          />
          {customers.length === 0 ? (
            <p>{t("params.customer.none")}</p>
          ) : (
            <SelectField
              label={t("params.customer.label")}
              tip={t("params.customer.tip")}
              value={form.customerId}
              options={customers.map((each) => ({ value: each.id, label: each.name }))}
              onChange={(customerId) => set({ customerId })}
            />
          )}
        </>
      ) : null}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
    </div>
  );
}
