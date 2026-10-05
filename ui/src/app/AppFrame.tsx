import { Info } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { AppInfo } from "../generated/AppInfo";
import { HardwarePage } from "../modules/hardware/pages/HardwarePage";
import { errorMessage } from "../shared/bridge";
import { ActionButton } from "../shared/ui/ActionButton";
import { getAppInfo } from "./api";

// The app window: a flat top bar, the app check and the printer and device page.
// Menus and routes come in a later milestone (C-M2).
export function AppFrame() {
  const { t } = useTranslation("app");
  const { t: tCommon } = useTranslation("common");
  const { t: tErrors } = useTranslation("errors");
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [shownVersion, setShownVersion] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  const loadInfo = (show: boolean) => {
    getAppInfo()
      .then((answer) => {
        setInfo(answer);
        setShownVersion((shown) => shown || show);
        setProblem(null);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  // The app info says whether the dev tools are built in.
  useEffect(() => {
    getAppInfo()
      .then(setInfo)
      .catch(() => undefined);
  }, []);

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text">
      <header className="border-b border-border bg-panel px-6 py-3">
        <h1 className="text-xl font-semibold">{tCommon("appName")}</h1>
      </header>
      <main className="flex flex-1 flex-col items-center gap-6 p-6">
        <section className="w-full max-w-xl rounded-ui border border-border bg-panel p-6">
          <h2 className="mb-2 text-[28px] font-semibold">{t("title")}</h2>
          <p className="mb-6 text-text-muted">{t("intro")}</p>
          <ActionButton
            colour="blue"
            icon={Info}
            label={t("checkButton.label")}
            tip={t("checkButton.tip")}
            onClick={() => loadInfo(true)}
          />
          {info && shownVersion ? (
            <p className="mt-4">
              {t("versionLabel")}
              <span className="ml-1 font-semibold tabular-nums">{info.version}</span>
            </p>
          ) : null}
          {problem ? (
            <p role="alert" className="mt-4 rounded-ui border border-red bg-red-soft p-3 text-red">
              {problem}
            </p>
          ) : null}
        </section>
        {info ? <HardwarePage devTools={info.dev_tools} /> : null}
      </main>
    </div>
  );
}
