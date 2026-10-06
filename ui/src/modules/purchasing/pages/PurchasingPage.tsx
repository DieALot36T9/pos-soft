import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { Permission } from "../../../generated/Permission";
import { Tip } from "../../../shared/ui/Tip";
import { OrdersPanel } from "../components/OrdersPanel";
import { PayablesPanel } from "../components/PayablesPanel";
import { ReceivePanel } from "../components/ReceivePanel";
import { ReturnsPanel } from "../components/ReturnsPanel";
import { SuppliersPanel } from "../components/SuppliersPanel";

type Area = "suppliers" | "orders" | "receive" | "returns" | "payables";
const AREAS: Area[] = ["suppliers", "orders", "receive", "returns", "payables"];

// Purchasing (Req - Purchasing): suppliers, purchase orders, receiving goods, supplier
// returns and what the shop owes. Needs PurchasingManage; paying needs PurchasingPay.
export function PurchasingPage({ permissions }: { permissions: Permission[] }) {
  const { t } = useTranslation("purchasing");
  const [area, setArea] = useState<Area>("suppliers");
  // "Receive goods" on an order opens the receiving tab with that order.
  const [receiveOrder, setReceiveOrder] = useState<string | null>(null);

  return (
    <section aria-labelledby="purchasing-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="purchasing-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>
      <div role="tablist" aria-label={t("tabs.label")} className="flex flex-wrap gap-2">
        {AREAS.map((each) => (
          <Tip key={each} tip={t(`tabs.${each}.tip`)}>
            <button
              type="button"
              role="tab"
              aria-selected={each === area}
              onClick={() => {
                setReceiveOrder(null);
                setArea(each);
              }}
              className={
                "min-h-10 rounded-ui border px-4 " +
                (each === area
                  ? "border-blue bg-blue-soft font-semibold text-blue"
                  : "border-border bg-panel")
              }
            >
              {t(`tabs.${each}.label`)}
            </button>
          </Tip>
        ))}
      </div>
      {area === "suppliers" ? (
        <SuppliersPanel canPay={permissions.includes("purchasing_pay")} />
      ) : null}
      {area === "orders" ? (
        <OrdersPanel
          onReceive={(orderId) => {
            setReceiveOrder(orderId);
            setArea("receive");
          }}
        />
      ) : null}
      {area === "receive" ? <ReceivePanel startOrder={receiveOrder} /> : null}
      {area === "returns" ? <ReturnsPanel /> : null}
      {area === "payables" ? <PayablesPanel /> : null}
    </section>
  );
}
