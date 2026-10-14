import { BarChart3, Boxes, Info, Package, ShoppingCart, Users, Wallet } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { AppInfo } from "../generated/AppInfo";
import type { ShiftState } from "../generated/ShiftState";
import { StockAlertsCard } from "../modules/inventory/components/StockAlertsCard";
import { errorMessage } from "../shared/bridge";
import { formatMoney } from "../shared/format/money";
import { formatDateTime } from "../shared/format/time";
import { ActionButton } from "../shared/ui/ActionButton";
import { Alert } from "../shared/ui/Alert";
import type { Page } from "./OfficeFrame";
import { getAppInfo, shiftState } from "./api";

const activeTillShiftTitle = "Active Till Shift";
const noShiftOpenTitle = "No Shift Open";
const noShiftOpenDesc = "Open the Till to count starting float and begin selling.";
const openingFloatTitle = "Opening Float";
const openingFloatDesc = "Starting cash counted at shift open";
const expectedDrawerCashTitle = "Expected Drawer Cash";
const blindCountOnLabel = "Blind Count On";
const expectedDrawerCashDesc = "Live cash balance from sales, returns & cash moves";
const openTillNavTitle = "Open Till (Checkout)";
const openTillNavDesc = "Ring up customers, scan barcodes, and take payments.";
const productsNavTitle = "Products & Prices";
const productsNavDesc = "Add products, update prices, and generate barcodes.";
const stockNavTitle = "Stock Levels";
const stockNavDesc = "Inspect on-hand stock, expiry batches, and adjustments.";
const customersNavTitle = "Customers & Credit";
const customersNavDesc = "View loyalty points, baki balances, and take debt payments.";
const emDash = "—";
const openedAtText = (openedAt: number, businessDate: string) =>
  `Opened ${formatDateTime(openedAt)} · Day ${businessDate}`;

type HomePageProps = {
  showStockAlerts: boolean;
  onOpenStock: () => void;
  onNavigate?: (page: Page) => void;
};

