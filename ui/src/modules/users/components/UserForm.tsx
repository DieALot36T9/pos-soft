import { Save, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { RoleSummary } from "../../../generated/RoleSummary";
import type { UserSummary } from "../../../generated/UserSummary";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveUser } from "../api";

type UserFormProps = {
  // The user being changed, or null to add one.
  user: UserSummary | null;
  roles: RoleSummary[];
  onSaved: () => void;
  onCancel: () => void;
};

// Adds or changes one user (USR-06). A PIN or password typed here resets it.
export function UserForm({ user, roles, onSaved, onCancel }: UserFormProps) {
  const { t } = useTranslation("users");
  const cashier = roles.find((role) => role.is_system && !role.is_admin);
  const [username, setUsername] = useState(user?.username ?? "");
  const [displayName, setDisplayName] = useState(user?.display_name ?? "");
  const [roleId, setRoleId] = useState(user?.role_id ?? cashier?.id ?? roles[0]?.id ?? "");
  const [active, setActive] = useState(user?.active ?? true);
  const [pin, setPin] = useState("");
  const [password, setPassword] = useState("");
  const save = useSave();

  const submit = () =>
    save.run(
      () =>
        saveUser({
          id: user?.id ?? null,
          username,
          display_name: displayName,
          role_id: roleId,
          active,
          pin: pin === "" ? null : pin,
          password: password === "" ? null : password,
        }),
      () => {
        setPin("");
        setPassword("");
        if (!user) {
          setUsername("");
          setDisplayName("");
        }
        onSaved();
      },
    );

  const fieldProblem = ["username", "display_name", "pin", "password"].some(save.errorFor);

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {user ? t("userForm.editHeading", { name: user.display_name }) : t("userForm.addHeading")}
      </h3>
      <div className="grid grid-cols-2 gap-3">
        {user ? (
          <div className="flex flex-col gap-1">
            <span className="font-semibold">{t("userForm.username.label")}</span>
            <span className="min-h-10 py-2">{user.username}</span>
            <span className="text-sm text-text-muted">{t("userForm.username.fixed")}</span>
          </div>
        ) : (
          <Field
            label={t("userForm.username.label")}
            tip={t("userForm.username.tip")}
            value={username}
            onChange={setUsername}
            error={save.errorFor("username")}
          />
        )}
        <Field
          label={t("userForm.displayName.label")}
          tip={t("userForm.displayName.tip")}
          value={displayName}
          onChange={setDisplayName}
          error={save.errorFor("display_name")}
        />
        <SelectField
          label={t("userForm.role.label")}
          tip={t("userForm.role.tip")}
          value={roleId}
          options={roles.map((role) => ({ value: role.id, label: role.name }))}
          onChange={setRoleId}
        />
        <div className="flex items-end pb-2">
          <CheckBox
            label={t("userForm.active.label")}
            tip={t("userForm.active.tip")}
            checked={active}
            onChange={setActive}
          />
        </div>
        <Field
          label={user ? t("userForm.pin.label") : t("userForm.newPin.label")}
          tip={user ? t("userForm.pin.tip") : t("userForm.newPin.tip")}
          value={pin}
          onChange={setPin}
          error={save.errorFor("pin")}
          secret
        />
        <Field
          label={user ? t("userForm.password.label") : t("userForm.newPassword.label")}
          tip={user ? t("userForm.password.tip") : t("userForm.newPassword.tip")}
          value={password}
          onChange={setPassword}
          error={save.errorFor("password")}
          secret
        />
      </div>
      {save.problem && !fieldProblem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("userForm.saved")}</Alert> : null}
      <div className="flex justify-between">
        {user ? (
          <ActionButton
            colour="grey"
            icon={X}
            label={t("userForm.cancel.label")}
            tip={t("userForm.cancel.tip")}
            onClick={onCancel}
          />
        ) : (
          <span />
        )}
        <ActionButton
          colour="green"
          icon={Save}
          label={t("userForm.save.label")}
          tip={t("userForm.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
