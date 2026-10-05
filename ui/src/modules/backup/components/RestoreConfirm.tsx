import { RotateCcw, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { BackupRow } from "../../../generated/BackupRow";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";

type RestoreConfirmProps = {
  row: BackupRow;
  busy: boolean;
  onYes: () => void;
  onNo: () => void;
};

// Asks once more before a restore, naming the backup's date (Back Office Screens).
export function RestoreConfirm({ row, busy, onYes, onNo }: RestoreConfirmProps) {
  const { t } = useTranslation("backup");
  return (
    <section
      role="alertdialog"
      aria-labelledby="restore-title"
      className="flex flex-col gap-3 rounded-ui border border-red bg-panel p-4"
    >
      <h3 id="restore-title" className="text-xl font-semibold">
        {t("restore.confirmTitle", { when: row.taken_at })}
      </h3>
      <Alert kind="warning">{t("restore.confirmText", { when: row.taken_at })}</Alert>
      {busy ? <Alert kind="info">{t("restore.busy")}</Alert> : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={X}
          label={t("restore.no.label")}
          tip={t("restore.no.tip")}
          onClick={onNo}
        />
        <ActionButton
          colour="red"
          icon={RotateCcw}
          label={t("restore.yes.label")}
          tip={t("restore.yes.tip")}
          onClick={onYes}
        />
      </div>
    </section>
  );
}
