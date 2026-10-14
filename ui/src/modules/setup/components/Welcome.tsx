import { DatabaseBackup, Store } from "lucide-react";
import { useTranslation } from "react-i18next";

import { pickBackupFile } from "../../../shared/bridge/pickers";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { restoreBackup } from "../api";

type WelcomeProps = {
  onNewShop: () => void;
  onRestored: () => void;
};

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
    <section
      aria-labelledby="welcome-title"
      className="mx-auto my-8 flex max-w-2xl flex-col gap-6 rounded-ui border border-border bg-panel p-8"
    >
      <div className="border-b border-border pb-4">
        <h2 id="welcome-title" className="text-[28px] font-bold text-text">
          {t("welcome.title")}
        </h2>
        <p className="text-text-muted mt-1">{t("welcome.intro")}</p>
      </div>
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
