import {
  DatabaseBackup,
  Home,
  LogOut,
  Percent,
  Printer,
  Settings,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { SessionView } from "../generated/SessionView";
import { BackupsPage } from "../modules/backup/pages/BackupsPage";
import { HardwarePage } from "../modules/hardware/pages/HardwarePage";
import { SettingsPage } from "../modules/settings/pages/SettingsPage";
import { TaxRatesPage } from "../modules/tax/pages/TaxRatesPage";
import { errorMessage } from "../shared/bridge";
import { ActionButton } from "../shared/ui/ActionButton";
import { Alert } from "../shared/ui/Alert";
import { Tip } from "../shared/ui/Tip";
import { HomePage } from "./HomePage";
import { signOut } from "./api";

type Page = "home" | "hardware" | "settings" | "tax" | "backups";

const menu: { page: Page; icon: LucideIcon }[] = [
  { page: "home", icon: Home },
  { page: "hardware", icon: Printer },
  { page: "settings", icon: Settings },
  { page: "tax", icon: Percent },
  { page: "backups", icon: DatabaseBackup },
];

type OfficeFrameProps = {
  session: SessionView;
  devTools: boolean;
  backupWarning: string | null;
  onSignedOut: () => void;
};

// The back office: a side menu and one page. React Router replaces this in a later task.
export function OfficeFrame({ session, devTools, backupWarning, onSignedOut }: OfficeFrameProps) {
  const { t } = useTranslation("app");
  const { t: tErrors } = useTranslation("errors");
  const [page, setPage] = useState<Page>("home");

  const leave = () => {
    void signOut()
      .catch(() => undefined)
      .then(onSignedOut);
  };

  return (
    <div className="flex flex-1">
      <nav
        aria-label={t("menu.label")}
        className="flex w-60 shrink-0 flex-col gap-1 border-r border-border bg-panel p-3"
      >
        {menu.map(({ page: each, icon: Icon }) => (
          <Tip key={each} tip={t(`menu.${each}.tip`)}>
            <button
              type="button"
              aria-current={each === page ? "page" : undefined}
              onClick={() => setPage(each)}
              className={
                "flex min-h-10 items-center gap-2 rounded-ui border px-3 text-left " +
                (each === page
                  ? "border-blue bg-blue-soft font-semibold text-blue"
                  : "border-transparent")
              }
            >
              <Icon aria-hidden="true" size={20} />
              {t(`menu.${each}.label`)}
            </button>
          </Tip>
        ))}
        <div className="mt-auto flex flex-col gap-2 border-t border-border pt-3">
          <p className="text-sm text-text-muted">
            {t("signedInAs", { name: session.display_name })}
          </p>
          <ActionButton
            colour="grey"
            icon={LogOut}
            label={t("signOut.label")}
            tip={t("signOut.tip")}
            onClick={leave}
          />
        </div>
      </nav>
      <main className="flex flex-1 flex-col gap-4 p-6">
        {backupWarning ? (
          <Alert kind="warning">
            {errorMessage({ kind: "conflict", code: backupWarning, params: {} }, tErrors)}
          </Alert>
        ) : null}
        {page === "home" ? <HomePage /> : null}
        {page === "hardware" ? <HardwarePage devTools={devTools} /> : null}
        {page === "settings" ? <SettingsPage /> : null}
        {page === "tax" ? <TaxRatesPage /> : null}
        {page === "backups" ? <BackupsPage /> : null}
      </main>
    </div>
  );
}
