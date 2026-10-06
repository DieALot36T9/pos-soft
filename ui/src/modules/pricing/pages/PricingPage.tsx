import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { ProductChoices } from "../../../generated/ProductChoices";
import { errorMessage } from "../../../shared/bridge";
import { Alert } from "../../../shared/ui/Alert";
import { Tip } from "../../../shared/ui/Tip";
import { loadChoices } from "../api";
import { PriceListsPanel } from "../components/PriceListsPanel";
import { PromotionsPanel } from "../components/PromotionsPanel";
import { TiersPanel } from "../components/TiersPanel";

type Area = "lists" | "tiers" | "promotions";
const AREAS: Area[] = ["lists", "tiers", "promotions"];

// Price lists, tiers and promotions (Req - Pricing and Promotions). Needs PricingManage.
export function PricingPage() {
  const { t } = useTranslation("pricing");
  const { t: tErrors } = useTranslation("errors");
  const [area, setArea] = useState<Area>("lists");
  const [choices, setChoices] = useState<ProductChoices | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    loadChoices()
      .then(setChoices)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, []);

  return (
    <section aria-labelledby="pricing-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="pricing-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>
      <div role="tablist" aria-label={t("tabs.label")} className="flex gap-2">
        {AREAS.map((each) => (
          <Tip key={each} tip={t(`tabs.${each}.tip`)}>
            <button
              type="button"
              role="tab"
              aria-selected={each === area}
              onClick={() => setArea(each)}
              className={
                "min-h-10 rounded-ui border px-4 " +
                (each === area
                  ? "border-blue bg-blue-soft font-semibold text-blue"
                  : "border-border bg-panel")
              }
            >
              {t(`tabs.${each}.label`)}
            </button>
          </Tip>
        ))}
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {choices && area === "lists" ? <PriceListsPanel currency={choices.currency} /> : null}
      {area === "tiers" ? <TiersPanel /> : null}
      {choices && area === "promotions" ? <PromotionsPanel choices={choices} /> : null}
    </section>
  );
}
