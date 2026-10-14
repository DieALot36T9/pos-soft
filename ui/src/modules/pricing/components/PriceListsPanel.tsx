import { CheckCircle, ListOrdered, PauseCircle, Pencil, Save, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { PriceList } from "../../../generated/PriceList";
import type { PriceListItemRow } from "../../../generated/PriceListItemRow";
import type { SearchHit } from "../../../generated/SearchHit";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { listItems, listPriceLists, removeItems, savePriceList, setItems } from "../api";
import { ProductPicker } from "./ProductPicker";

export function PriceListsPanel({ currency }: { currency: CurrencyFormat }) {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const [lists, setLists] = useState<PriceList[]>([]);
  const [editing, setEditing] = useState<PriceList | null>(null);
  const [opened, setOpened] = useState<PriceList | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    listPriceLists()
      .then(setLists)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  useEffect(load, []);

  return (
    <div className="flex flex-col gap-4">
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {lists.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-6 text-center text-text-muted">
          {t("lists.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("common.name")}</th>
              <th className="p-2.5">{t("lists.status")}</th>
              <th className="p-2.5">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {lists.map((list) => (
              <tr key={list.id} className="border-b border-border hover:bg-blue-soft/30">
                <td className="p-2.5 font-semibold">{list.name}</td>
                <td className="p-2.5">
                  {list.active ? (
                    <Badge
                      colour="green"
                      icon={CheckCircle}
                      word={t("common.active")}
                      tip={t("common.activeTip")}
                    />
                  ) : (
                    <Badge
                      colour="amber"
                      icon={PauseCircle}
                      word={t("common.inactive")}
                      tip={t("common.inactiveTip")}
                    />
                  )}
                </td>
                <td className="flex gap-2 p-2.5">
                  <IconButton
                    icon={Pencil}
                    colour="purple"
                    tip={t("lists.edit")}
                    onClick={() => setEditing(list)}
                  />
                  <IconButton
                    icon={ListOrdered}
                    colour="blue"
                    tip={t("lists.open")}
                    onClick={() => setOpened(list)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {opened ? (
        <ItemsEditor
          key={opened.id}
          list={opened}
          currency={currency}
          onClose={() => setOpened(null)}
        />
      ) : null}
      <ListForm
        key={editing?.id ?? "new"}
        list={editing}
        onSaved={() => {
          setEditing(null);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </div>
  );
}

type ListFormProps = { list: PriceList | null; onSaved: () => void; onCancel: () => void };

function ListForm({ list, onSaved, onCancel }: ListFormProps) {
  const { t } = useTranslation("pricing");
  const [name, setName] = useState(list?.name ?? "");
  const [active, setActive] = useState(list?.active ?? true);
  const save = useSave();
  const submit = () =>
    save.run(
      () => savePriceList({ id: list?.id ?? null, name, active }),
      () => {
        setName("");
        setActive(true);
        onSaved();
      },
    );
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {list ? t("lists.editHeading", { name: list.name }) : t("lists.addHeading")}
      </h3>
      <Field
        label={t("lists.name.label")}
        tip={t("lists.name.tip")}
        value={name}
        onChange={setName}
        error={save.errorFor("name")}
      />
      <CheckBox
        label={t("lists.activeBox.label")}
        tip={t("lists.activeBox.tip")}
        checked={active}
        onChange={setActive}
      />
      {save.problem && !save.errorFor("name") ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("common.saved")}</Alert> : null}
      <FormButtons editing={list !== null} onCancel={onCancel} onSave={submit} />
    </section>
  );
}

type ItemsEditorProps = { list: PriceList; currency: CurrencyFormat; onClose: () => void };

function ItemsEditor({ list, currency, onClose }: ItemsEditorProps) {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const [rows, setRows] = useState<PriceListItemRow[]>([]);
  const [chosen, setChosen] = useState<SearchHit | null>(null);
  const [price, setPrice] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const money = (minor: number) => formatMoney(minor, currency);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  const load = () => {
    listItems(list.id).then(setRows).catch(fail);
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once per list.
  useEffect(load, []);

  const add = () => {
    if (!chosen) {
      setProblem(t("items.add.pickFirst"));
      return;
    }
    const parsed = parseMoney(price, currency.minor_digits);
    if (!parsed.ok) {
      setProblem(tErrors("money_not_a_number"));
      return;
    }
    setProblem(null);
    setItems(list.id, [{ product_id: chosen.product_id, price: parsed.minor }])
      .then(() => {
        setChosen(null);
        setPrice("");
        load();
      })
      .catch(fail);
  };
  const remove = (productId: string) => {
    removeItems(list.id, [productId]).then(load).catch(fail);
  };

  return (
    <section
      aria-label={t("items.heading", { name: list.name })}
      className="flex flex-col gap-3 rounded-ui border border-blue bg-panel p-4"
    >
      <h3 className="text-xl font-semibold">{t("items.heading", { name: list.name })}</h3>
      <p className="text-text-muted">{t("items.intro")}</p>
      {rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-bg p-4 text-text-muted">
          {t("items.empty")}
        </p>
      ) : (
        <table className="w-full border-collapse">
          <thead>
            <tr className="border-b border-border bg-bg text-left text-sm font-bold">
              <th className="p-2.5">{t("items.product")}</th>
              <th className="p-2.5">{t("items.sku")}</th>
              <th className="p-2.5 text-right">{t("items.normal")}</th>
              <th className="p-2.5 text-right">{t("items.price")}</th>
              <th className="p-2.5">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.product_id} className="border-b border-border">
                <td className="p-2.5 font-semibold">{row.name}</td>
                <td className="p-2.5 font-mono text-sm text-text-muted">{row.sku}</td>
                <td className="p-2.5 text-right tabular-nums">{money(row.retail_price)}</td>
                <td className="p-2.5 text-right font-bold text-purple tabular-nums">
                  {money(row.price)}
                </td>
                <td className="p-2.5">
                  <IconButton
                    icon={Trash2}
                    colour="red"
                    tip={t("items.remove", { name: row.name })}
                    onClick={() => remove(row.product_id)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <h4 className="font-semibold pt-2 border-t border-border">{t("items.add.heading")}</h4>
      <ProductPicker currency={currency} chosen={chosen} onChoose={setChosen} />
      <Field
        label={t("items.add.price.label")}
        tip={t("items.add.price.tip")}
        value={price}
        onChange={setPrice}
        width="short"
      />
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("items.close.label")}
          tip={t("items.close.tip")}
          onClick={onClose}
        />
        <ActionButton
          colour="green"
          icon={Save}
          label={t("items.add.save.label")}
          tip={t("items.add.save.tip")}
          onClick={add}
        />
      </div>
    </section>
  );
}

export function FormButtons(props: { editing: boolean; onCancel: () => void; onSave: () => void }) {
  const { t } = useTranslation("pricing");
  return (
    <div className="flex justify-between">
      {props.editing ? (
        <ActionButton
          colour="grey"
          icon={X}
          label={t("common.cancel.label")}
          tip={t("common.cancel.tip")}
          onClick={props.onCancel}
        />
      ) : (
        <span />
      )}
      <ActionButton
        colour="green"
        icon={Save}
        label={t("common.save.label")}
        tip={t("common.save.tip")}
        onClick={props.onSave}
      />
    </div>
  );
}
