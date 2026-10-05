import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { pickFolder } from "../../../shared/bridge/pickers";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Field } from "../../../shared/ui/Field";
import { saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

const wholeNumber = /^\d+$/;

// Backup folders, how often and how many to keep (SET-11). Rust checks the folders.
export function BackupSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const [folder, setFolder] = useState(saved.backup.primary_dir);
  const [second, setSecond] = useState(saved.backup.secondary_dir ?? "");
  const [interval, setIntervalText] = useState(String(saved.backup.interval_min));
  const [keep, setKeep] = useState(String(saved.backup.keep_last));
  const [numberProblem, setNumberProblem] = useState<string | null>(null);
  const save = useSave();

  const choose = (setter: (folder: string) => void) => {
    pickFolder(t("backup.pickTitle"))
      .then((picked) => {
        if (picked) {
          setter(picked);
        }
      })
      .catch(() => undefined);
  };

  const submit = () => {
    if (!wholeNumber.test(interval.trim()) || !wholeNumber.test(keep.trim())) {
      setNumberProblem(t("backup.numberInvalid"));
      return;
    }
    setNumberProblem(null);
    save.run(
      () =>
        saveSettings({
          section: "backup",
          value: {
            primary_dir: folder.trim(),
            secondary_dir: second.trim() === "" ? null : second.trim(),
            interval_min: Number(interval),
            keep_last: Number(keep),
          },
        }),
      onSaved,
    );
  };

  return (
    <SectionFrame
      title={t("sections.backup")}
      busy={save.busy}
      done={save.done}
      problem={numberProblem ?? save.problem}
      onSave={submit}
    >
      <div className="flex items-end gap-2">
        <Field
          label={t("backup.folder.label")}
          tip={t("backup.folder.tip")}
          value={folder}
          onChange={setFolder}
          error={save.errorFor("primary_dir")}
        />
        <ActionButton
          colour="grey"
          icon={FolderOpen}
          label={t("backup.choose.label")}
          tip={t("backup.choose.tip")}
          onClick={() => choose(setFolder)}
        />
      </div>
      <div className="flex items-end gap-2">
        <Field
          label={t("backup.second.label")}
          tip={t("backup.second.tip")}
          value={second}
          onChange={setSecond}
          error={save.errorFor("secondary_dir")}
        />
        <ActionButton
          colour="grey"
          icon={FolderOpen}
          label={t("backup.choose.label")}
          tip={t("backup.choose.tip")}
          onClick={() => choose(setSecond)}
        />
      </div>
      <div className="grid grid-cols-2 gap-4">
        <Field
          label={t("backup.interval.label")}
          tip={t("backup.interval.tip")}
          value={interval}
          onChange={setIntervalText}
          error={save.errorFor("interval_min")}
          width="short"
        />
        <Field
          label={t("backup.keep.label")}
          tip={t("backup.keep.tip")}
          value={keep}
          onChange={setKeep}
          error={save.errorFor("keep_last")}
          width="short"
        />
      </div>
    </SectionFrame>
  );
}
