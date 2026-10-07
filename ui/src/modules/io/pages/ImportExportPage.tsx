import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Choice } from "../../../generated/Choice";
import type { Permission } from "../../../generated/Permission";
import { Tip } from "../../../shared/ui/Tip";
import { priceLists as loadPriceLists } from "../api";
import { ExportPanel } from "../components/ExportPanel";
import { ImportWizard } from "../components/ImportWizard";

type Tab = "import" | "export";

// Import / Export (Req - Import and Export). The tabs the role allows: import needs
// "data import", export needs "data export". Rust checks both anyway.
export function ImportExportPage({ permissions }: { permissions: Permission[] }) {
  const { t } = useTranslation("io");
  const tabs: Tab[] = [];
  if (permissions.includes("data_import")) {
    tabs.push("import");
  }
  if (permissions.includes("data_export")) {
    tabs.push("export");
  }
  const [tab, setTab] = useState<Tab>(tabs[0] ?? "import");
  const [lists, setLists] = useState<Choice[]>([]);

  useEffect(() => {
    loadPriceLists()
      .then(setLists)
      .catch(() => setLists([]));
  }, []);

  return (
    <section aria-labelledby="io-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="io-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>
      <div role="tablist" className="flex gap-2 border-b border-border">
        {tabs.map((each) => (
          <Tip key={each} tip={t(`page.${each}Tab.tip`)}>
            <button
              type="button"
              role="tab"
              aria-selected={each === tab}
              onClick={() => setTab(each)}
              className={
                "min-h-10 border-b-2 px-4 " +
                (each === tab ? "border-blue font-semibold text-blue" : "border-transparent")
              }
            >
              {t(`page.${each}Tab.label`)}
            </button>
          </Tip>
        ))}
      </div>
      {tab === "import" ? <ImportWizard priceLists={lists} /> : <ExportPanel priceLists={lists} />}
    </section>
  );
}
