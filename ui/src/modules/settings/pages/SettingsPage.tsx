import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { SettingsSnapshot } from "../../../generated/SettingsSnapshot";
import { errorMessage } from "../../../shared/bridge";
import { Alert } from "../../../shared/ui/Alert";
import { Tip } from "../../../shared/ui/Tip";
import { getSettings } from "../api";
import { BackupSection } from "../components/BackupSection";
import { DevicesSection } from "../components/DevicesSection";
import { LogsSection } from "../components/LogsSection";
import { LoyaltyCreditSection } from "../components/LoyaltyCreditSection";
import { MoneySection } from "../components/MoneySection";
import { PaymentMethodsSection } from "../components/PaymentMethodsSection";
import { ReceiptSection } from "../components/ReceiptSection";
import { ShopSection } from "../components/ShopSection";
import { TaxSection } from "../components/TaxSection";
import { TillSection } from "../components/TillSection";

const sections = [
  "shop",
  "money",
  "tax",
  "till",
  "receipt",
  "devices",
  "payments",
  "loyalty",
  "backup",
  "logs",
] as const;
type Section = (typeof sections)[number];

export function SettingsPage() {
  const { t } = useTranslation("settings");
  const { t: tErrors } = useTranslation("errors");
  const [saved, setSaved] = useState<SettingsSnapshot | null>(null);
  const [section, setSection] = useState<Section>("shop");
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then(setSaved)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const props = saved ? { saved, onSaved: setSaved } : null;

  return (
    <section aria-labelledby="settings-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="settings-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex flex-col md:flex-row gap-5 items-start">
        <nav
          aria-label={t("page.sections")}
          className="w-full md:w-56 shrink-0 rounded-ui border border-border bg-panel p-2"
        >
          <ul className="flex flex-col gap-1">
            {sections.map((each) => (
              <li key={each}>
                <Tip tip={t("sectionTip", { name: t(`sections.${each}`) })}>
                  <button
                    type="button"
                    aria-current={each === section ? "page" : undefined}
                    onClick={() => setSection(each)}
                    className={
                      "min-h-10 w-full rounded-ui border px-3 text-left text-sm cursor-pointer " +
                      (each === section
                        ? "border-blue bg-blue-soft font-bold text-blue"
                        : "border-transparent bg-panel text-text hover:bg-grey-soft")
                    }
                  >
                    {t(`sections.${each}`)}
                  </button>
                </Tip>
              </li>
            ))}
          </ul>
        </nav>
        {props ? (
          <div className="flex flex-1 w-full" key={section}>
            {section === "shop" ? <ShopSection {...props} /> : null}
            {section === "money" ? <MoneySection {...props} /> : null}
            {section === "tax" ? <TaxSection {...props} /> : null}
            {section === "till" ? <TillSection {...props} /> : null}
            {section === "receipt" ? <ReceiptSection {...props} /> : null}
            {section === "devices" ? <DevicesSection {...props} /> : null}
            {section === "payments" ? <PaymentMethodsSection /> : null}
            {section === "loyalty" && props ? <LoyaltyCreditSection {...props} /> : null}
            {section === "backup" ? <BackupSection {...props} /> : null}
            {section === "logs" && saved ? <LogsSection saved={saved} /> : null}
          </div>
        ) : null}
      </div>
    </section>
  );
}
