import { BookOpen, Percent, Star, Tags, User, UserX } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CartView } from "../../../generated/CartView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { formatMoney } from "../../../shared/format/money";
import { formatPercent } from "../../../shared/format/percent";
import { Badge } from "../../../shared/ui/Badge";

// The strip over the cart: who the customer is, their tier, points and debt, and the
// price list or whole-sale discount in use (CHK-11, CHK-12, CHK-17). All from Rust.
export function CustomerBar({ cart, currency }: { cart: CartView; currency: CurrencyFormat }) {
  const { t } = useTranslation("checkout");
  const money = (minor: number) => formatMoney(minor, currency);
  const { customer } = cart;
  const discount = cart.invoice_discount;
  return (
    <div className="flex flex-wrap items-center gap-2" aria-label={t("buttons.customer.label")}>
      {customer ? (
        <>
          <Badge
            colour="blue"
            icon={User}
            word={t("customerBar.customer", { name: customer.name, phone: customer.phone })}
            tip={t("customerBar.customerTip")}
          />
          {customer.tier_name ? (
            <Badge
              colour="purple"
              icon={Tags}
              word={t("customerBar.tier", { tier: customer.tier_name })}
              tip={t("customerBar.tierTip")}
            />
          ) : null}
          <Badge
            colour="purple"
            icon={Star}
            word={t("customerBar.points", { points: customer.points })}
            tip={t("customerBar.pointsTip")}
          />
          {customer.credit > 0 ? (
            <Badge
              colour="amber"
              icon={BookOpen}
              word={t("customerBar.owes", { amount: money(customer.credit) })}
              tip={t("customerBar.owesTip")}
            />
          ) : null}
          {customer.credit < 0 ? (
            <Badge
              colour="green"
              icon={BookOpen}
              word={t("customerBar.storeCredit", { amount: money(-customer.credit) })}
              tip={t("customerBar.storeCreditTip")}
            />
          ) : null}
        </>
      ) : (
        <Badge
          colour="grey"
          icon={UserX}
          word={t("customerBar.none")}
          tip={t("customerBar.noneTip")}
        />
      )}
      {cart.price_list ? (
        <Badge
          colour="purple"
          icon={Tags}
          word={t("customerBar.priceList", { name: cart.price_list.name })}
          tip={t("customerBar.priceListTip", { name: cart.price_list.name })}
        />
      ) : null}
      {discount ? (
        <Badge
          colour="purple"
          icon={Percent}
          word={t("customerBar.invoiceDiscount", {
            value:
              discount.kind === "percent" ? formatPercent(discount.value) : money(discount.value),
          })}
          tip={t("customerBar.invoiceDiscountTip")}
        />
      ) : null}
    </div>
  );
}
