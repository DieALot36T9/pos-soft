import { Pencil, ShieldCheck, Users } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { RoleSummary } from "../../../generated/RoleSummary";
import type { RolesView } from "../../../generated/RolesView";
import { errorMessage } from "../../../shared/bridge";
import { formatPercent } from "../../../shared/format/percent";
import { Alert } from "../../../shared/ui/Alert";
import { Badge } from "../../../shared/ui/Badge";
import { IconButton } from "../../../shared/ui/IconButton";
import { Tip } from "../../../shared/ui/Tip";
import { listRoles } from "../api";
import { RoleForm } from "../components/RoleForm";

// The roles list and form: permission checkboxes and the discount limit (USR-04).
export function RolesPage() {
  const { t } = useTranslation("users");
  const { t: tErrors } = useTranslation("errors");
  const [view, setView] = useState<RolesView | null>(null);
  const [editing, setEditing] = useState<RoleSummary | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const load = () => {
    listRoles()
      .then(setView)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- load once when the page opens.
  useEffect(load, []);

  const all = view?.all_permissions.length ?? 0;

  return (
    <section aria-labelledby="roles-heading" className="flex flex-col gap-4">
      <div>
        <h2 id="roles-heading" className="text-[28px] font-semibold">
          {t("roles.title")}
        </h2>
        <p className="text-text-muted">{t("roles.intro")}</p>
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <table className="w-full border-collapse rounded-ui border border-border bg-panel">
        <thead>
          <tr className="border-b border-border text-left">
            <th className="p-2">{t("roles.table.name")}</th>
            <th className="p-2">{t("roles.table.permissions")}</th>
            <th className="p-2 text-right">{t("roles.table.discount")}</th>
            <th className="p-2">{t("roles.table.kind")}</th>
            <th className="p-2">{t("roles.table.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {(view?.roles ?? []).map((role) => (
            <tr key={role.id} className="border-b border-border">
              <td className="p-2">{role.name}</td>
              <td className="p-2 tabular-nums">
                {t("roles.table.count", { count: role.permissions.length, all })}
              </td>
              <td className="p-2 text-right tabular-nums">{formatPercent(role.max_discount)}</td>
              <td className="p-2">
                {role.is_system ? (
                  <Badge
                    colour="blue"
                    icon={ShieldCheck}
                    word={t("roles.table.system")}
                    tip={t("roles.table.systemTip")}
                  />
                ) : (
                  <Badge
                    colour="grey"
                    icon={Users}
                    word={t("roles.table.custom")}
                    tip={t("roles.table.customTip")}
                  />
                )}
              </td>
              <td className="p-2">
                {role.is_admin ? (
                  <Tip tip={t("roles.adminFixed")}>
                    <span tabIndex={0} className="text-text-muted">
                      {t("roles.table.fixed")}
                    </span>
                  </Tip>
                ) : (
                  <IconButton
                    icon={Pencil}
                    colour="grey"
                    tip={t("roles.edit.tip")}
                    onClick={() => setEditing(role)}
                  />
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {view ? (
        <RoleForm
          key={editing?.id ?? "new"}
          role={editing}
          allPermissions={view.all_permissions}
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
