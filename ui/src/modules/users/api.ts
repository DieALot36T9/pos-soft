import type { RoleInput } from "../../generated/RoleInput";
import type { RolesView } from "../../generated/RolesView";
import type { UserInput } from "../../generated/UserInput";
import type { UserSummary } from "../../generated/UserSummary";
import { call } from "../../shared/bridge";

// Every user, active or disabled (USR-06).
export const listUsers = () => call<UserSummary[]>("user_list");
// Adds a user (no id) or changes one. Rust checks names, PIN and password, and
// refuses to disable or demote the last active admin.
export const saveUser = (input: UserInput) => call<string>("user_save", { input });
// Every role, and every permission for the checkboxes (USR-04).
export const listRoles = () => call<RolesView>("role_list");
// Adds a custom role (no id) or changes a role's permissions and discount limit.
export const saveRole = (input: RoleInput) => call<string>("role_save", { input });