export function HomePage({ showStockAlerts, onOpenStock, onNavigate }: HomePageProps) {
  const { t } = useTranslation("app");
  const { t: tErrors } = useTranslation("errors");
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [shift, setShift] = useState<ShiftState | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    shiftState()
      .then(setShift)
      .catch(() => undefined);
  }, []);

  const check = () => {
    getAppInfo()
      .then((answer) => {
        setInfo(answer);
        setProblem(null);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <div className="flex flex-col gap-5">
      {/* Top KPI Summary Cards */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div className="rounded-ui border border-border bg-panel p-4 flex flex-col justify-between">
          <div className="flex items-center justify-between text-text-muted text-xs font-bold uppercase tracking-wider">
            <span>{activeTillShiftTitle}</span>
            <Wallet className="w-4 h-4 text-blue" />
          </div>
          {shift?.shift ? (
            <div className="mt-2">
              <div className="text-xl font-bold text-text">{shift.shift.cashier_name}</div>
              <p className="text-xs text-text-muted mt-0.5 tabular-nums">
                {openedAtText(shift.shift.opened_at, shift.shift.business_date)}
              </p>
            </div>
          ) : (
            <div className="mt-2">
              <div className="text-lg font-bold text-amber">{noShiftOpenTitle}</div>
              <p className="text-xs text-text-muted mt-0.5">{noShiftOpenDesc}</p>
            </div>
          )}
        </div>

        <div className="rounded-ui border border-border bg-panel p-4 flex flex-col justify-between">
          <div className="flex items-center justify-between text-text-muted text-xs font-bold uppercase tracking-wider">
            <span>{openingFloatTitle}</span>
            <ShoppingCart className="w-4 h-4 text-green" />
          </div>
          <div className="mt-2">
            <div className="text-2xl font-extrabold text-text tabular-nums">
              {shift && shift.shift
                ? formatMoney(shift.shift.opening_float, shift.currency)
                : emDash}
            </div>
            <p className="text-xs text-text-muted mt-0.5">{openingFloatDesc}</p>
          </div>
        </div>

        <div className="rounded-ui border border-border bg-panel p-4 flex flex-col justify-between">
          <div className="flex items-center justify-between text-text-muted text-xs font-bold uppercase tracking-wider">
            <span>{expectedDrawerCashTitle}</span>
            <BarChart3 className="w-4 h-4 text-purple" />
          </div>
          <div className="mt-2">
            <div className="text-2xl font-extrabold text-text tabular-nums">
              {shift && shift.expected_cash !== null
                ? formatMoney(shift.expected_cash, shift.currency)
                : shift?.blind_close
                  ? blindCountOnLabel
                  : emDash}
            </div>
            <p className="text-xs text-text-muted mt-0.5">{expectedDrawerCashDesc}</p>
          </div>
        </div>
      </div>

      {/* Quick Action Cards */}
      {onNavigate ? (
        <section
          aria-label="Quick navigation"
          className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4"
        >
          <button
            type="button"
            onClick={() => onNavigate("checkout")}
            className="flex flex-col gap-1 rounded-ui border border-green bg-green-soft p-4 text-left cursor-pointer"
          >
            <div className="flex items-center gap-2 font-bold text-green">
              <ShoppingCart className="w-4 h-4" />
              <span>{openTillNavTitle}</span>
            </div>
            <p className="text-xs text-text-muted">{openTillNavDesc}</p>
          </button>

          <button
            type="button"
            onClick={() => onNavigate("products")}
            className="flex flex-col gap-1 rounded-ui border border-border bg-panel hover:bg-grey-soft p-4 text-left cursor-pointer"
          >
            <div className="flex items-center gap-2 font-bold text-blue">
              <Package className="w-4 h-4" />
              <span>{productsNavTitle}</span>
            </div>
            <p className="text-xs text-text-muted">{productsNavDesc}</p>
          </button>

          <button
            type="button"
            onClick={() => onNavigate("stock")}
            className="flex flex-col gap-1 rounded-ui border border-border bg-panel hover:bg-grey-soft p-4 text-left cursor-pointer"
          >
            <div className="flex items-center gap-2 font-bold text-text">
              <Boxes className="w-4 h-4 text-amber" />
              <span>{stockNavTitle}</span>
            </div>
            <p className="text-xs text-text-muted">{stockNavDesc}</p>
          </button>

          <button
            type="button"
            onClick={() => onNavigate("customers")}
            className="flex flex-col gap-1 rounded-ui border border-border bg-panel hover:bg-grey-soft p-4 text-left cursor-pointer"
          >
            <div className="flex items-center gap-2 font-bold text-text">
              <Users className="w-4 h-4 text-purple" />
              <span>{customersNavTitle}</span>
            </div>
            <p className="text-xs text-text-muted">{customersNavDesc}</p>
          </button>
        </section>
      ) : null}

      {/* Stock Alerts */}
      {showStockAlerts ? <StockAlertsCard onOpenStock={onOpenStock} /> : null}

      {/* System Status & Diagnostics Card */}
      <section className="w-full max-w-xl rounded-ui border border-border bg-panel p-5">
        <h2 className="mb-1 text-xl font-bold">{t("title")}</h2>
        <p className="mb-4 text-sm text-text-muted">{t("intro")}</p>
        <ActionButton
          colour="blue"
          icon={Info}
          label={t("checkButton.label")}
          tip={t("checkButton.tip")}
          onClick={check}
        />
        {info ? (
          <p className="mt-3 text-sm">
            {t("versionLabel")}
            <span className="ml-1 font-semibold tabular-nums">{info.version}</span>
          </p>
        ) : null}
        {problem ? (
          <div className="mt-3">
            <Alert kind="problem">{problem}</Alert>
          </div>
        ) : null}
      </section>
    </div>
  );
}
