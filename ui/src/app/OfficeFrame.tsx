import {
  BadgePercent,
  ArrowLeftRight,
  BarChart3,
  Barcode,
  Boxes,
  ShoppingCart,
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
  Contact,
  Tag,
  Truck,
  Users,
  Search,
  ChevronDown,
  ChevronRight,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { Permission } from "../generated/Permission";
import type { SessionView } from "../generated/SessionView";
import { AuditLogPage } from "../modules/audit/pages/AuditLogPage";
import { MyAccountPage } from "../modules/auth/pages/MyAccountPage";
import { CustomersPage } from "../modules/customers/pages/CustomersPage";
import { BackupsPage } from "../modules/backup/pages/BackupsPage";
import { BrandsPage } from "../modules/catalog/pages/BrandsPage";
import { CategoriesPage } from "../modules/catalog/pages/CategoriesPage";
import { LabelsPage } from "../modules/catalog/pages/LabelsPage";
import { ProductsPage } from "../modules/catalog/pages/ProductsPage";
import { UnitsPage } from "../modules/catalog/pages/UnitsPage";
import { HardwarePage } from "../modules/hardware/pages/HardwarePage";
import { ImportExportPage } from "../modules/io/pages/ImportExportPage";
import { PricingPage } from "../modules/pricing/pages/PricingPage";
import { PurchasingPage } from "../modules/purchasing/pages/PurchasingPage";
import { ReportsPage } from "../modules/reports/pages/ReportsPage";
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
import { CommandPalette, type CommandEntry } from "./CommandPalette";
import { HomePage } from "./HomePage";
import { lockTill, logOut } from "./api";
import { useIdleLock } from "./useIdleLock";

export type Page =
  | "home"
  | "checkout"
  | "customers"
  | "products"
  | "stock"
  | "counts"
  | "categories"
  | "brands"
  | "units"
  | "labels"
  | "pricing"
  | "purchasing"
  | "reports"
  | "io"
  | "hardware"
  | "settings"
  | "tax"
  | "backups"
  | "users"
  | "roles"
  | "audit"
  | "account";

interface NavItemDef {
  page: Page;
  icon: LucideIcon;
  needs: Permission | Permission[] | null;
  description: string;
}

interface NavGroupDef {
  id: string;
  title: string;
  items: NavItemDef[];
}

const NAV_GROUPS: NavGroupDef[] = [
  {
    id: "overview",
    title: "Overview",
    items: [
      {
        page: "home",
        icon: Home,
        needs: null,
        description: "Today's shift status, stock alerts, quick actions, and system diagnostics.",
      },
    ],
  },
  {
    id: "sell",
    title: "Sell",
    items: [
      {
        page: "checkout",
        icon: ShoppingCart,
        needs: "checkout_sell",
        description: "Switch to full-screen cashier checkout mode.",
      },
    ],
  },
  {
    id: "catalog",
    title: "Catalog",
    items: [
      {
        page: "products",
        icon: Package,
        needs: "stock_view",
        description: "Add, edit, and search products, prices, barcodes, kits, and variants.",
      },
      {
        page: "categories",
        icon: FolderTree,
        needs: "catalog_manage",
        description: "Organize the product category tree.",
      },
      {
        page: "brands",
        icon: Tag,
        needs: "catalog_manage",
        description: "Manage product brand names.",
      },
      {
        page: "units",
        icon: Ruler,
        needs: "catalog_manage",
        description: "Manage units of measure and decimal precision.",
      },
      {
        page: "labels",
        icon: Barcode,
        needs: "labels_print",
        description: "Print barcode stickers and shelf price tags on thermal or A4 sheets.",
      },
    ],
  },
  {
    id: "pricing",
    title: "Pricing",
    items: [
      {
        page: "pricing",
        icon: BadgePercent,
        needs: "pricing_manage",
        description: "Manage wholesale price lists, member tiers, and promotional offers.",
      },
    ],
  },
  {
    id: "stock",
    title: "Stock",
    items: [
      {
        page: "stock",
        icon: Boxes,
        needs: "stock_view",
        description: "Check on-hand quantities, expiry batches, ledger history, and adjust stock.",
      },
      {
        page: "counts",
        icon: ClipboardList,
        needs: "stock_count_enter",
        description: "Count physical shelves, review variances, and post stock updates.",
      },
    ],
  },
  {
    id: "buying",
    title: "Buying",
    items: [
      {
        page: "purchasing",
        icon: Truck,
        needs: "purchasing_manage",
        description: "Manage suppliers, purchase orders, goods receiving (GRN), and returns.",
      },
    ],
  },
  {
    id: "customers",
    title: "Customers",
    items: [
      {
        page: "customers",
        icon: Contact,
        needs: "customer_view_create",
        description: "Look up customers, loyalty points, credit balances, and take payments.",
      },
    ],
  },
  {
    id: "reports",
    title: "Reports & Data",
    items: [
      {
        page: "reports",
        icon: BarChart3,
        needs: ["reports_view_all", "shift_open_close_own"],
        description: "Run daily sales, profit, tax, shift, and stock movement reports.",
      },
      {
        page: "io",
        icon: ArrowLeftRight,
        needs: ["data_import", "data_export"],
        description: "Bulk import and export products, stock, prices, customers, or suppliers.",
      },
    ],
  },
  {
    id: "admin",
    title: "Admin",
    items: [
      {
        page: "users",
        icon: Users,
        needs: "users_manage",
        description: "Manage cashier and manager logins, PINs, and passwords.",
      },
      {
        page: "roles",
        icon: ShieldCheck,
        needs: "users_manage",
        description: "Control what each staff role is allowed to view or approve.",
      },
      {
        page: "account",
        icon: KeyRound,
        needs: null,
        description: "Change your own password or PIN.",
      },
      {
        page: "settings",
        icon: Settings,
        needs: "settings_manage",
        description: "Configure shop profile, currency rounding, receipt layout, and rules.",
      },
      {
        page: "tax",
        icon: Percent,
        needs: "tax_manage",
        description: "Configure VAT/GST rates and tax components.",
      },
      {
        page: "hardware",
        icon: Printer,
        needs: null,
        description: "Set up and test receipt printers, cash drawers, scales, and label printers.",
      },
      {
        page: "backups",
        icon: DatabaseBackup,
        needs: "backup_run",
        description: "Create instant database backups, verify backup health, or restore.",
      },
      {
        page: "audit",
        icon: History,
        needs: "audit_view",
        description: "Review security logs of voids, discounts, overrides, and settings changes.",
      },
    ],
  },
];

type OfficeFrameProps = {
  session: SessionView;
  devTools: boolean;
  backupWarning: string | null;
  idleLockMinutes: number;
  onLocked: (session: SessionView) => void;
  onSignedOut: () => void;
  onOpenTill: () => void;
};

const backOfficeBadge = "Back Office";
const slashChar = "/";
const searchPlaceholder = "Search or jump to screen…";
const ctrlKLabel = "Ctrl+K";
const openTillButtonText = "Open Till";
const breadcrumbPath = (group: string, item: string) => `${group} › ${item}`;

export function OfficeFrame({
  session,
  devTools,
  backupWarning,
  idleLockMinutes,
  onLocked,
  onSignedOut,
  onOpenTill,
}: OfficeFrameProps) {
  const { t } = useTranslation("app");
  const { t: tErrors } = useTranslation("errors");
  const [page, setPage] = useState<Page>("home");
  const [problem, setProblem] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({});

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
  useHotkeys("meta+k, ctrl+k", () => setPaletteOpen(true), {
    enableOnFormTags: true,
    preventDefault: true,
  });

  useIdleLock(idleLockMinutes, lock);

  const canAccess = (needs: Permission | Permission[] | null) => {
    if (needs === null) return true;
    const list = Array.isArray(needs) ? needs : [needs];
    return list.some((p) => session.permissions.includes(p));
  };

  const toggleGroup = (id: string) => {
    setCollapsedGroups((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  let activeGroupTitle = "Overview";
  let activeItemDesc = "Today's shift status, stock alerts, quick actions, and system diagnostics.";
  let activeItemLabel = t("menu.home.label");

  for (const group of NAV_GROUPS) {
    for (const item of group.items) {
      if (item.page === page) {
        activeGroupTitle = group.title;
        activeItemDesc = item.description;
        activeItemLabel = t(`menu.${item.page}.label`);
        break;
      }
    }
  }

  const commandEntries: CommandEntry[] = [];
  if (canAccess("checkout_sell")) {
    commandEntries.push({
      id: "cmd-till",
      group: "Sell",
      label: "Open Till (Checkout)",
      description: "Switch to full-screen cashier checkout mode",
      shortcut: "F1",
      onSelect: onOpenTill,
    });
  }
  commandEntries.push({
    id: "cmd-lock",
    group: "Security",
    label: "Lock Till",
    description: "Lock the terminal and require PIN/password to unlock",
    shortcut: shortcuts.lock.label,
    onSelect: lock,
  });
  commandEntries.push({
    id: "cmd-logout",
    group: "Security",
    label: "Sign Out",
    description: "Sign out of your session",
    onSelect: leave,
  });

  NAV_GROUPS.forEach((group) => {
    group.items.forEach((item) => {
      if (canAccess(item.needs)) {
        commandEntries.push({
          id: `cmd-${item.page}`,
          group: group.title,
          label: t(`menu.${item.page}.label`),
          description: item.description,
          onSelect: () => (item.page === "checkout" ? onOpenTill() : setPage(item.page)),
        });
      }
    });
  });

  return (
    <div className="flex flex-1 flex-col h-full bg-bg text-text overflow-hidden">
      {/* Top Bar: Quick Jump Search (Ctrl+K) + User Info + Till Switch */}
      <div className="h-12 border-b border-border bg-panel px-4 flex items-center justify-between gap-4 shrink-0">
        <div className="flex items-center gap-2 text-xs text-text-muted font-medium">
          <span className="px-2 py-0.5 rounded-ui font-bold bg-blue-soft text-blue border border-blue">
            {backOfficeBadge}
          </span>
          <span>{activeGroupTitle}</span>
          <span>{slashChar}</span>
          <span className="text-text font-bold">{activeItemLabel}</span>
        </div>

        <button
          type="button"
          onClick={() => setPaletteOpen(true)}
          title="Search screens and actions (Ctrl+K)"
          className="flex items-center gap-2.5 px-3 py-1.5 rounded-ui border border-border bg-bg hover:bg-grey-soft text-text-muted text-xs cursor-pointer w-72 max-w-sm"
        >
          <Search className="w-3.5 h-3.5 text-blue shrink-0" />
          <span className="truncate flex-1 text-left">{searchPlaceholder}</span>
          <kbd className="px-1.5 py-0.5 font-mono text-[10px] bg-panel border border-border rounded">
            <span>{ctrlKLabel}</span>
          </kbd>
        </button>

        <div className="flex items-center gap-2">
          {canAccess("checkout_sell") && (
            <button
              type="button"
              onClick={onOpenTill}
              title="Open Till checkout screen"
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-ui border border-green bg-green text-white text-xs font-bold cursor-pointer"
            >
              <ShoppingCart className="w-3.5 h-3.5" />
              <span>{openTillButtonText}</span>
            </button>
          )}
        </div>
      </div>

      {/* Main Split: Sidebar + Content */}
      <div className="flex flex-1 overflow-hidden">
        <nav
          aria-label={t("menu.label")}
          className="flex w-64 shrink-0 flex-col gap-1 border-r border-border bg-panel p-3 overflow-y-auto"
        >
          {NAV_GROUPS.map((group) => {
            const allowedItems = group.items.filter((item) => canAccess(item.needs));
            if (allowedItems.length === 0) return null;
            const isCollapsed = !!collapsedGroups[group.id];

            return (
              <div key={group.id} className="mb-2">
                <button
                  type="button"
                  onClick={() => toggleGroup(group.id)}
                  className="flex items-center justify-between w-full px-2 py-1 text-[11px] font-bold uppercase tracking-wider text-text-muted hover:text-text cursor-pointer"
                >
                  <span>{group.title}</span>
                  {isCollapsed ? (
                    <ChevronRight className="w-3 h-3" />
                  ) : (
                    <ChevronDown className="w-3 h-3" />
                  )}
                </button>

                {!isCollapsed && (
                  <div className="flex flex-col gap-0.5 mt-0.5">
                    {allowedItems.map(({ page: each, icon: Icon }) => (
                      <Tip key={each} tip={t(`menu.${each}.tip`)}>
                        <button
                          type="button"
                          aria-current={each === page ? "page" : undefined}
                          onClick={() => (each === "checkout" ? onOpenTill() : setPage(each))}
                          className={
                            "flex min-h-9 items-center gap-2 rounded-ui border px-2.5 text-sm text-left cursor-pointer " +
                            (each === page
                              ? "border-blue bg-blue-soft font-semibold text-blue"
                              : "border-transparent text-text hover:bg-grey-soft")
                          }
                        >
                          <Icon aria-hidden="true" className="w-4 h-4 shrink-0" />
                          <span className="truncate">{t(`menu.${each}.label`)}</span>
                        </button>
                      </Tip>
                    ))}
                  </div>
                )}
              </div>
            );
          })}

          <div className="mt-auto flex flex-col gap-2 border-t border-border pt-3">
            <p className="text-xs font-medium text-text-muted">
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

        <main className="flex flex-1 flex-col gap-4 p-6 overflow-y-auto">
          <div className="rounded-ui border border-border bg-panel px-4 py-2.5 flex items-center justify-between gap-4">
            <div>
              <span className="text-xs font-bold uppercase tracking-wider text-blue">
                {breadcrumbPath(activeGroupTitle, activeItemLabel)}
              </span>
              <p className="text-sm text-text-muted mt-0.5">{activeItemDesc}</p>
            </div>
          </div>

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
              onNavigate={(target) => (target === "checkout" ? onOpenTill() : setPage(target))}
            />
          ) : null}
          {page === "customers" ? <CustomersPage permissions={session.permissions} /> : null}
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
          {page === "labels" ? <LabelsPage /> : null}
          {page === "pricing" ? <PricingPage /> : null}
          {page === "purchasing" ? <PurchasingPage permissions={session.permissions} /> : null}
          {page === "reports" ? <ReportsPage permissions={session.permissions} /> : null}
          {page === "io" ? <ImportExportPage permissions={session.permissions} /> : null}
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

      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        entries={commandEntries}
      />
    </div>
  );
}
