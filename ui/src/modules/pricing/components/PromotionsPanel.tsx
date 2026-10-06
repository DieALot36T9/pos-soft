import { CheckCircle, PauseCircle, Pencil, Play, Plus, Square, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { ProductChoices } from "../../../generated/ProductChoices";
import type { PromoRule } from "../../../generated/PromoRule";
import type { PromoTarget } from "../../../generated/PromoTarget";
import type { Promotion } from "../../../generated/Promotion";
import type { SearchHit } from "../../../generated/SearchHit";
import type { Tier } from "../../../generated/Tier";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { formatPercent, parsePercent } from "../../../shared/format/percent";
import { formatDateTime } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { listPromotions, listTiers, productName, savePromotion, setPromotionActive } from "../api";
import { FormButtons } from "./PriceListsPanel";
import { ProductPicker } from "./ProductPicker";

type RuleKind = PromoRule["kind"];
const RULE_KINDS: RuleKind[] = [
  "percent_off",
  "amount_off",
  "fixed_price",
  "buy_x_get_y",
  "multi_buy",
];
const EVERY_DAY = 0b111_1111;
const DAYS = [0, 1, 2, 3, 4, 5, 6] as const;

// A day typed as "2026-10-01", as the first moment of that day on this computer.
function parseDay(text: string): number | null | "bad" {
  if (text.trim() === "") {
    return null;
  }
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text.trim());
  if (!match) {
    return "bad";
  }
  return new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3])).getTime();
}
const showDay = (at: number | null) => (at === null ? "" : formatDateTime(at).slice(0, 10));

