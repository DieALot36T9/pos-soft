import { Save, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { Permission } from "../../../generated/Permission";
import type { RoleSummary } from "../../../generated/RoleSummary";
import { useSave } from "../../../shared/bridge/useSave";
import { formatPercent, parsePercent } from "../../../shared/format/percent";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { saveRole } from "../api";
import { byArea } from "./permissionAreas";

type RoleFormProps = {
  // The role being changed, or null to add one. Never the Admin role.
  role: RoleSummary | null;
  allPermissions: Permission[];
  onSaved: () => void;
  onCancel: () => void;
};

// Adds a custom role or changes a role's permissions and discount limit (USR-04).
export function RoleForm({ role, allPermissions, onSaved, onCancel }: RoleFormProps) {
  const { t } = useTranslation("users");
  const { t: tPermissions } = useTranslation("permissions");
  const [name, setName] = useState(role?.name ?? "");
  const [discount, setDiscount] = useState(
    role ? formatPercent(role.max_discount).replace("%", "") : "0",
  );
  const [ticked, setTicked] = useState<Permission[]>(role?.permissions ?? []);
  const [discountProblem, setDiscountProblem] = useState<string | null>(null);
  const save = useSave();

  const toggle = (permission: Permission, on: boolean) =>
    setTicked((now) => (on ? [...now, permission] : now.filter((each) => each !== permission)));

  const submit = () => {
    const parsed = parsePercent(discount);
    if (!parsed.ok) {
      setDiscountProblem(t("roleForm.discount.invalid"));
      return;
    }
    setDiscountProblem(null);
    save.run(
      () =>
        saveRole({
          id: role?.id ?? null,
          name,
          permissions: ticked,
          max_discount: parsed.bps,
        }),
      () => {
        if (!role) {
          setName("");
          setDiscount("0");
          setTicked([]);
        }
        onSaved();
      },
    );
  };

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">
        {role ? t("roleForm.editHeading", { name: role.name }) : t("roleForm.addHeading")}
      </h3>
      <div className="flex flex-wrap items-end gap-3">
        {role ? (
          <p className="text-text-muted">{t("roleForm.name.fixed")}</p>
        ) : (
          <Field
            label={t("roleForm.name.label")}
            tip={t("roleForm.name.tip")}
            value={name}
            onChange={setName}
            error={save.errorFor("name")}
          />
        )}
        <Field
          label={t("roleForm.discount.label")}
          tip={t("roleForm.discount.tip")}
          value={discount}
          onChange={setDiscount}
          error={discountProblem ?? save.errorFor("max_discount")}
          width="short"
        />
      </div>
      <div className="grid grid-cols-2 gap-4">
        {byArea(allPermissions).map((group) => (
          <fieldset
            key={group.area}
            className="flex flex-col gap-1 rounded-ui border border-border p-3"
          >
            <legend className="px-1 font-semibold">{t(`areas.${group.area}`)}</legend>
            {group.permissions.map((permission) => (
              <CheckBox
                key={permission}
                label={tPermissions(permission)}
                tip={t("roleForm.permissionTip")}
                checked={ticked.includes(permission)}
                onChange={(on) => toggle(permission, on)}
              />
            ))}
          </fieldset>
        ))}
      </div>
      {save.problem && !save.errorFor("name") && !save.errorFor("max_discount") ? (
        <Alert kind="problem">{save.problem}</Alert>
      ) : null}
      {save.done ? <Alert kind="done">{t("roleForm.saved")}</Alert> : null}
      <div className="flex justify-between">
        {role ? (
          <ActionButton
            colour="grey"
            icon={X}
            label={t("roleForm.cancel.label")}
            tip={t("roleForm.cancel.tip")}
            onClick={onCancel}
          />
        ) : (
          <span />
        )}
        <ActionButton
          colour="green"
          icon={Save}
          label={t("roleForm.save.label")}
          tip={t("roleForm.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
