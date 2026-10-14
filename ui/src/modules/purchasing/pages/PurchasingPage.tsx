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

export function PurchasingPage({ permissions }: { permissions: Permission[] }) {
  const { t } = useTranslation("purchasing");
  const [area, setArea] = useState<Area>("suppliers");
  const [receiveOrder, setReceiveOrder] = useState<string | null>(null);

  return (
    <section aria-labelledby="purchasing-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="purchasing-heading" className="text-[28px] font-semibold">
          {t("page.title")}
        </h2>
        <p className="text-text-muted">{t("page.intro")}</p>
      </div>
      <div
        role="tablist"
        aria-label={t("tabs.label")}
        className="flex flex-wrap gap-1.5 rounded-ui border border-border bg-panel p-1.5"
      >
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
                "min-h-10 rounded-ui border px-4 text-sm cursor-pointer " +
                (each === area
                  ? "border-blue bg-blue-soft font-bold text-blue"
                  : "border-transparent bg-panel text-text hover:bg-grey-soft")
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
