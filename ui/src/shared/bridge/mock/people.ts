import type { AppError } from "../../../generated/AppError";
import type { ApprovalResult } from "../../../generated/ApprovalResult";
import type { AuditFilter } from "../../../generated/AuditFilter";
import type { AuditList } from "../../../generated/AuditList";
import type { AuditPage } from "../../../generated/AuditPage";
import type { AuditRow } from "../../../generated/AuditRow";
import type { LoginResult } from "../../../generated/LoginResult";
import type { Permission } from "../../../generated/Permission";
import type { RecoveryDone } from "../../../generated/RecoveryDone";
import type { RoleInput } from "../../../generated/RoleInput";
import type { RoleSummary } from "../../../generated/RoleSummary";
import type { RolesView } from "../../../generated/RolesView";
import type { SecretCheck } from "../../../generated/SecretCheck";
import type { SecretKind } from "../../../generated/SecretKind";
import type { SessionView } from "../../../generated/SessionView";
import type { UserInput } from "../../../generated/UserInput";
import type { UserSummary } from "../../../generated/UserSummary";
import type { UserTile } from "../../../generated/UserTile";

// Pretend users, roles, the session, admin approvals and the audit log for the fake
// bridge. Rust does the real checks; this stand-in follows the same rules closely
// enough for the screens and their tests, with the same answers and error codes.

const ADMIN_ROLE = "00000000000000000000000001";
const CASHIER_ROLE = "00000000000000000000000002";

export const allPermissions: Permission[] = [
  "checkout_sell",
  "checkout_cancel_cart",
  "checkout_hold_resume",
  "checkout_manual_weight",
  "discount_apply",
  "price_override",
  "price_list_switch",
  "sell_on_credit",
  "credit_limit_exceed",
  "receipt_reprint_last",
  "receipt_reprint_any",
  "sale_void",
  "return_with_receipt",
  "return_without_receipt",
  "shift_open_close_own",
  "shift_cash_in_out",
  "shift_no_sale_drawer",
  "shift_force_close",
  "customer_view_create",
  "customer_manage",
  "customer_receive_payment",
  "loyalty_adjust",
  "stock_view",
  "stock_adjust",
  "stock_count_enter",
  "stock_count_manage",
  "catalog_manage",
  "catalog_price_edit",
  "pricing_manage",
  "tax_manage",
  "purchasing_manage",
  "purchasing_pay",
  "labels_print",
  "reports_view_all",
  "data_import",
  "data_export",
  "users_manage",
  "settings_manage",
  "backup_run",
  "backup_restore",
  "audit_view",
  "logs_manage",
];

const cashierPermissions: Permission[] = [
  "checkout_sell",
  "checkout_cancel_cart",
  "checkout_hold_resume",
  "checkout_manual_weight",
  "discount_apply",
  "sell_on_credit",
  "receipt_reprint_last",
  "shift_open_close_own",
  "customer_view_create",
  "customer_receive_payment",
  "stock_view",
  "stock_count_enter",
  "labels_print",
  "backup_run",
];

type FakeUser = {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  active: boolean;
  pin: string | null;
  password: string | null;
  failed: number;
  locked_until: number | null;
};

let users: FakeUser[] = [];
let roles: RoleSummary[] = [];
let session: { userId: string; locked: boolean } | null = null;
let approvals: { token: string; permission: Permission; until: number }[] = [];
let events: AuditRow[] = [];
let recoveryCode = "7KQ2-M9XD-4RTW-PZ8C-H3NB";

function cashierRole(): RoleSummary {
  return {
    id: CASHIER_ROLE,
    name: "Cashier",
    is_system: true,
    is_admin: false,
    max_discount: 0,
    permissions: [...cashierPermissions],
  };
}

function systemRoles(): RoleSummary[] {
  const admin: RoleSummary = {
    id: ADMIN_ROLE,
    name: "Admin",
    is_system: true,
    is_admin: true,
    max_discount: 10000,
    permissions: [...allPermissions],
  };
  return [admin, cashierRole()];
}

