import { DatabaseBackup, Store } from "lucide-react";
import { useTranslation } from "react-i18next";

import { pickBackupFile } from "../../../shared/bridge/pickers";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { restoreBackup } from "../api";

type WelcomeProps = {
  onNewShop: () => void;
  // Called after the restore was asked for. The real app restarts by itself.
  onRestored: () => void;
};

// The first screen of a fresh install: a new shop, or a backup from another computer.
export function Welcome({ onNewShop, onRestored }: WelcomeProps) {
  const { t } = useTranslation("setup");
  const save = useSave();

  const restore = () =>
    save.run(async () => {
      const file = await pickBackupFile(t("welcome.pickTitle"));
      if (file) {
        await restoreBackup(file);
        onRestored();
      }
    });

  return (
    <section aria-labelledby="welcome-title" className="mx-auto flex max-w-xl flex-col gap-6 py-12">
      <h2 id="welcome-title" className="text-[28px] font-semibold">
        {t("welcome.title")}
      </h2>
      <p className="text-text-muted">{t("welcome.intro")}</p>
      <div className="flex flex-wrap gap-4">
        <ActionButton
          colour="green"
          icon={Store}
          label={t("welcome.newShop.label")}
          tip={t("welcome.newShop.tip")}
          size="till"
          onClick={onNewShop}
        />
        <ActionButton
          colour="blue"
          icon={DatabaseBackup}
          label={t("welcome.restore.label")}
          tip={t("welcome.restore.tip")}
          size="till"
          onClick={restore}
        />
      </div>
      {save.busy ? <Alert kind="info">{t("welcome.restoring")}</Alert> : null}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
    </section>
  );
}
