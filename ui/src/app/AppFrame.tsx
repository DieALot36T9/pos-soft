import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { AppInfo } from "../generated/AppInfo";
import type { StartupView } from "../generated/StartupView";
import { RecoveryPage } from "../modules/backup/pages/RecoveryPage";
import { SetupWizard } from "../modules/setup/pages/SetupWizard";
import { Alert } from "../shared/ui/Alert";
import { OfficeFrame } from "./OfficeFrame";
import { SignInPage } from "./SignInPage";
import { getAppInfo, getStartup } from "./api";

// The app window. Which screen shows depends on Rust's startup answer: the recovery
// screen (damaged data, BAK-06), the setup wizard (SET-01), sign-in, or the back office.
export function AppFrame() {
  const { t: tCommon } = useTranslation("common");
  const { t } = useTranslation("app");
  const [startup, setStartup] = useState<StartupView | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [problem, setProblem] = useState(false);

  const reload = () => {
    getStartup()
      .then((view) => {
        setStartup(view);
        setProblem(false);
      })
      .catch(() => setProblem(true));
  };

  useEffect(() => {
    reload();
    // The app info says whether the dev tools are built in.
    getAppInfo()
      .then(setInfo)
      .catch(() => undefined);
  }, []);

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text">
      <header className="border-b border-border bg-panel px-6 py-3">
        <h1 className="text-xl font-semibold">{tCommon("appName")}</h1>
      </header>
      {problem ? (
        <div className="p-6">
          <Alert kind="problem">{t("startProblem")}</Alert>
        </div>
      ) : startup === null ? (
        <p className="p-6">{t("loading")}</p>
      ) : startup.damaged ? (
        <RecoveryPage />
      ) : !startup.setup.is_complete ? (
        <div className="p-6">
          <SetupWizard status={startup.setup} onFinished={reload} />
        </div>
      ) : startup.session === null ? (
        <div className="p-6">
          <SignInPage onSignedIn={(session) => setStartup({ ...startup, session })} />
        </div>
      ) : (
        <OfficeFrame
          session={startup.session}
          devTools={info?.dev_tools ?? false}
          backupWarning={startup.backup_warning}
          onSignedOut={reload}
        />
      )}
    </div>
  );
}