// Promotions that apply by themselves at the till (PRC-04). Rust works out the savings.
export function PromotionsPanel({ choices }: { choices: ProductChoices }) {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const [promotions, setPromotions] = useState<Promotion[]>([]);
  const [tiers, setTiers] = useState<Tier[]>([]);
  const [editing, setEditing] = useState<Promotion | null>(null);
  // Counts saves, so the "add" form starts empty again after each one.
  const [saves, setSaves] = useState(0);
  const [problem, setProblem] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));
  const money = (minor: number) => formatMoney(minor, choices.currency);

  const load = () => {
    Promise.all([listPromotions(), listTiers()])
      .then(([foundPromotions, foundTiers]) => {
        setPromotions(foundPromotions);
        setTiers(foundTiers.filter((tier) => tier.archived_at === null));
      })
      .catch(fail);
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  useEffect(load, []);

  const summary = (rule: PromoRule) => {
    switch (rule.kind) {
      case "percent_off":
        return t("promotions.summary.percent_off", { percent: formatPercent(rule.percent) });
      case "amount_off":
        return t("promotions.summary.amount_off", { amount: money(rule.amount_per_unit) });
      case "fixed_price":
        return t("promotions.summary.fixed_price", { price: money(rule.price_per_unit) });
      case "buy_x_get_y":
        return t("promotions.summary.buy_x_get_y", { buy: rule.buy, get: rule.get });
      case "multi_buy":
        return t("promotions.summary.multi_buy", {
          qty: rule.bundle_qty,
          price: money(rule.bundle_price),
        });
    }
  };
  const when = (promotion: Promotion) => {
    const from = showDay(promotion.starts_at);
    const until = showDay(promotion.ends_at);
    if (from && until) {
      return t("promotions.fromUntil", { from, until });
    }
    if (from) {
      return t("promotions.from", { date: from });
    }
    return until ? t("promotions.until", { date: until }) : t("promotions.always");
  };

  return (
    <div className="flex flex-col gap-4">
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {promotions.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("promotions.empty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("common.name")}</th>
              <th className="p-2">{t("promotions.what")}</th>
              <th className="p-2">{t("promotions.covers")}</th>
              <th className="p-2">{t("promotions.when")}</th>
              <th className="p-2">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {promotions.map((promotion) => (
              <tr key={promotion.id} className="border-b border-border">
                <td className="p-2">
                  {promotion.name}{" "}
                  {promotion.active ? (
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
                <td className="p-2 text-purple">{summary(promotion.rule)}</td>
                <td className="p-2">
                  {t("promotions.targets.count", { count: promotion.targets.length })}
                </td>
                <td className="p-2">{when(promotion)}</td>
                <td className="flex gap-2 p-2">
                  <IconButton
                    icon={Pencil}
                    colour="purple"
                    tip={t("promotions.edit")}
                    onClick={() => setEditing(promotion)}
                  />
                  <IconButton
                    icon={promotion.active ? Square : Play}
                    colour={promotion.active ? "amber" : "green"}
                    tip={promotion.active ? t("promotions.switchOff") : t("promotions.switchOn")}
                    onClick={() => {
                      setPromotionActive(promotion.id, !promotion.active).then(load).catch(fail);
                    }}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <PromotionForm
        key={editing?.id ?? `new-${String(saves)}`}
        promotion={editing}
        choices={choices}
        tiers={tiers}
        onSaved={() => {
          setEditing(null);
          setSaves((old) => old + 1);
          load();
        }}
        onCancel={() => setEditing(null)}
      />
    </div>
  );
}

type PromotionFormProps = {
  promotion: Promotion | null;
  choices: ProductChoices;
  tiers: Tier[];
  onSaved: () => void;
  onCancel: () => void;
};

// The typed values of each kind of rule, kept as text until Save.
function ruleTexts(rule: PromoRule | undefined, plain: (minor: number) => string) {
  return {
    percent: rule?.kind === "percent_off" ? formatPercent(rule.percent).replace("%", "") : "",
    amount: rule?.kind === "amount_off" ? plain(rule.amount_per_unit) : "",
    price: rule?.kind === "fixed_price" ? plain(rule.price_per_unit) : "",
    buy: rule?.kind === "buy_x_get_y" ? String(rule.buy) : "",
    get: rule?.kind === "buy_x_get_y" ? String(rule.get) : "",
    bundleQty: rule?.kind === "multi_buy" ? String(rule.bundle_qty) : "",
    bundlePrice: rule?.kind === "multi_buy" ? plain(rule.bundle_price) : "",
  };
}

function PromotionForm({ promotion, choices, tiers, onSaved, onCancel }: PromotionFormProps) {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const plain = (minor: number) =>
    formatMoney(minor, { ...choices.currency, symbol: "", group_sep: "", decimal_sep: "." });
  const [name, setName] = useState(promotion?.name ?? "");
  const [kind, setKind] = useState<RuleKind>(promotion?.rule.kind ?? "percent_off");
  const [texts, setTexts] = useState(() => ruleTexts(promotion?.rule, plain));
  const [starts, setStarts] = useState(showDay(promotion?.starts_at ?? null));
  const [ends, setEnds] = useState(showDay(promotion?.ends_at ?? null));
  const [days, setDays] = useState(promotion?.days_mask ?? EVERY_DAY);
  const [tierId, setTierId] = useState(promotion?.tier_id ?? "");
  const [targets, setTargets] = useState<PromoTarget[]>(promotion?.targets ?? []);
  const [product, setProduct] = useState<SearchHit | null>(null);
  const [names, setNames] = useState<Record<string, string>>({});
  const [categoryId, setCategoryId] = useState("");
  const [brandId, setBrandId] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const save = useSave();
  // A saved promotion keeps only product ids: look their names up once.
  useEffect(() => {
    for (const target of promotion?.targets ?? []) {
      if (target.type === "product") {
        productName(target.id)
          .then((found) => setNames((old) => ({ ...old, [target.id]: found })))
          .catch(() => undefined);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- once per promotion opened.
  }, []);
  const setText = (key: keyof typeof texts) => (value: string) =>
    setTexts((old) => ({ ...old, [key]: value }));

  const count = (text: string) => {
    const value = Number(text);
    return Number.isInteger(value) && value > 0 ? value : null;
  };
  const amount = (text: string) => {
    const parsed = parseMoney(text, choices.currency.minor_digits);
    return parsed.ok ? parsed.minor : null;
  };
  // The rule from what was typed, or the message to show.
  const rule = (): PromoRule | string => {
    switch (kind) {
      case "percent_off": {
        const parsed = parsePercent(texts.percent);
        return parsed.ok ? { kind, percent: parsed.bps } : tErrors("promotion_percent_invalid");
      }
      case "amount_off": {
        const value = amount(texts.amount);
        return value === null ? tErrors("money_not_a_number") : { kind, amount_per_unit: value };
      }
      case "fixed_price": {
        const value = amount(texts.price);
        return value === null ? tErrors("money_not_a_number") : { kind, price_per_unit: value };
      }
      case "buy_x_get_y": {
        const buy = count(texts.buy);
        const get = count(texts.get);
        return buy === null || get === null ? t("promotions.numberInvalid") : { kind, buy, get };
      }
      case "multi_buy": {
        const qty = count(texts.bundleQty);
        const price = amount(texts.bundlePrice);
        if (qty === null) {
          return t("promotions.numberInvalid");
        }
        return price === null
          ? tErrors("money_not_a_number")
          : { kind, bundle_qty: qty, bundle_price: price };
      }
    }
  };

  const submit = () => {
    const built = rule();
    const from = parseDay(starts);
    const until = parseDay(ends);
    if (typeof built === "string") {
      setProblem(built);
      return;
    }
    if (from === "bad" || until === "bad") {
      setProblem(t("promotions.dateInvalid"));
      return;
    }
    setProblem(null);
    save.run(
      () =>
        savePromotion({
          id: promotion?.id ?? null,
          promotion: {
            name,
            rule: built,
            starts_at: from,
            ends_at: until,
            days_mask: days,
            tier_id: tierId === "" ? null : tierId,
            targets,
          },
        }),
      onSaved,
    );
  };

  const addTarget = (target: PromoTarget, label: string) => {
    if (!targets.some((each) => each.type === target.type && each.id === target.id)) {
      setTargets((old) => [...old, target]);
    }
    setNames((old) => ({ ...old, [target.id]: label }));
  };
  const targetLabel = (target: PromoTarget) => {
    const known =
      names[target.id] ??
      (target.type === "category"
        ? choices.categories.find((each) => each.id === target.id)?.name
        : target.type === "brand"
          ? choices.brands.find((each) => each.id === target.id)?.name
          : undefined) ??
      target.id;
    return t(`promotions.targets.${target.type}`, { name: known });
  };

  const fields: Record<RuleKind, React.ReactNode> = {
    percent_off: (
      <Field
        label={t("promotions.percent.label")}
        tip={t("promotions.percent.tip")}
        value={texts.percent}
        onChange={setText("percent")}
        width="short"
      />
    ),
    amount_off: (
      <Field
        label={t("promotions.amount.label")}
        tip={t("promotions.amount.tip")}
        value={texts.amount}
        onChange={setText("amount")}
        width="short"
      />
    ),
    fixed_price: (
      <Field
        label={t("promotions.price.label")}
        tip={t("promotions.price.tip")}
        value={texts.price}
        onChange={setText("price")}
        width="short"
      />
    ),
    buy_x_get_y: (
      <>
        <Field
          label={t("promotions.buy.label")}
          tip={t("promotions.buy.tip")}
          value={texts.buy}
          onChange={setText("buy")}
          width="short"
        />
        <Field
          label={t("promotions.get.label")}
          tip={t("promotions.get.tip")}
          value={texts.get}
          onChange={setText("get")}
          width="short"
        />
      </>
    ),
    multi_buy: (
      <>
        <Field
          label={t("promotions.bundleQty.label")}
          tip={t("promotions.bundleQty.tip")}
          value={texts.bundleQty}
          onChange={setText("bundleQty")}
          width="short"
        />
        <Field
          label={t("promotions.bundlePrice.label")}
          tip={t("promotions.bundlePrice.tip")}
          value={texts.bundlePrice}
          onChange={setText("bundlePrice")}
          width="short"
        />
      </>
    ),
  };

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {promotion
          ? t("promotions.editHeading", { name: promotion.name })
          : t("promotions.addHeading")}
      </h3>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("promotions.name.label")}
          tip={t("promotions.name.tip")}
          value={name}
          onChange={setName}
          error={save.errorFor("name")}
        />
        <SelectField<RuleKind>
          label={t("promotions.rule.label")}
          tip={t("promotions.rule.tip")}
          value={kind}
          options={RULE_KINDS.map((each) => ({
            value: each,
            label: t(`promotions.rule.${each}`),
          }))}
          onChange={setKind}
        />
        {fields[kind]}
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("promotions.starts.label")}
          tip={t("promotions.starts.tip")}
          value={starts}
          onChange={setStarts}
          width="short"
        />
        <Field
          label={t("promotions.ends.label")}
          tip={t("promotions.ends.tip")}
          value={ends}
          onChange={setEnds}
          width="short"
        />
        <SelectField
          label={t("promotions.tier.label")}
          tip={t("promotions.tier.tip")}
          value={tierId}
          options={[
            { value: "", label: t("promotions.tier.everyone") },
            ...tiers.map((tier) => ({ value: tier.id, label: tier.name })),
          ]}
          onChange={setTierId}
        />
      </div>
      <fieldset className="flex flex-wrap gap-3">
        <legend className="font-semibold">{t("promotions.days.label")}</legend>
        {DAYS.map((day) => (
          <CheckBox
            key={day}
            label={t(`promotions.dayNames.d${day}`)}
            tip={t("promotions.days.tip")}
            checked={(days & (1 << day)) !== 0}
            onChange={(on) => setDays((old) => (on ? old | (1 << day) : old & ~(1 << day)))}
          />
        ))}
      </fieldset>
      <section className="flex flex-col gap-2 border-t border-border pt-3">
        <h4 className="font-semibold">{t("promotions.targets.heading")}</h4>
        {targets.length === 0 ? <p>{t("promotions.targets.empty")}</p> : null}
        <ul className="flex flex-wrap gap-2">
          {targets.map((target) => (
            <li
              key={`${target.type}-${target.id}`}
              className="flex items-center gap-2 rounded-ui border border-purple bg-purple-soft px-2 text-purple"
            >
              {targetLabel(target)}
              <IconButton
                icon={Trash2}
                colour="red"
                tip={t("promotions.targets.remove")}
                onClick={() =>
                  setTargets((old) =>
                    old.filter((each) => !(each.type === target.type && each.id === target.id)),
                  )
                }
              />
            </li>
          ))}
        </ul>
        <div className="flex flex-wrap items-end gap-3">
          <ProductPicker currency={choices.currency} chosen={product} onChoose={setProduct} />
          <ActionButton
            colour="blue"
            icon={Plus}
            label={t("promotions.targets.addProduct.label")}
            tip={t("promotions.targets.addProduct.tip")}
            disabledReason={product ? undefined : t("items.add.pickFirst")}
            onClick={() => {
              if (product) {
                addTarget({ type: "product", id: product.product_id }, product.name);
                setProduct(null);
              }
            }}
          />
        </div>
        <div className="flex flex-wrap items-end gap-3">
          <SelectField
            label={t("promotions.targets.category_pick.label")}
            tip={t("promotions.targets.category_pick.tip")}
            value={categoryId}
            options={[
              { value: "", label: t("promotions.targets.choose") },
              ...choices.categories.map((each) => ({ value: each.id, label: each.name })),
            ]}
            onChange={setCategoryId}
          />
          <ActionButton
            colour="blue"
            icon={Plus}
            label={t("promotions.targets.addCategory.label")}
            tip={t("promotions.targets.addCategory.tip")}
            onClick={() => {
              if (categoryId !== "") {
                addTarget({ type: "category", id: categoryId }, "");
              }
            }}
          />
          <SelectField
            label={t("promotions.targets.brand_pick.label")}
            tip={t("promotions.targets.brand_pick.tip")}
            value={brandId}
            options={[
              { value: "", label: t("promotions.targets.choose") },
              ...choices.brands.map((each) => ({ value: each.id, label: each.name })),
            ]}
            onChange={setBrandId}
          />
          <ActionButton
            colour="blue"
            icon={Plus}
            label={t("promotions.targets.addBrand.label")}
            tip={t("promotions.targets.addBrand.tip")}
            onClick={() => {
              if (brandId !== "") {
                addTarget({ type: "brand", id: brandId }, "");
              }
            }}
          />
        </div>
      </section>
      {(problem ?? (save.problem && !save.errorFor("name"))) ? (
        <Alert kind="problem">{problem ?? save.problem}</Alert>
      ) : null}
      {save.done ? <Alert kind="done">{t("common.saved")}</Alert> : null}
      <FormButtons editing={promotion !== null} onCancel={onCancel} onSave={submit} />
    </section>
  );
}
