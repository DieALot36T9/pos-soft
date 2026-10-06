import { Archive, CheckCircle, ChevronLeft, ChevronRight, FolderOpen, Plus } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { ProductChoices } from "../../../generated/ProductChoices";
import type { ProductList } from "../../../generated/ProductList";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { SelectField } from "../../../shared/ui/SelectField";
import { listProducts, productChoices } from "../api";
import { categoryPaths } from "../components/categoryPaths";
import { ProductEditor } from "./ProductEditor";

// Products per page of the list.
const PAGE_SIZE = 50;

// The product list with its filters, and the product page when one is open (CAT-01, CAT-08).
export function ProductsPage() {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [choices, setChoices] = useState<ProductChoices | null>(null);
  const [list, setList] = useState<ProductList>({ rows: [], total: 0 });
  const [text, setText] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [brandId, setBrandId] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [offset, setOffset] = useState(0);
  // "new" for a new product, a product id, or null for the list.
  const [open, setOpen] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  // Each answer is shown only if it is for the newest question, so fast typing never shows old rows.
  const asked = useRef(0);

  const loadChoices = () => {
    productChoices()
      .then(setChoices)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the page opens.
  useEffect(loadChoices, []);

  const load = () => {
    asked.current += 1;
    const question = asked.current;
    listProducts(
      {
        text: text.trim() === "" ? null : text,
        category_id: categoryId === "" ? null : categoryId,
        brand_id: brandId === "" ? null : brandId,
        kind: null,
        parent_id: null,
        include_archived: showArchived,
      },
      { offset, limit: PAGE_SIZE },
    )
      .then((found) => {
        if (question === asked.current) {
          setList(found);
        }
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- reload when a filter changes.
  useEffect(load, [text, categoryId, brandId, showArchived, offset, open]);

  const filterBy = (apply: () => void) => {
    apply();
    setOffset(0);
  };

  if (!choices) {
    return problem ? <Alert kind="problem">{problem}</Alert> : null;
  }
  if (open !== null) {
    return (
      <ProductEditor
        key={open}
        productId={open === "new" ? null : open}
        choices={choices}
        onBack={() => {
          setOpen(null);
          loadChoices();
        }}
        onOpen={setOpen}
      />
    );
  }

  return (
    <section aria-labelledby="products-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 id="products-heading" className="text-[28px] font-semibold">
            {t("products.title")}
          </h2>
          <p className="text-text-muted">{t("products.intro")}</p>
        </div>
        <ActionButton
          colour="green"
          icon={Plus}
          label={t("products.add.label")}
          tip={t("products.add.tip")}
          onClick={() => setOpen("new")}
        />
      </div>
      <div className="grid grid-cols-3 gap-3">
        <Field
          label={t("products.search.label")}
          tip={t("products.search.tip")}
          value={text}
          onChange={(typed) => filterBy(() => setText(typed))}
        />
        <SelectField
          label={t("products.category.label")}
          tip={t("products.category.tip")}
          value={categoryId}
          options={[
            { value: "", label: t("products.anyCategory") },
            ...categoryPaths(choices.categories).map((each) => ({
              value: each.id,
              label: each.path,
            })),
          ]}
          onChange={(picked) => filterBy(() => setCategoryId(picked))}
        />
        <SelectField
          label={t("products.brand.label")}
          tip={t("products.brand.tip")}
          value={brandId}
          options={[
            { value: "", label: t("products.anyBrand") },
            ...choices.brands.map((brand) => ({ value: brand.id, label: brand.name })),
          ]}
          onChange={(picked) => filterBy(() => setBrandId(picked))}
        />
      </div>
      <CheckBox
        label={t("products.showArchived.label")}
        tip={t("products.showArchived.tip")}
        checked={showArchived}
        onChange={(checked) => filterBy(() => setShowArchived(checked))}
      />
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {list.rows.length === 0 ? (
        <p className="rounded-ui border border-border bg-panel p-3">{t("products.table.empty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("products.table.name")}</th>
              <th className="p-2">{t("products.table.sku")}</th>
              <th className="p-2">{t("products.table.category")}</th>
              <th className="p-2">{t("products.table.brand")}</th>
              <th className="p-2 text-right">{t("products.table.price")}</th>
              <th className="p-2 text-right">{t("products.table.cost")}</th>
              <th className="p-2">{t("products.table.status")}</th>
              <th className="p-2">{t("products.table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {list.rows.map((row) => (
              <tr key={row.id} className="border-b border-border">
                <td className="p-2">{row.name}</td>
                <td className="p-2">{row.sku}</td>
                <td className="p-2">{row.category_name}</td>
                <td className="p-2">{row.brand_name ?? ""}</td>
                <td className="p-2 text-right tabular-nums">
                  {formatMoney(row.sale_price, choices.currency)}
                </td>
                <td className="p-2 text-right tabular-nums">
                  {formatMoney(row.cost_price, choices.currency)}
                </td>
                <td className="p-2">
                  {row.archived_at === null ? (
                    <Badge
                      colour="green"
                      icon={CheckCircle}
                      word={t("products.table.active")}
                      tip={t("products.table.activeTip")}
                    />
                  ) : (
                    <Badge
                      colour="red"
                      icon={Archive}
                      word={t("products.table.archived")}
                      tip={t("products.table.archivedTip")}
                    />
                  )}
                </td>
                <td className="p-2">
                  <IconButton
                    icon={FolderOpen}
                    colour="blue"
                    tip={`${t("products.open.tip")} ${row.name}`}
                    onClick={() => setOpen(row.id)}
                  />
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
          label={t("products.previous.label")}
          tip={t("products.previous.tip")}
          disabledReason={offset === 0 ? t("products.previous.none") : undefined}
          onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
        />
        <span>
          {t("products.count", {
            shown: list.rows.length,
            total: list.total,
          })}
        </span>
        <ActionButton
          colour="grey"
          icon={ChevronRight}
          label={t("products.next.label")}
          tip={t("products.next.tip")}
          disabledReason={offset + PAGE_SIZE >= list.total ? t("products.next.none") : undefined}
          onClick={() => setOffset(offset + PAGE_SIZE)}
        />
      </div>
    </section>
  );
}
