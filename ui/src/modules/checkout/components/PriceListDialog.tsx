import { X } from "lucide-react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { PriceListChoice } from "../../../generated/PriceListChoice";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Dialog } from "../../../shared/ui/Dialog";
import { Tip } from "../../../shared/ui/Tip";

type PriceListDialogProps = {
  lists: PriceListChoice[];
  current: string | null;
  // A list's id, or null for normal prices.
  onPick: (priceListId: string | null) => void;
  onClose: () => void;
};

// Sells this sale at another price list, such as wholesale (CHK-17). Without
// `PriceListSwitch` an admin approves.
export function PriceListDialog({ lists, current, onPick, onClose }: PriceListDialogProps) {
  const { t } = useTranslation("checkout");
  useHotkeys("escape", onClose, { enableOnFormTags: true });
  const choices = [
    { id: null, name: t("priceList.normal.label"), tip: t("priceList.normal.tip") },
    ...lists.map((list) => ({
      id: list.id,
      name: list.name,
      tip: t("priceList.pickTip", { name: list.name }),
    })),
  ];
  return (
    <Dialog title={t("priceList.title")} colour="purple">
      <ul className="flex flex-col gap-1">
        {choices.map((choice) => (
          <li key={choice.id ?? "normal"}>
            <Tip tip={choice.tip}>
              <button
                type="button"
                onClick={() => onPick(choice.id)}
                aria-current={choice.id === current ? "true" : undefined}
                className={
                  "flex min-h-10 w-full justify-between rounded-ui border px-3 py-2 text-left " +
                  (choice.id === current ? "border-purple bg-purple-soft" : "border-border")
                }
              >
                <span className="font-semibold">{choice.name}</span>
                {choice.id === current ? (
                  <span className="text-purple">{t("priceList.current")}</span>
                ) : null}
              </button>
            </Tip>
          </li>
        ))}
      </ul>
      <div className="flex justify-start">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("priceList.back.label")}
          tip={t("priceList.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
      </div>
    </Dialog>
  );
}
