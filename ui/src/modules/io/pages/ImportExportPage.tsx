import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Choice } from "../../../generated/Choice";
import type { Permission } from "../../../generated/Permission";
import { Tip } from "../../../shared/ui/Tip";
import { priceLists as loadPriceLists } from "../api";
import { ExportPanel } from "../components/ExportPanel";
import { ImportWizard } from "../components/ImportWizard";

type Tab = "import" | "export";

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
      <div
        role="tablist"
        className="flex flex-wrap gap-1.5 rounded-ui border border-border bg-panel p-1.5"
      >
        {tabs.map((each) => (
          <Tip key={each} tip={t(`page.${each}Tab.tip`)}>
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
              {t(`page.${each}Tab.label`)}
            </button>
          </Tip>
        ))}
      </div>
      {tab === "import" ? <ImportWizard priceLists={lists} /> : <ExportPanel priceLists={lists} />}
    </section>
  );
}