function fakeId(prefix: string, number: number) {
  return `${prefix}${String(number).padStart(26 - prefix.length, "0")}`;
}

// Starts again: no users, or the set-up shop's owner (password) and rina (PIN 1234).
export function resetPeople(setUp: boolean, signedIn: boolean) {
  users = [];
  roles = systemRoles();
  session = null;
  approvals = [];
  events = [];
  recoveryCode = "7KQ2-M9XD-4RTW-PZ8C-H3NB";
  if (setUp) {
    addFakeUser("owner", "Shop Owner", ADMIN_ROLE, "shop-owner-1", null);
    addFakeUser("rina", "Rina Akter", CASHIER_ROLE, null, "1234");
    if (signedIn) {
      session = { userId: users[0]?.id ?? "", locked: false };
    }
  }
}

function addFakeUser(
  username: string,
  displayName: string,
  roleId: string,
  password: string | null,
  pin: string | null,
): FakeUser {
  const user: FakeUser = {
    id: fakeId("01FAKEUSER", users.length + 1),
    username,
    display_name: displayName,
    role_id: roleId,
    active: true,
    pin,
    password,
    failed: 0,
    locked_until: null,
  };
  users.push(user);
  return user;
}

// The setup wizard's admin step: the first admin, signed in straight away.
export function createFirstAdmin(username: string, displayName: string, password: string) {
  const admin = addFakeUser(username, displayName, ADMIN_ROLE, password, null);
  session = { userId: admin.id, locked: false };
  return { recovery_code: recoveryCode, session: currentSession() };
}

function roleOf(user: FakeUser): RoleSummary {
  return roles.find((role) => role.id === user.role_id) ?? cashierRole();
}

function viewOf(user: FakeUser, locked: boolean): SessionView {
  const role = roleOf(user);
  return {
    user_id: user.id,
    username: user.username,
    display_name: user.display_name,
    role_name: role.name,
    is_admin: role.is_admin,
    locked,
    max_discount: role.max_discount,
    permissions: role.permissions,
  };
}

function signedInUser(): FakeUser | null {
  return users.find((user) => user.id === session?.userId && user.active) ?? null;
}

export function currentSession(): SessionView | null {
  const user = signedInUser();
  return user && session ? viewOf(user, session.locked) : null;
}

