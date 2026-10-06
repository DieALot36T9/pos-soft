import { ShieldCheck, X } from "lucide-react";
import { useEffect, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { Permission } from "../../generated/Permission";
import type { UserTile } from "../../generated/UserTile";
import { errorMessage } from "../bridge";
import { secondsUntil } from "../format/time";
import { ActionButton } from "../ui/ActionButton";
import { Alert } from "../ui/Alert";
import { Field } from "../ui/Field";
import { SelectField } from "../ui/SelectField";
import { approverTiles, requestOverride } from "./api";

type ApprovalDialogProps = {
  permission: Permission;
  // Gets the single-use token. The caller then runs the action again with it.
  onApproved: (token: string) => void;
  onCancel: () => void;
};

// The admin approval dialog (USR-05, Checkout Screen "Admin approval dialog"): an amber
// header naming the action, the admin, and their password. Rust checks everything.
export function ApprovalDialog({ permission, onApproved, onCancel }: ApprovalDialogProps) {
  const { t } = useTranslation("approval");
  const { t: tPermissions } = useTranslation("permissions");
  const { t: tErrors } = useTranslation("errors");
  const [admins, setAdmins] = useState<UserTile[]>([]);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    approverTiles()
      .then((tiles) => {
        const withPassword = tiles.filter((tile) => tile.has_password);
        setAdmins(withPassword);
        setUsername(withPassword[0]?.username ?? "");
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const approve = () => {
    setProblem(null);
    requestOverride(permission, username, password)
      .then((result) => {
        setPassword("");
        switch (result.kind) {
          case "approved":
            onApproved(result.token);
            break;
          case "wrong_secret":
            setProblem(t("wrong", { left: result.attempts_left }));
            break;
          case "locked":
            setProblem(t("locked", { seconds: secondsUntil(result.until_ms) }));
            break;
          case "not_allowed":
            setProblem(t("notAllowed"));
            break;
        }
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const onEnter = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      approve();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-text/50 p-4">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="approval-title"
        className="flex w-full max-w-md flex-col rounded-ui border border-amber bg-panel"
      >
        <h2
          id="approval-title"
          className="rounded-t-ui border-b border-amber bg-amber-fill p-4 text-xl font-semibold text-text"
        >
          {t("title", { action: tPermissions(permission) })}
        </h2>
        <div className="flex flex-col gap-3 p-4">
          <p className="text-text-muted">{t("intro")}</p>
          {admins.length === 0 ? <Alert kind="warning">{t("noAdmins")}</Alert> : null}
          <SelectField
            label={t("admin.label")}
            tip={t("admin.tip")}
            value={username}
            options={admins.map((tile) => ({ value: tile.username, label: tile.display_name }))}
            onChange={setUsername}
          />
          <Field
            label={t("password.label")}
            tip={t("password.tip")}
            value={password}
            onChange={setPassword}
            onKeyDown={onEnter}
            secret
          />
          {problem ? <Alert kind="problem">{problem}</Alert> : null}
          <div className="flex justify-between">
            <ActionButton
              colour="grey"
              icon={X}
              label={t("cancel.label")}
              tip={t("cancel.tip")}
              onClick={onCancel}
            />
            <ActionButton
              colour="green"
              icon={ShieldCheck}
              label={t("approve.label")}
              tip={t("approve.tip")}
              onClick={approve}
            />
          </div>
        </div>
      </section>
    </div>
  );
}
