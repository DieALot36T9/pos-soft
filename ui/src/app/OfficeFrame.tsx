import {
  Boxes,
  ClipboardList,
  DatabaseBackup,
  FolderTree,
  History,
  Home,
  KeyRound,
  Lock,
  LogOut,
  Package,
  Percent,
  Printer,
  Ruler,
  Settings,
  ShieldCheck,
  Tag,
  Users,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { Permission } from "../generated/Permission";
import type { SessionView } from "../generated/SessionView";
import { AuditLogPage } from "../modules/audit/pages/AuditLogPage";
import { MyAccountPage } from "../modules/auth/pages/MyAccountPage";
import { BackupsPage } from "../modules/backup/pages/BackupsPage";
import { BrandsPage } from "../modules/catalog/pages/BrandsPage";
import { CategoriesPage } from "../modules/catalog/pages/CategoriesPage";
import { ProductsPage } from "../modules/catalog/pages/ProductsPage";
import { UnitsPage } from "../modules/catalog/pages/UnitsPage";
import { HardwarePage } from "../modules/hardware/pages/HardwarePage";
import { StockCountsPage } from "../modules/inventory/pages/StockCountsPage";
import { StockLevelsPage } from "../modules/inventory/pages/StockLevelsPage";
import { SettingsPage } from "../modules/settings/pages/SettingsPage";
import { TaxRatesPage } from "../modules/tax/pages/TaxRatesPage";
import { RolesPage } from "../modules/users/pages/RolesPage";
import { UsersPage } from "../modules/users/pages/UsersPage";
import { errorMessage } from "../shared/bridge";
import { shortcuts } from "../shared/keyboard/shortcuts";
import { ActionButton } from "../shared/ui/ActionButton";
import { Alert } from "../shared/ui/Alert";
import { Tip } from "../shared/ui/Tip";
import { HomePage } from "./HomePage";
import { lockTill, logOut } from "./api";

type Page =
  | "home"
  | "products"
  | "stock"
  | "counts"
  | "categories"
  | "brands"
  | "units"
  | "hardware"
  | "settings"
  | "tax"
  | "backups"
  | "users"
  | "roles"
  | "audit"
  | "account";

// Each menu entry, and the permission it needs. The menu hides what the user's role
// can't open, as a convenience: Rust checks every command anyway.
const menu: { page: Page; icon: LucideIcon; needs: Permission | null }[] = [
  { page: "home", icon: Home, needs: null },
  { page: "products", icon: Package, needs: "stock_view" },
  { page: "stock", icon: Boxes, needs: "stock_view" },
  { page: "counts", icon: ClipboardList, needs: "stock_count_enter" },
  { page: "categories", icon: FolderTree, needs: "catalog_manage" },
  { page: "brands", icon: Tag, needs: "catalog_manage" },
  { page: "units", icon: Ruler, needs: "catalog_manage" },
  { page: "hardware", icon: Printer, needs: null },
  { page: "settings", icon: Settings, needs: "settings_manage" },
  { page: "tax", icon: Percent, needs: "tax_manage" },
  { page: "backups", icon: DatabaseBackup, needs: "backup_run" },
  { page: "users", icon: Users, needs: "users_manage" },
  { page: "roles", icon: ShieldCheck, needs: "users_manage" },
  { page: "audit", icon: History, needs: "audit_view" },
  { page: "account", icon: KeyRound, needs: null },
];

type OfficeFrameProps = {
  session: SessionView;
  devTools: boolean;
  backupWarning: string | null;
  // Lock the till after this many minutes without a key or mouse press (USR-03).
  idleLockMinutes: number;
  onLocked: (session: SessionView) => void;
  onSignedOut: () => void;
};

// The back office: a side menu and one page. React Router replaces this in a later task.
export function OfficeFrame({
  session,
  devTools,
  backupWarning,
  idleLockMinutes,
  onLocked,
  onSignedOut,
}: OfficeFrameProps) {
  const { t } = useTranslation("app");
  const { t: tErrors } = useTranslation("errors");
  const [page, setPage] = useState<Page>("home");
  const [problem, setProblem] = useState<string | null>(null);

  const leave = () => {
    void logOut()
      .catch(() => undefined)
      .then(onSignedOut);
  };
  const lock = () => {
    lockTill()
      .then(onLocked)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  useHotkeys(shortcuts.lock.keys, lock, { enableOnFormTags: true, preventDefault: true });

  // The idle timer starts again at every key or mouse press.
  useEffect(() => {
    let timer = window.setTimeout(lock, idleLockMinutes * 60_000);
    const restart = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(lock, idleLockMinutes * 60_000);
    };
    const events = ["keydown", "mousedown", "touchstart"] as const;
    events.forEach((name) => window.addEventListener(name, restart));
    return () => {
      window.clearTimeout(timer);
      events.forEach((name) => window.removeEventListener(name, restart));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- restart only when the setting changes.
  }, [idleLockMinutes]);

  const allowed = menu.filter(
    (entry) => entry.needs === null || session.permissions.includes(entry.needs),
  );

  return (
    <div className="flex flex-1">
      <nav
        aria-label={t("menu.label")}
        className="flex w-60 shrink-0 flex-col gap-1 border-r border-border bg-panel p-3"
      >
        {allowed.map(({ page: each, icon: Icon }) => (
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
            {t("signedInAs", { name: session.display_name, role: session.role_name })}
          </p>
          <ActionButton
            colour="grey"
            icon={Lock}
            label={t("lock.label")}
            tip={t("lock.tip")}
            shortcut={shortcuts.lock.label}
            onClick={lock}
          />
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
        {problem ? <Alert kind="problem">{problem}</Alert> : null}
        {page === "home" ? (
          <HomePage
            showStockAlerts={session.permissions.includes("stock_view")}
            onOpenStock={() => setPage("stock")}
          />
        ) : null}
        {page === "products" ? <ProductsPage /> : null}
        {page === "stock" ? (
          <StockLevelsPage canAdjust={session.permissions.includes("stock_adjust")} />
        ) : null}
        {page === "counts" ? (
          <StockCountsPage canManage={session.permissions.includes("stock_count_manage")} />
        ) : null}
        {page === "categories" ? <CategoriesPage /> : null}
        {page === "brands" ? <BrandsPage /> : null}
        {page === "units" ? <UnitsPage /> : null}
        {page === "hardware" ? <HardwarePage devTools={devTools} /> : null}
        {page === "settings" ? <SettingsPage /> : null}
        {page === "tax" ? <TaxRatesPage /> : null}
        {page === "backups" ? <BackupsPage /> : null}
        {page === "users" ? <UsersPage /> : null}
        {page === "roles" ? <RolesPage /> : null}
        {page === "audit" ? <AuditLogPage /> : null}
        {page === "account" ? <MyAccountPage /> : null}
      </main>
    </div>
  );
}