function fail(error: AppError): never {
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

function conflict(code: string): never {
  return fail({ kind: "conflict", code, params: {} });
}

function refuse(field: string | null, code: string): never {
  return fail({ kind: "validation", field, code, params: {} });
}

// Like Rust's `require`: signed in, not locked, and the role has the permission.
export function requirePermission(permission: Permission) {
  if (session?.locked) {
    conflict("till_locked");
  }
  if (!currentSession()?.permissions.includes(permission)) {
    fail({ kind: "permission_denied", permission });
  }
}

// Like Rust's `require_or_approval`: a fitting approval token lets it run once.
export function requireOrApproval(permission: Permission, approval: unknown) {
  if (currentSession()?.permissions.includes(permission)) {
    return;
  }
  const found = approvals.find((each) => each.token === approval);
  approvals = approvals.filter((each) => each.token !== approval);
  if (found?.permission === permission && found.until > Date.now()) {
    return;
  }
  fail({ kind: "override_required", permission });
}

function record(action: string, userId: string | null, approvedBy: string | null = null) {
  events.unshift({
    id: fakeId("01FAKEEVENT", events.length + 1),
    at: Date.now(),
    business_date: "2026-10-05",
    till_id: "01FAKETILL0000000000000001",
    user_id: userId,
    approved_by: approvedBy,
    action,
    entity_type: null,
    entity_id: null,
    details: {},
  });
}

type Checked = { kind: "ok" } | { kind: "wrong"; left: number } | { kind: "locked"; until: number };

// The lockout rule: 5 wrong tries in a row lock the user for 60 seconds (USR-02).
function check(user: FakeUser, typed: unknown, secrets: (string | null)[]): Checked {
  if (user.locked_until !== null && user.locked_until > Date.now()) {
    return { kind: "locked", until: user.locked_until };
  }
  if (secrets.some((secret) => secret !== null && secret === typed)) {
    user.failed = 0;
    user.locked_until = null;
    return { kind: "ok" };
  }
  user.failed += 1;
  if (user.failed >= 5) {
    user.failed = 0;
    user.locked_until = Date.now() + 60_000;
    record("user.lockout", user.id);
    return { kind: "locked", until: user.locked_until };
  }
  return { kind: "wrong", left: 5 - user.failed };
}

function tiles(): UserTile[] {
  return users
    .filter((user) => user.active)
    .sort((a, b) => a.display_name.localeCompare(b.display_name))
    .map((user) => ({
      id: user.id,
      username: user.username,
      display_name: user.display_name,
      has_pin: user.pin !== null,
      has_password: user.password !== null,
    }));
}

function byUsername(username: unknown): FakeUser | undefined {
  const wanted = String(username).trim().toLowerCase();
  return users.find((user) => user.active && user.username.toLowerCase() === wanted);
}

function summary(user: FakeUser): UserSummary {
  const role = roleOf(user);
  return {
    id: user.id,
    username: user.username,
    display_name: user.display_name,
    role_id: user.role_id,
    role_name: role.name,
    is_admin: role.is_admin,
    active: user.active,
    has_pin: user.pin !== null,
    has_password: user.password !== null,
    locked_until: user.locked_until,
  };
}

function checkSecrets(pin: string | null, password: string | null) {
  if (pin !== null && !/^\d{4,8}$/.test(pin)) {
    refuse("pin", "pin_format");
  }
  if (password !== null && password.length < 8) {
    refuse("password", "password_too_short");
  }
}

function typed(text: string | null): string | null {
  return text === null || text === "" ? null : text;
}

function saveUser(input: UserInput): string {
  const pin = typed(input.pin);
  const password = typed(input.password);
  checkSecrets(pin, password);
  if (input.display_name.trim() === "") {
    refuse("display_name", "display_name_missing");
  }
  const by = session?.userId ?? null;
  if (input.id === null) {
    if (input.username.trim() === "") {
      refuse("username", "username_missing");
    }
    if (pin === null && password === null) {
      refuse(null, "secret_missing");
    }
    if (users.some((user) => user.username.toLowerCase() === input.username.trim().toLowerCase())) {
      conflict("username_in_use");
    }
    const user = addFakeUser(
      input.username.trim(),
      input.display_name.trim(),
      input.role_id,
      password,
      pin,
    );
    user.active = input.active;
    record("user.create", by);
    return user.id;
  }
  const user = users.find((each) => each.id === input.id);
  if (!user) {
    return fail({ kind: "not_found", entity: "user", key: input.id });
  }
  const otherAdmins = users.filter(
    (each) => each.id !== user.id && each.active && each.role_id === ADMIN_ROLE,
  );
  const staysAdmin = input.active && input.role_id === ADMIN_ROLE;
  if (user.active && user.role_id === ADMIN_ROLE && !staysAdmin && otherAdmins.length === 0) {
    conflict("last_active_admin");
  }
  user.display_name = input.display_name.trim();
  user.role_id = input.role_id;
  user.active = input.active;
  if (pin !== null) {
    user.pin = pin;
  }
  if (password !== null) {
    user.password = password;
  }
  record("user.change", by);
  return user.id;
}

function saveRole(input: RoleInput): string {
  if (input.max_discount > 10000) {
    refuse("max_discount", "max_discount_too_high");
  }
  const by = session?.userId ?? null;
  if (input.id === null) {
    const name = input.name.trim();
    if (name === "") {
      refuse("name", "role_name_missing");
    }
    if (roles.some((role) => role.name.toLowerCase() === name.toLowerCase())) {
      conflict("role_name_in_use");
    }
    const role: RoleSummary = {
      id: fakeId("01FAKEROLE", roles.length + 1),
      name,
      is_system: false,
      is_admin: false,
      max_discount: input.max_discount,
      permissions: allPermissions.filter((each) => input.permissions.includes(each)),
    };
    roles.push(role);
    record("role.create", by);
    return role.id;
  }
  if (input.id === ADMIN_ROLE) {
    conflict("admin_role_fixed");
  }
  roles = roles.map((role) =>
    role.id === input.id
      ? {
          ...role,
          max_discount: input.max_discount,
          permissions: allPermissions.filter((each) => input.permissions.includes(each)),
        }
      : role,
  );
  record("role.change", by);
  return input.id;
}

function filtered(filter: AuditFilter): AuditRow[] {
  return events.filter(
    (event) =>
      (filter.user_id === null || event.user_id === filter.user_id) &&
      (filter.action === null || event.action === filter.action) &&
      (filter.from === null || event.business_date >= filter.from) &&
      (filter.to === null || event.business_date <= filter.to),
  );
}

const auditActions = [
  "cart.cancel",
  "sale.void",
  "drawer.no_sale",
  "settings.change",
  "tax_rate.change",
  "backup.manual",
  "backup.restore",
  "logging.switch",
  "session.login",
  "session.logout",
  "session.unlock_by_admin",
  "user.lockout",
  "approval.granted",
  "user.create",
  "user.change",
  "user.own_secret_change",
  "role.create",
  "role.change",
  "recovery.used",
];

type Args = Record<string, unknown> | undefined;

export const peopleHandlers = {
  session_current: (): SessionView | null => currentSession(),
  login_tiles: (): UserTile[] => tiles(),
  login: (args?: Args): LoginResult => {
    const user = users.find((each) => each.id === args?.userId && each.active);
    if (!user) {
      return conflict("user_inactive");
    }
    if (session?.locked && session.userId !== user.id) {
      conflict("till_locked");
    }
    const result = check(user, args?.secret, [user.pin, user.password]);
    if (result.kind === "wrong") {
      return { kind: "wrong_secret", attempts_left: result.left };
    }
    if (result.kind === "locked") {
      return { kind: "locked", until_ms: result.until };
    }
    session = { userId: user.id, locked: false };
    approvals = [];
    record("session.login", user.id);
    return { kind: "logged_in", session: viewOf(user, false) };
  },
  logout: (): null => {
    if (session) {
      record("session.logout", session.userId);
    }
    session = null;
    approvals = [];
    return null;
  },
  lock: (): SessionView => {
    if (!session) {
      conflict("not_signed_in");
    }
    session.locked = true;
    approvals = [];
    return currentSession() as SessionView;
  },
  unlock: (args?: Args): SecretCheck => {
    const user = signedInUser();
    if (!user || !session?.locked) {
      return conflict("till_not_locked");
    }
    const result = check(user, args?.secret, [user.pin, user.password]);
    if (result.kind === "wrong") {
      return { kind: "wrong_secret", attempts_left: result.left };
    }
    if (result.kind === "locked") {
      return { kind: "locked", until_ms: result.until };
    }
    session.locked = false;
    return { kind: "accepted" };
  },
  unlock_by_admin: (args?: Args): SecretCheck => {
    if (!session?.locked) {
      return conflict("till_not_locked");
    }
    const admin = byUsername(args?.username);
    if (!admin) {
      return refuse(null, "override_user_unknown");
    }
    const result = check(admin, args?.password, [admin.password]);
    if (result.kind === "wrong") {
      return { kind: "wrong_secret", attempts_left: result.left };
    }
    if (result.kind === "locked") {
      return { kind: "locked", until_ms: result.until };
    }
    if (!roleOf(admin).permissions.includes("users_manage")) {
      return { kind: "not_allowed" };
    }
    session.locked = false;
    record("session.unlock_by_admin", session.userId, admin.id);
    return { kind: "accepted" };
  },
  request_override: (args?: Args): ApprovalResult => {
    if (!session || session.locked) {
      return conflict("not_signed_in");
    }
    const admin = byUsername(args?.username);
    if (!admin) {
      return refuse(null, "override_user_unknown");
    }
    const result = check(admin, args?.password, [admin.password]);
    if (result.kind === "wrong") {
      return { kind: "wrong_secret", attempts_left: result.left };
    }
    if (result.kind === "locked") {
      return { kind: "locked", until_ms: result.until };
    }
    const permission = args?.permission as Permission;
    if (!roleOf(admin).permissions.includes(permission)) {
      return { kind: "not_allowed" };
    }
    const token = `fake-token-${String(approvals.length + 1)}-${String(Date.now())}`;
    approvals.push({ token, permission, until: Date.now() + 60_000 });
    record("approval.granted", session.userId, admin.id);
    return { kind: "approved", token };
  },
  change_own_secret: (args?: Args): SecretCheck => {
    const user = signedInUser();
    if (!user || session?.locked) {
      return conflict("not_signed_in");
    }
    const result = check(user, args?.current, [user.pin, user.password]);
    if (result.kind === "wrong") {
      return { kind: "wrong_secret", attempts_left: result.left };
    }
    if (result.kind === "locked") {
      return { kind: "locked", until_ms: result.until };
    }
    const kind = args?.kind as SecretKind;
    const next = typeof args?.newSecret === "string" ? args.newSecret : "";
    checkSecrets(kind === "pin" ? next : null, kind === "password" ? next : null);
    if (kind === "pin") {
      user.pin = next;
    } else {
      user.password = next;
    }
    record("user.own_secret_change", user.id);
    return { kind: "accepted" };
  },
  recover_admin: (args?: Args): RecoveryDone => {
    if (args?.password !== args?.passwordAgain) {
      refuse("password_again", "passwords_differ");
    }
    const admin = byUsername(args?.username);
    if (!admin || admin.role_id !== ADMIN_ROLE) {
      return refuse("username", "recovery_user_not_admin");
    }
    const clean = (code: unknown) =>
      String(code)
        .toUpperCase()
        .replace(/[^0-9A-Z]/g, "");
    if (clean(args?.code) !== clean(recoveryCode)) {
      refuse("code", "recovery_code_wrong");
    }
    const password = String(args?.password);
    checkSecrets(null, password);
    admin.password = password;
    admin.failed = 0;
    admin.locked_until = null;
    recoveryCode = "H3NB-PZ8C-4RTW-M9XD-7KQ2";
    record("recovery.used", admin.id);
    return { recovery_code: recoveryCode };
  },
  user_list: (): UserSummary[] => {
    requirePermission("users_manage");
    return [...users].sort((a, b) => a.display_name.localeCompare(b.display_name)).map(summary);
  },
  user_save: (args?: Args): string => {
    requirePermission("users_manage");
    return saveUser(args?.input as UserInput);
  },
  role_list: (): RolesView => {
    requirePermission("users_manage");
    return { roles, all_permissions: [...allPermissions] };
  },
  role_save: (args?: Args): string => {
    requirePermission("users_manage");
    return saveRole(args?.input as RoleInput);
  },
  audit_query: (args?: Args): AuditList => {
    requirePermission("audit_view");
    const page = args?.page as AuditPage;
    const rows = filtered(args?.filter as AuditFilter).slice(page.offset, page.offset + page.limit);
    return {
      rows,
      people: users.map((user) => ({
        id: user.id,
        username: user.username,
        display_name: user.display_name,
      })),
      actions: auditActions,
    };
  },
  audit_export: (args?: Args): number => {
    requirePermission("audit_view");
    return filtered(args?.filter as AuditFilter).length;
  },
};

// Lets shop.ts record its own events, such as "Back up now".
export { record as recordFakeEvent };

// Tests start signed in as someone else, such as the cashier "rina".
export function signInFakeAs(username: string) {
  const user = byUsername(username);
  session = user ? { userId: user.id, locked: false } : null;
}
