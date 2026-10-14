import { Archive, Pencil } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { PriceList } from "../../../generated/PriceList";
import type { Tier } from "../../../generated/Tier";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatPercent, parsePercent } from "../../../shared/format/percent";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { archiveTier, listPriceLists, listTiers, saveTier } from "../api";
import { FormButtons } from "./PriceListsPanel";

export function TiersPanel() {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const [tiers, setTiers] = useState<Tier[]>([]);
  const [lists, setLists] = useState<PriceList[]>([]);
  const [editing, setEditing] = useState<Tier | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  const load = () => {
    Promise.all([listTiers(), listPriceLists()])
      .then(([foundTiers, foundLists]) => {
        setTiers(foundTiers);
        setLists(foundLists);
      })
      .catch(fail);
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  useEffect(load, []);
  const listName = (id: string | null) =>
    lists.find((list) => list.id === id)?.name ?? t("tiers.noList");

  return (
    <div className="flex flex-col gap-4">
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {tiers.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("tiers.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("common.name")}</th>
              <th className="p-2.5 text-right">{t("tiers.discount")}</th>
              <th className="p-2.5">{t("tiers.priceList")}</th>
              <th className="p-2.5">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {tiers.map((tier) => (
              <tr key={tier.id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="p-2.5 font-semibold">
                  {tier.name}
                  {tier.archived_at !== null ? (
                    <span className="ml-2">
                      <Badge
                        colour="grey"
                        icon={Archive}
                        word={t("common.archived")}
                        tip={t("common.archivedTip")}
                      />
                    </span>
                  ) : null}
                </td>
                <td className="p-2.5 text-right font-semibold tabular-nums">
                  {formatPercent(tier.discount)}
                </td>
                <td className="p-2.5">{listName(tier.price_list_id)}</td>
                <td className="flex gap-2 p-2.5">
                  {tier.archived_at === null ? (
                    <>
                      <IconButton
                        icon={Pencil}
                        colour="purple"
                        tip={t("tiers.edit")}
                        onClick={() => setEditing(tier)}
                      />
                      <IconButton
                        icon={Archive}
                        colour="red"
                        tip={t("tiers.archive")}
                        onClick={() => {
                          archiveTier(tier.id).then(load).catch(fail);
                        }}
                      />
                    </>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <TierForm
        key={editing?.id ?? "new"}
        tier={editing}
        lists={lists}
        onSaved={() => {
          setEditing(null);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </div>
  );
}

type TierFormProps = {
  tier: Tier | null;
  lists: PriceList[];
  onSaved: () => void;
  onCancel: () => void;
};

function TierForm({ tier, lists, onSaved, onCancel }: TierFormProps) {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const [name, setName] = useState(tier?.name ?? "");
  const [discount, setDiscount] = useState(
    tier ? formatPercent(tier.discount).replace("%", "") : "0",
  );
  const [listId, setListId] = useState(tier?.price_list_id ?? "");
  const [discountProblem, setDiscountProblem] = useState<string | null>(null);
  const save = useSave();

  const submit = () => {
    const parsed = parsePercent(discount);
    if (!parsed.ok) {
      setDiscountProblem(tErrors("tier_discount_invalid"));
      return;
    }
    setDiscountProblem(null);
    save.run(
      () =>
        saveTier({
          id: tier?.id ?? null,
          name,
          price_list_id: listId === "" ? null : listId,
          discount: parsed.bps,
          sort: tier?.sort ?? 0,
        }),
      () => {
        setName("");
        setDiscount("0");
        setListId("");
        onSaved();
      },
    );
  };

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {tier ? t("tiers.editHeading", { name: tier.name }) : t("tiers.addHeading")}
      </h3>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("tiers.name.label")}
          tip={t("tiers.name.tip")}
          value={name}
          onChange={setName}
          error={save.errorFor("name")}
        />
        <Field
          label={t("tiers.discountField.label")}
          tip={t("tiers.discountField.tip")}
          value={discount}
          onChange={setDiscount}
          error={discountProblem ?? save.errorFor("discount")}
          width="short"
        />
        <SelectField
          label={t("tiers.listField.label")}
          tip={t("tiers.listField.tip")}
          value={listId}
          options={[
            { value: "", label: t("tiers.noList") },
            ...lists.map((list) => ({ value: list.id, label: list.name })),
          ]}
          onChange={setListId}
        />
      </div>
      {save.problem && !save.errorFor("name") && !save.errorFor("discount") ? (
        <Alert kind="problem">{save.problem}</Alert>
      ) : null}
      {save.done ? <Alert kind="done">{t("common.saved")}</Alert> : null}
      <FormButtons editing={tier !== null} onCancel={onCancel} onSave={submit} />
    </section>
  );
}
