import { Eye, ListPlus, Plus, Printer, Search, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Category } from "../../../generated/Category";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { LabelItem } from "../../../generated/LabelItem";
import type { LabelKind } from "../../../generated/LabelKind";
import type { LabelPick } from "../../../generated/LabelPick";
import type { LabelPicture } from "../../../generated/LabelPicture";
import type { LabelSettings } from "../../../generated/LabelSettings";
import type { SearchHit } from "../../../generated/SearchHit";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { printWindow } from "../../../shared/print/api";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import {
  listCategories,
  pickLabels,
  previewLabels,
  printLabels,
  productChoices,
  searchProducts,
} from "../api";

// Common label sizes in millimetres: width × height.
const SIZES = ["58x40", "50x30", "38x25", "80x50", "80x40", "60x30"] as const;
type Size = (typeof SIZES)[number];
type Output = "printer" | "a4";

const DAY = /^\d{4}-\d{2}-\d{2}$/;

// Labels and shelf tags (CAT-12, CAT-13): choose products, a template and how many of
// each; preview the real pictures; print on the label printer or on A4 sheets through
// the print dialog (ADR-013). Prices and promotions come from Rust.
export function LabelsPage() {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [currency, setCurrency] = useState<CurrencyFormat | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [typed, setTyped] = useState("");
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [categoryId, setCategoryId] = useState("");
  const [since, setSince] = useState("");
  const [items, setItems] = useState<LabelItem[]>([]);
  const [copies, setCopies] = useState<Record<string, string>>({});
  const [kind, setKind] = useState<LabelKind>("label");
  const [size, setSize] = useState<Size>("58x40");
  const [output, setOutput] = useState<Output>("printer");
  const [show, setShow] = useState({ name: true, price: true, unitPrice: true, barcode: true });
  const [packedOn, setPackedOn] = useState("");
  const [expiresOn, setExpiresOn] = useState("");
  const [pictures, setPictures] = useState<LabelPicture[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(() => {
    productChoices()
      .then((choices) => setCurrency(choices.currency))
      .catch(fail);
    listCategories(false).then(setCategories).catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, []);

  const clear = () => {
    setProblem(null);
    setNotice(null);
  };

  // Adds what Rust picked; a product already on the list gets the extra labels.
  const add = (pick: LabelPick) => {
    clear();
    pickLabels(pick)
      .then((picked) => {
        setCurrency(picked.currency);
        if (picked.items.length === 0) {
          setNotice(t("labels.pick.noneFound"));
          return;
        }
        setPictures(null);
        setItems((current) => {
          const next = [...current];
          for (const item of picked.items) {
            if (!next.some((each) => each.product_id === item.product_id)) {
              next.push(item);
            }
          }
          return next;
        });
        setCopies((current) => {
          const next = { ...current };
          for (const item of picked.items) {
            next[item.product_id] = String(Number(current[item.product_id] ?? 0) + item.copies);
          }
          return next;
        });
      })
      .catch(fail);
  };

  const find = () => {
    clear();
    searchProducts(typed, 20).then(setHits).catch(fail);
  };

  const remove = (id: string) => {
    setItems((current) => current.filter((item) => item.product_id !== id));
    setPictures(null);
  };

  // The request for Rust, or null with the problem shown.
  const request = () => {
    if (items.length === 0) {
      setProblem(t("labels.needItems"));
      return null;
    }
    const counted = items.map((item) => ({
      product_id: item.product_id,
      copies: Number(copies[item.product_id] ?? "1"),
    }));
    if (counted.some(({ copies: n }) => !Number.isInteger(n) || n < 1 || n > 500)) {
      setProblem(t("labels.items.copiesWrong"));
      return null;
    }
    const day = (text: string) => (text.trim() === "" ? null : text.trim());
    const dates = [packedOn, expiresOn].filter((text) => text.trim() !== "");
    if (dates.some((text) => !DAY.test(text.trim()))) {
      setProblem(t("labels.template.dateWrong"));
      return null;
    }
    const [width, height] = size.split("x").map(Number);
    const settings: LabelSettings = {
      kind,
      width_mm: width ?? 58,
      height_mm: height ?? 40,
      gap_mm: 2,
      show_name: show.name,
      show_price: show.price,
      show_unit_price: show.unitPrice,
      show_barcode: show.barcode,
      packed_on: kind === "label" ? day(packedOn) : null,
      expires_on: kind === "label" ? day(expiresOn) : null,
    };
    return { settings, items: counted };
  };

  const preview = () => {
    clear();
    const wanted = request();
    if (wanted !== null) {
      previewLabels(wanted).then(setPictures).catch(fail);
    }
  };

  const print = () => {
    clear();
    const wanted = request();
    if (wanted === null) {
      return;
    }
    if (output === "a4") {
      if (pictures === null) {
        setProblem(t("labels.needPreview"));
        return;
      }
      printWindow().catch(fail);
      return;
    }
    printLabels(wanted)
      .then((printed) =>
        setNotice(
          t(printed.to_preview ? "labels.printedToPreview" : "labels.printed", {
            count: printed.labels,
          }),
        ),
      )
      .catch(fail);
  };

  const [widthMm] = size.split("x").map(Number);
  const sheet =
    pictures?.flatMap((picture) =>
      Array.from({ length: picture.copies }, (_, index) => ({
        key: `${picture.product_id}-${index}`,
        src: picture.picture,
      })),
    ) ?? [];

  return (
    <section aria-labelledby="labels-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="labels-heading" className="text-[28px] font-semibold">
          {t("labels.title")}
        </h2>
        <p className="text-text-muted">{t("labels.intro")}</p>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <div className="w-72">
          <Field
            label={t("labels.pick.search.label")}
            tip={t("labels.pick.search.tip")}
            value={typed}
            onChange={setTyped}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                find();
              }
            }}
          />
        </div>
        <ActionButton
          colour="blue"
          icon={Search}
          label={t("labels.pick.find.label")}
          tip={t("labels.pick.find.tip")}
          onClick={find}
        />
        <div className="w-64">
          <SelectField
            label={t("labels.pick.category.label")}
            tip={t("labels.pick.category.tip")}
            value={categoryId}
            options={[
              { value: "", label: t("labels.pick.chooseCategory") },
              ...categories.map((category) => ({ value: category.id, label: category.name })),
            ]}
            onChange={setCategoryId}
          />
        </div>
        <ActionButton
          colour="blue"
          icon={ListPlus}
          label={t("labels.pick.addCategory.label")}
          tip={t("labels.pick.addCategory.tip")}
          disabledReason={categoryId === "" ? t("labels.pick.chooseCategory") : undefined}
          onClick={() => add({ kind: "category", id: categoryId })}
        />
        <div className="w-48">
          <Field
            label={t("labels.pick.since.label")}
            tip={t("labels.pick.since.tip")}
            value={since}
            onChange={setSince}
          />
        </div>
        <ActionButton
          colour="blue"
          icon={ListPlus}
          label={t("labels.pick.addSince.label")}
          tip={t("labels.pick.addSince.tip")}
          onClick={() =>
            DAY.test(since.trim())
              ? add({ kind: "price_changed_since", date: since.trim() })
              : setProblem(t("labels.template.dateWrong"))
          }
        />
      </div>

      {hits !== null ? (
        <ul aria-label={t("labels.pick.heading")} className="flex flex-col gap-1">
          {hits.length === 0 ? (
            <li className="text-text-muted">{t("labels.pick.noHits")}</li>
          ) : null}
          {hits.map((hit) => (
            <li key={hit.product_id} className="flex items-center gap-2">
              <IconButton
                icon={Plus}
                colour="green"
                tip={t("labels.pick.add.tip", { name: hit.name })}
                onClick={() => add({ kind: "products", ids: [hit.product_id] })}
              />
              <span>{t("labels.pick.hit", { name: hit.name, sku: hit.sku })}</span>
            </li>
          ))}
        </ul>
      ) : null}

      <table className="w-full text-left">
        <thead className="bg-panel">
          <tr>
            <th className="p-2">{t("labels.items.name")}</th>
            <th className="p-2">{t("labels.items.sku")}</th>
            <th className="p-2 text-right">{t("labels.items.price")}</th>
            <th className="p-2">{t("labels.items.copies.label")}</th>
            <th className="p-2" />
          </tr>
        </thead>
        <tbody>
          {items.length === 0 ? (
            <tr>
              <td colSpan={5} className="p-2 text-text-muted">
                {t("labels.items.empty")}
              </td>
            </tr>
          ) : null}
          {items.map((item) => (
            <tr key={item.product_id} className="border-t border-border">
              <td className="p-2">{item.name}</td>
              <td className="p-2">{item.sku}</td>
              <td className="p-2 text-right tabular-nums">
                {currency ? formatMoney(item.price, currency) : ""}
              </td>
              <td className="p-2">
                <Field
                  label={t("labels.items.copies.label")}
                  tip={t("labels.items.copies.tip")}
                  width="short"
                  value={copies[item.product_id] ?? "1"}
                  onChange={(value) => {
                    setCopies({ ...copies, [item.product_id]: value });
                    setPictures(null);
                  }}
                />
              </td>
              <td className="p-2">
                <IconButton
                  icon={Trash2}
                  colour="red"
                  tip={t("labels.items.remove.tip", { name: item.name })}
                  onClick={() => remove(item.product_id)}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3 className="text-xl font-semibold">{t("labels.template.heading")}</h3>
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-48">
          <SelectField<LabelKind>
            label={t("labels.template.kind.label")}
            tip={t("labels.template.kind.tip")}
            value={kind}
            options={[
              { value: "label", label: t("labels.template.kind.label_option") },
              { value: "shelf_tag", label: t("labels.template.kind.shelf_tag_option") },
            ]}
            onChange={(next) => {
              setKind(next);
              setSize(next === "shelf_tag" ? "80x40" : "58x40");
              setPictures(null);
            }}
          />
        </div>
        <div className="w-40">
          <SelectField<Size>
            label={t("labels.template.size.label")}
            tip={t("labels.template.size.tip")}
            value={size}
            options={SIZES.map((each) => ({ value: each, label: each.replace("x", " × ") }))}
            onChange={(next) => {
              setSize(next);
              setPictures(null);
            }}
          />
        </div>
        <div className="w-48">
          <SelectField<Output>
            label={t("labels.template.output.label")}
            tip={t("labels.template.output.tip")}
            value={output}
            options={[
              { value: "printer", label: t("labels.template.output.printer") },
              { value: "a4", label: t("labels.template.output.a4") },
            ]}
            onChange={setOutput}
          />
        </div>
        {kind === "label" ? (
          <>
            <div className="w-40">
              <Field
                label={t("labels.template.packedOn.label")}
                tip={t("labels.template.packedOn.tip")}
                value={packedOn}
                onChange={setPackedOn}
              />
            </div>
            <div className="w-40">
              <Field
                label={t("labels.template.expiresOn.label")}
                tip={t("labels.template.expiresOn.tip")}
                value={expiresOn}
                onChange={setExpiresOn}
              />
            </div>
          </>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-4">
        {kind === "label" ? (
          <>
            <CheckBox
              label={t("labels.template.showName.label")}
              tip={t("labels.template.showName.tip")}
              checked={show.name}
              onChange={(name) => setShow({ ...show, name })}
            />
            <CheckBox
              label={t("labels.template.showPrice.label")}
              tip={t("labels.template.showPrice.tip")}
              checked={show.price}
              onChange={(price) => setShow({ ...show, price })}
            />
          </>
        ) : null}
        <CheckBox
          label={t("labels.template.showUnitPrice.label")}
          tip={t("labels.template.showUnitPrice.tip")}
          checked={show.unitPrice}
          onChange={(unitPrice) => setShow({ ...show, unitPrice })}
        />
        <CheckBox
          label={t("labels.template.showBarcode.label")}
          tip={t("labels.template.showBarcode.tip")}
          checked={show.barcode}
          onChange={(barcode) => setShow({ ...show, barcode })}
        />
      </div>

      <div className="flex gap-2">
        <ActionButton
          colour="blue"
          icon={Eye}
          label={t("labels.preview.label")}
          tip={t("labels.preview.tip")}
          disabledReason={items.length === 0 ? t("labels.needItems") : undefined}
          onClick={preview}
        />
        <ActionButton
          colour="green"
          icon={Printer}
          label={t("labels.print.label")}
          tip={t("labels.print.tip")}
          disabledReason={items.length === 0 ? t("labels.needItems") : undefined}
          onClick={print}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}

      {pictures !== null && output === "printer" ? (
        <ul aria-label={t("labels.preview.label")} className="flex flex-wrap gap-4">
          {pictures.map((picture) => (
            <li key={picture.product_id} className="flex flex-col items-center gap-1">
              <img
                src={picture.picture}
                alt={items.find((item) => item.product_id === picture.product_id)?.name ?? ""}
                className="border border-border bg-white"
              />
              <span>{t("labels.copies", { count: picture.copies })}</span>
            </li>
          ))}
        </ul>
      ) : null}
      {pictures !== null && output === "a4" ? (
        <section aria-label={t("labels.sheetHeading")} className="flex flex-col gap-2">
          <h3 className="text-xl font-semibold">{t("labels.sheetHeading")}</h3>
          <div data-print-area className="flex w-[186mm] flex-wrap content-start bg-white">
            {sheet.map((label) => (
              <img
                key={label.key}
                src={label.src}
                alt=""
                style={{ width: `${widthMm ?? 58}mm` }}
                className="border border-dashed border-border"
              />
            ))}
          </div>
        </section>
      ) : null}
    </section>
  );
}
