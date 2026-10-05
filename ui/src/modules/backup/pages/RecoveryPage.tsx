import { RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { BackupRow } from "../../../generated/BackupRow";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { recoveryBackups, recoveryRestore } from "../api";
import { BackupTable } from "../components/BackupTable";

// Shown instead of everything else when the startup check fails (BAK-06).
export function RecoveryPage() {
  const { t } = useTranslation("backup");
  const [rows, setRows] = useState<BackupRow[] | null>(null);
  const restore = useSave();

  useEffect(() => {
    recoveryBackups()
      .then(setRows)
      .catch(() => setRows([]));
  }, []);

  const newest = rows?.find((row) => row.usable) ?? null;
  const restoreFile = (file: string) => restore.run(() => recoveryRestore(file));

  return (
    <section
      aria-labelledby="recovery-heading"
      className="mx-auto flex max-w-4xl flex-col gap-4 py-8"
    >
      <h2 id="recovery-heading" className="text-[28px] font-semibold text-red">
        {t("recovery.title")}
      </h2>
      <Alert kind="problem">{t("recovery.intro")}</Alert>
      {rows !== null && newest === null ? <Alert kind="warning">{t("recovery.none")}</Alert> : null}
      {newest ? (
        <div>
          <ActionButton
            colour="green"
            icon={RotateCcw}
            label={t("recovery.restoreNewest.label")}
            tip={t("recovery.restoreNewest.tip")}
            size="till"
            disabledReason={restore.busy || restore.done ? t("restore.busy") : undefined}
            onClick={() => restoreFile(newest.file)}
          />
        </div>
      ) : null}
      {restore.busy || restore.done ? <Alert kind="info">{t("restore.busy")}</Alert> : null}
      {restore.problem ? <Alert kind="problem">{restore.problem}</Alert> : null}
      {rows ? <BackupTable rows={rows} onRestore={(row) => restoreFile(row.file)} /> : null}
    </section>
  );
}
