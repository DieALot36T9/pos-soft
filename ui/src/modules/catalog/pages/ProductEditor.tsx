import { Archive, ArchiveRestore, ArrowLeft, Save, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { ProductChoices } from "../../../generated/ProductChoices";
import type { ProductDetails } from "../../../generated/ProductDetails";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Tip } from "../../../shared/ui/Tip";
import {
  archiveProduct,
  deleteProduct,
  generateBarcode,
  getProduct,
  saveProduct,
  unarchiveProduct,
} from "../api";
import { BarcodesTab } from "../components/BarcodesTab";
import { type Draft, draftOf, emptyDraft, inputOf } from "../components/draft";
import { GeneralTab } from "../components/GeneralTab";
import { ImageTab } from "../components/ImageTab";
import { KitTab } from "../components/KitTab";
import { PricesTab } from "../components/PricesTab";
import { StockTab } from "../components/StockTab";
import { VariantsTab } from "../components/VariantsTab";

type Tab = "general" | "prices" | "barcodes" | "stock" | "variants" | "kit" | "image";

type ProductEditorProps = {
  productId: string | null;
  choices: ProductChoices;
  onBack: () => void;
  onOpen: (id: string) => void;
};

export function ProductEditor({ productId, choices, onBack, onOpen }: ProductEditorProps) {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [id, setId] = useState(productId);
  const [details, setDetails] = useState<ProductDetails | null>(null);
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [tab, setTab] = useState<Tab>("general");
  const [message, setMessage] = useState<string | null>(null);
  const [localProblem, setLocalProblem] = useState<{ field: string; text: string } | null>(null);
  const save = useSave();
  const action = useSave();

  const load = (which: string) => {
    getProduct(which)
      .then((found) => {
        setDetails(found);
        setDraft(draftOf(found, choices.currency, choices.units));
      })
      .catch((error: unknown) =>
        setLocalProblem({ field: "", text: errorMessage(error, tErrors) }),
      );
  };
  useEffect(() => {
    if (id) {
      load(id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load when the product changes.
  }, [id]);

  const change = (part: Partial<Draft>) => setDraft((current) => ({ ...current, ...part }));
  const errorFor = (field: string) =>
    localProblem?.field === field ? localProblem.text : save.errorFor(field);

  const submit = () => {
    const read = inputOf(draft, id, choices.currency, choices.units);
    if (!read.ok) {
      setLocalProblem({ field: read.field, text: tErrors(read.code as "unknown", read.params) });
      return;
    }
    setLocalProblem(null);
    setMessage(null);
    save.run(
      () => saveProduct(read.input),
      (savedId) => {
        setMessage(t("editor.saved"));
        if (savedId === id) {
          load(savedId);
        } else {
          setId(savedId);
        }
      },
    );
  };

  const run = (work: () => Promise<unknown>, done: string, after?: () => void) => {
    setMessage(null);
    action.run(work, () => {
      setMessage(done);
      after?.();
    });
  };

  useHotkeys(shortcuts.save.keys, submit, { enableOnFormTags: true, preventDefault: true });
  useHotkeys(shortcuts.back.keys, onBack, { enableOnFormTags: true });

  const isNew = id === null;
  const product = details?.product ?? null;
  const tabs: Tab[] = ["general", "prices", "barcodes", "stock"];
  if (draft.kind === "variant_parent") {
    tabs.push("variants");
  }
  if (draft.kind === "kit") {
    tabs.push("kit");
  }
  tabs.push("image");

  const fieldProblem = save.problem ?? localProblem?.text ?? null;
  const skuLabel = product ? `SKU: ${product.sku}` : "";

  return (
    <section aria-labelledby="product-heading" className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-3">
        <div>
          <h2 id="product-heading" className="text-[28px] font-semibold">
            {product ? t("editor.editHeading", { name: product.name }) : t("editor.addHeading")}
          </h2>
          {product ? <p className="font-mono text-xs text-text-muted mt-0.5">{skuLabel}</p> : null}
        </div>
      </div>

      {product?.archived_at ? <Alert kind="warning">{t("editor.isArchived")}</Alert> : null}

      <div
        role="tablist"
        aria-label={t("editor.tabsLabel")}
        className="flex flex-wrap gap-1.5 rounded-ui border border-border bg-panel p-1.5"
      >
        {tabs.map((each) => (
          <Tip key={each} tip={t(`editor.tabs.${each}.tip`)}>
            <button
              type="button"
              role="tab"
              aria-selected={each === tab}
              onClick={() => setTab(each)}
              className={
                "min-h-10 rounded-ui border px-4 text-sm cursor-pointer " +
                (each === tab
                  ? "border-blue bg-blue-soft font-bold text-blue"
                  : "border-transparent bg-panel text-text hover:bg-grey-soft")
              }
            >
              {t(`editor.tabs.${each}.label`)}
            </button>
          </Tip>
        ))}
      </div>

      <div role="tabpanel" className="rounded-ui border border-border bg-panel p-5">
        {tab === "general" ? (
          <GeneralTab
            draft={draft}
            change={change}
            errorFor={errorFor}
            choices={choices}
            isNew={isNew}
            variantLabel={product?.variant_label ?? null}
          />
        ) : null}
        {tab === "prices" ? (
          <PricesTab
            draft={draft}
            change={change}
            errorFor={errorFor}
            choices={choices}
            history={details?.price_history ?? []}
          />
        ) : null}
        {tab === "barcodes" ? (
          <BarcodesTab
            draft={draft}
            change={change}
            errorFor={errorFor}
            isParent={draft.kind === "variant_parent"}
            onGenerate={
              id
                ? () =>
                    action.run(
                      () => generateBarcode(id),
                      (code) => {
                        setMessage(t("barcodes.generated", { code }));
                        change({ barcodes: [...draft.barcodes, code] });
                      },
                    )
                : null
            }
          />
        ) : null}
        {tab === "stock" ? (
          <StockTab
            draft={draft}
            change={change}
            errorFor={errorFor}
            choices={choices}
            onHand={details?.on_hand ?? null}
          />
        ) : null}
        {tab === "variants" ? (
          id ? (
            <VariantsTab
              parentId={id}
              variants={details?.variants ?? []}
              currency={choices.currency}
              onMade={() => load(id)}
              onOpen={onOpen}
            />
          ) : (
            <p>{t("editor.saveFirst")}</p>
          )
        ) : null}
        {tab === "kit" ? (
          id && details ? (
            <KitTab
              key={details.product.updated_at}
              kitId={id}
              parts={details.kit_parts}
              onSaved={() => load(id)}
            />
          ) : (
            <p>{t("editor.saveFirst")}</p>
          )
        ) : null}
        {tab === "image" ? (
          id && details ? (
            <ImageTab
              productId={id}
              name={details.product.name}
              image={details.image}
              onSaved={() => load(id)}
            />
          ) : (
            <p>{t("editor.saveFirst")}</p>
          )
        ) : null}
      </div>

      {fieldProblem ? <Alert kind="problem">{fieldProblem}</Alert> : null}
      {action.problem ? <Alert kind="problem">{action.problem}</Alert> : null}
      {message ? <Alert kind="done">{message}</Alert> : null}

      <div className="flex flex-wrap justify-between gap-3 rounded-ui border border-border bg-panel p-4">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("editor.back.label")}
          tip={t("editor.back.tip")}
          shortcut={shortcuts.back.label}
          onClick={onBack}
        />
        <div className="flex flex-wrap gap-3">
          {id && product && product.archived_at === null ? (
            <ActionButton
              colour="red"
              icon={Archive}
              label={t("editor.archive.label")}
              tip={t("editor.archive.tip")}
              onClick={() =>
                run(
                  () => archiveProduct(id),
                  t("editor.archived"),
                  () => load(id),
                )
              }
            />
          ) : null}
          {id && product?.archived_at ? (
            <ActionButton
              colour="green"
              icon={ArchiveRestore}
              label={t("editor.unarchive.label")}
              tip={t("editor.unarchive.tip")}
              onClick={() =>
                run(
                  () => unarchiveProduct(id),
                  t("editor.unarchived"),
                  () => load(id),
                )
              }
            />
          ) : null}
          {id ? (
            <ActionButton
              colour="red"
              icon={Trash2}
              label={t("editor.delete.label")}
              tip={t("editor.delete.tip")}
              onClick={() => run(() => deleteProduct(id), t("editor.deleted"), onBack)}
            />
          ) : null}
          <ActionButton
            colour="green"
            icon={Save}
            label={t("editor.save.label")}
            tip={t("editor.save.tip")}
            shortcut={shortcuts.save.label}
            onClick={submit}
          />
        </div>
      </div>
    </section>
  );
}
