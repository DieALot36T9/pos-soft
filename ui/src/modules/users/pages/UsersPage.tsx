import { CheckCircle, Lock, Pencil, PauseCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { RoleSummary } from "../../../generated/RoleSummary";
import type { UserSummary } from "../../../generated/UserSummary";
import { errorMessage } from "../../../shared/bridge";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { IconButton } from "../../../shared/ui/IconButton";
import { listRoles, listUsers } from "../api";
import { UserForm } from "../components/UserForm";

export function UsersPage() {
  const { t } = useTranslation("users");
  const { t: tErrors } = useTranslation("errors");
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [roles, setRoles] = useState<RoleSummary[] | null>(null);
  const [editing, setEditing] = useState<UserSummary | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [loadedAt, setLoadedAt] = useState(0);

  const load = () => {
    Promise.all([listUsers(), listRoles()])
      .then(([found, view]) => {
        setUsers(found);
        setRoles(view.roles);
        setLoadedAt(Date.now());
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the page opens.
  useEffect(load, []);

  const logsInWith = (user: UserSummary) => {
    if (user.has_pin && user.has_password) {
      return t("users.table.pinAndPassword");
    }
    return user.has_pin ? t("users.table.pin") : t("users.table.password");
  };

  return (
    <section aria-labelledby="users-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="users-heading" className="text-[28px] font-semibold">
          {t("users.title")}
        </h2>
        <p className="text-text-muted">{t("users.intro")}</p>
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <table className="w-full border-collapse rounded-ui border border-border bg-panel">
        <thead>
          <tr className="border-b border-border bg-bg text-left text-sm font-bold">
            <th className="p-2.5">{t("users.table.name")}</th>
            <th className="p-2.5">{t("users.table.username")}</th>
            <th className="p-2.5">{t("users.table.role")}</th>
            <th className="p-2.5">{t("users.table.logsInWith")}</th>
            <th className="p-2.5">{t("users.table.status")}</th>
            <th className="p-2.5">{t("users.table.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {users.map((user) => (
            <tr key={user.id} className="border-b border-border hover:bg-blue-soft/30">
              <td className="p-2.5 font-semibold">{user.display_name}</td>
              <td className="p-2.5 font-mono text-sm text-text-muted">{user.username}</td>
              <td className="p-2.5 font-medium">{user.role_name}</td>
              <td className="p-2.5">{logsInWith(user)}</td>
              <td className="flex flex-wrap gap-2 p-2.5">
                {user.active ? (
                  <Badge
                    colour="green"
                    icon={CheckCircle}
                    word={t("users.table.active")}
                    tip={t("users.table.activeTip")}
                  />
                ) : (
                  <Badge
                    colour="amber"
                    icon={PauseCircle}
                    word={t("users.table.disabled")}
                    tip={t("users.table.disabledTip")}
                  />
                )}
                {user.locked_until !== null && user.locked_until > loadedAt ? (
                  <Badge
                    colour="red"
                    icon={Lock}
                    word={t("users.table.locked")}
                    tip={t("users.table.lockedTip")}
                  />
                ) : null}
              </td>
              <td className="p-2.5">
                <IconButton
                  icon={Pencil}
                  colour="grey"
                  tip={t("users.edit.tip")}
                  onClick={() => setEditing(user)}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {roles ? (
        <UserForm
          key={editing?.id ?? "new"}
          user={editing}
          roles={roles}
          onSaved={() => {
            setEditing(null);
            load();
          }}
          onCancel={() => setEditing(null)}
        />
      ) : null}
    </section>
  );
}
