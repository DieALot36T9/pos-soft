import { DatabaseBackup } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { BackupRow } from "../../../generated/BackupRow";
import { useApproval } from "../../../shared/approval/useApproval";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { backupNow, listBackups, restoreBackup } from "../api";
import { BackupTable } from "../components/BackupTable";
import { RestoreConfirm } from "../components/RestoreConfirm";

// Backups: the list, "Back up now", and restore with a confirmation (BAK-04, BAK-05).
export function BackupsPage() {
  const { t } = useTranslation("backup");
  const { t: tErrors } = useTranslation("errors");
  const [rows, setRows] = useState<BackupRow[]>([]);
  const [listProblem, setListProblem] = useState<string | null>(null);
  const [made, setMade] = useState<string | null>(null);
  const [asking, setAsking] = useState<BackupRow | null>(null);
  const backup = useSave();
  const restore = useSave();
  const approval = useApproval();

  const load = () => {
    listBackups()
      .then(setRows)
      .catch((error: unknown) => setListProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the page opens.
  useEffect(load, []);

  const now = () =>
    backup.run(backupNow, (done) => {
      setMade(done.file);
      load();
    });

  return (
    <section aria-labelledby="backups-heading" className="flex flex-col gap-4">
      <div className="flex items-start justify-between">
        <div>
          <h2 id="backups-heading" className="text-[28px] font-semibold">
            {t("page.title")}
          </h2>
          <p className="text-text-muted">{t("page.intro")}</p>
        </div>
        <ActionButton
          colour="green"
          icon={DatabaseBackup}
          label={t("now.label")}
          tip={t("now.tip")}
          disabledReason={backup.busy ? t("now.busy") : undefined}
          onClick={now}
        />
      </div>
      {made && backup.done ? <Alert kind="done">{t("now.done", { file: made })}</Alert> : null}
      {backup.problem ? <Alert kind="problem">{backup.problem}</Alert> : null}
      {listProblem ? <Alert kind="problem">{listProblem}</Alert> : null}
      {restore.problem ? <Alert kind="problem">{restore.problem}</Alert> : null}
      {asking ? (
        <RestoreConfirm
          row={asking}
          busy={restore.busy || restore.done}
          onYes={() =>
            restore.run(() => approval.withApproval((token) => restoreBackup(asking.file, token)))
          }
          onNo={() => setAsking(null)}
        />
      ) : null}
      <BackupTable rows={rows} onRestore={setAsking} />
      {approval.dialog}
    </section>
  );
}
