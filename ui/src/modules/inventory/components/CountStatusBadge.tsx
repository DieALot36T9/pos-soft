import { Ban, CheckCircle, ClipboardList } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CountStatus } from "../../../generated/CountStatus";
import { Badge } from "../../../shared/ui/Badge";

// Open (blue), posted (green) or cancelled (grey).
export function CountStatusBadge({ status }: { status: CountStatus }) {
  const { t } = useTranslation("inventory");
  const looks = {
    open: { colour: "blue", icon: ClipboardList },
    posted: { colour: "green", icon: CheckCircle },
    cancelled: { colour: "grey", icon: Ban },
  } as const;
  const { colour, icon } = looks[status];
  return (
    <Badge
      colour={colour}
      icon={icon}
      word={t(`counts.status.${status}`)}
      tip={t(`counts.status.${status}Tip`)}
    />
  );
}
