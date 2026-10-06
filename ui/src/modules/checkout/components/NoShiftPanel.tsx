import { PlayCircle, Power } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ShiftView } from "../../../generated/ShiftView";
import { ActionButton } from "../../../shared/ui/ActionButton";

type NoShiftPanelProps = {
  // The shift someone else has on this till, if any (SHF-02).
  otherShift: ShiftView | null;
  onOpenShift: () => void;
  onForceClose: () => void;
};

// Shown instead of the cart while the cashier has no shift on this till (CHK-20).
export function NoShiftPanel({ otherShift, onOpenShift, onForceClose }: NoShiftPanelProps) {
  const { t } = useTranslation("checkout");
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <h2 className="text-[28px] font-semibold">{t("noShift.title")}</h2>
      {otherShift ? (
        <>
          <p className="text-xl">{t("noShift.other", { name: otherShift.cashier_name })}</p>
          <ActionButton
            colour="red"
            icon={Power}
            size="till"
            label={t("noShift.forceClose.label")}
            tip={t("noShift.forceClose.tip")}
            onClick={onForceClose}
          />
        </>
      ) : (
        <ActionButton
          colour="green"
          icon={PlayCircle}
          size="till"
          label={t("noShift.open.label")}
          tip={t("noShift.open.tip")}
          onClick={onOpenShift}
        />
      )}
    </div>
  );
}
