import { Archive, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Badge } from "../../../shared/ui/Badge";
import { usePrintouts } from "../usePrintouts";

// The Preview printer at the till (HW-10): the newest receipt, report or drawer pulse,
// small at the side, until "Close". A real printer leaves nothing here.
export function TillPreviewPanel() {
  const { t } = useTranslation("hardware");
  const { printouts, clear } = usePrintouts();
  const newest = printouts[0];
  if (newest === undefined) {
    return null;
  }
  // The newest printout with a picture; a drawer pulse alone has none.
  const picture = printouts.find((each) => each.pictures.length > 0)?.pictures[0];
  return (
    <aside
      aria-label={t("tillPreview.heading")}
      className="fixed top-16 right-3 z-30 flex max-h-[calc(100vh-5rem)] w-72 flex-col gap-2 overflow-auto rounded-ui border border-border bg-panel p-3"
    >
      <div className="flex items-center justify-between gap-2">
        <h3 className="font-semibold">{t("tillPreview.heading")}</h3>
        <ActionButton
          colour="grey"
          icon={X}
          label={t("tillPreview.close.label")}
          tip={t("tillPreview.close.tip")}
          onClick={clear}
        />
      </div>
      {newest.drawer_opened ? (
        <div>
          <Badge
            colour="green"
            icon={Archive}
            word={t("preview.drawerOpened")}
            tip={t("preview.drawerTip")}
          />
        </div>
      ) : null}
      {picture ? (
        <img
          src={picture}
          alt={t("tillPreview.pictureAlt")}
          className="w-full border border-border [image-rendering:pixelated]"
        />
      ) : null}
    </aside>
  );
}
