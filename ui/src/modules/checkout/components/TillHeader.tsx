import { useTranslation } from "react-i18next";

import type { ShiftView } from "../../../generated/ShiftView";
import { formatDateTime } from "../../../shared/format/time";

// The strip at the top of the till: who has the shift and since when.
export function TillHeader({ shift }: { shift: ShiftView | null }) {
  const { t } = useTranslation("checkout");
  const since = shift ? formatDateTime(shift.opened_at).slice(11) : "";
  return (
    <header className="flex items-center justify-between border-b border-border bg-panel px-4 py-2">
      <span className="font-semibold">
        {shift ? t("header.shift", { name: shift.cashier_name, time: since }) : t("header.noShift")}
      </span>
    </header>
  );
}
