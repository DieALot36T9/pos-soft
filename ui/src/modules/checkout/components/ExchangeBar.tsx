import { ArrowLeftRight, Banknote } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ExchangeView } from "../../../generated/ExchangeView";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Badge } from "../../../shared/ui/Badge";

type ExchangeBarProps = {
  exchange: ExchangeView;
  currency: CurrencyFormat;
  onCashOut: () => void;
};

// Over the cart of an exchange: the return's credit that pays for this sale (RET-05),
// and the way out when the customer takes nothing new.
export function ExchangeBar({ exchange, currency, onCashOut }: ExchangeBarProps) {
  const { t } = useTranslation("checkout");
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 rounded-ui border border-blue bg-blue-soft p-2">
      <Badge
        colour="blue"
        icon={ArrowLeftRight}
        word={t("exchangeBar.credit", {
          number: exchange.return_no,
          amount: formatMoney(exchange.credit, currency),
        })}
        tip={t("exchangeBar.creditTip")}
      />
      <ActionButton
        colour="red"
        icon={Banknote}
        label={t("exchangeBar.cashOut.label")}
        tip={t("exchangeBar.cashOut.tip")}
        onClick={onCashOut}
      />
    </div>
  );
}
