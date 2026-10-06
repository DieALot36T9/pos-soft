import { Info } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { AppInfo } from "../generated/AppInfo";
import { StockAlertsCard } from "../modules/inventory/components/StockAlertsCard";
import { errorMessage } from "../shared/bridge";
import { ActionButton } from "../shared/ui/ActionButton";
import { Alert } from "../shared/ui/Alert";
import { getAppInfo } from "./api";

type HomePageProps = {
  // Low stock and near expiry at login, for users with StockView (INV-03).
  showStockAlerts: boolean;
  onOpenStock: () => void;
};

// The start page of the back office: the stock alerts and the app's version. The sales
// cards come with checkout (C-M6).
export function HomePage({ showStockAlerts, onOpenStock }: HomePageProps) {
  const { t } = useTranslation("app");
  const { t: tErrors } = useTranslation("errors");
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const check = () => {
    getAppInfo()
      .then((answer) => {
        setInfo(answer);
        setProblem(null);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <div className="flex flex-col gap-4">
      {showStockAlerts ? <StockAlertsCard onOpenStock={onOpenStock} /> : null}
      <section className="w-full max-w-xl rounded-ui border border-border bg-panel p-6">
        <h2 className="mb-2 text-[28px] font-semibold">{t("title")}</h2>
        <p className="mb-6 text-text-muted">{t("intro")}</p>
        <ActionButton
          colour="blue"
          icon={Info}
          label={t("checkButton.label")}
          tip={t("checkButton.tip")}
          onClick={check}
        />
        {info ? (
          <p className="mt-4">
            {t("versionLabel")}
            <span className="ml-1 font-semibold tabular-nums">{info.version}</span>
          </p>
        ) : null}
        {problem ? (
          <div className="mt-4">
            <Alert kind="problem">{problem}</Alert>
          </div>
        ) : null}
      </section>
    </div>
  );
}
