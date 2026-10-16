//! Roles and the permissions each role has (USR-04, Roles and Permissions).

use std::collections::BTreeSet;

use pos_core::{Bps, Id, Permission, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AuthError;

/// The id of the system role Admin. It always has every permission and can't be edited.
pub const ADMIN_ROLE_ID: &str = "00000000000000000000000001";
/// The id of the system role Cashier. It starts with the "C" permissions.
pub const CASHIER_ROLE_ID: &str = "00000000000000000000000002";

/// The highest discount limit a role may have: 100%.
const FULL_DISCOUNT: Bps = Bps(10_000);

/// A custom role an admin creates, such as "Supervisor".
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct NewRole {
    pub name: String,
    pub permissions: Vec<Permission>,
    /// The biggest discount this role may give without an admin. Example: Bps(1000) is 10%.
    pub max_discount: Bps,
}

/// The permissions a user has. Sent to the UI as a list, e.g. ["checkout_sell", "stock_view"].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(transparent)]
#[ts(export)]
pub struct PermissionSet(BTreeSet<Permission>);

impl PermissionSet {
    /// Every permission there is. This is what Admin has.
    pub fn all() -> PermissionSet {
        PermissionSet::from_list(&Permission::ALL)
    }

    /// A set holding the given permissions.
    pub fn from_list(permissions: &[Permission]) -> PermissionSet {
        PermissionSet(permissions.iter().copied().collect())
    }

    /// Whether the set allows this permission.
    pub fn contains(&self, permission: Permission) -> bool {
        self.0.contains(&permission)
    }

    /// The permissions in a fixed order (the order of the enum).
    pub fn iter(&self) -> impl Iterator<Item = Permission> + '_ {
        self.0.iter().copied()
    }

    /// How many permissions the set holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when the set allows nothing, as for a disabled user.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One row of the roles screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RoleSummary {
    pub id: Id,
    pub name: String,
    /// Admin and Cashier come with the app and can't be deleted.
    pub is_system: bool,
    /// The Admin role: every permission, and it can't be edited.
    pub is_admin: bool,
    pub max_discount: Bps,
    pub permissions: PermissionSet,
}

/// A role's name and discount limit, read with its permissions.
pub(crate) struct RoleInfo {
    pub name: String,
    pub max_discount: Bps,
    pub permissions: PermissionSet,
}

/// Creates a custom role with the ticked permissions. Returns its new id.
pub fn create_role(
    transaction: &Transaction,
    role: NewRole,
    now: Timestamp,
) -> Result<Id, AuthError> {
    let name = role.name.trim();
    if name.is_empty() {
        return Err(AuthError::RoleNameMissing);
    }
    check_discount(role.max_discount)?;
    let taken: Option<String> = transaction
        .query_row(
            "SELECT id FROM auth_roles WHERE name = :name COLLATE NOCASE",
            named_params! { ":name": name },
            |row| row.get(0),
        )
        .optional()?;
    if taken.is_some() {
        return Err(AuthError::RoleNameInUse);
    }

    let id = Id::new();
    transaction.execute(
        "INSERT INTO auth_roles (id, name, is_system, max_discount_bps, created_at)
         VALUES (:id, :name, 0, :max_discount, :now)",
        named_params! {
            ":id": id.as_str(),
            ":name": name,
            ":max_discount": role.max_discount.0,
            ":now": now.0,
        },
    )?;
    store_permissions(transaction, &id, &role.permissions)?;
    Ok(id)
}

/// Replaces a role's permissions and discount limit. The Admin role can't be changed.
pub fn set_role_permissions(
    transaction: &Transaction,
    role: &Id,
    permissions: &[Permission],
    max_discount: Bps,
) -> Result<(), AuthError> {
    if role.as_str() == ADMIN_ROLE_ID {
        return Err(AuthError::AdminRoleFixed);
    }
    check_discount(max_discount)?;
    let changed = transaction.execute(
        "UPDATE auth_roles SET max_discount_bps = :max_discount WHERE id = :id",
        named_params! { ":max_discount": max_discount.0, ":id": role.as_str() },
    )?;
    if changed == 0 {
        return Err(AuthError::RoleNotFound {
            role: role.to_string(),
        });
    }
    transaction.execute(
        "DELETE FROM auth_role_permissions WHERE role_id = :id",
        named_params! { ":id": role.as_str() },
    )?;
    store_permissions(transaction, role, permissions)
}

/// Every role with its permissions: Admin first, then Cashier, then custom roles by name.
pub fn list_roles(connection: &Connection) -> Result<Vec<RoleSummary>, AuthError> {
    let mut statement = connection.prepare(
        "SELECT id, name, is_system, max_discount_bps FROM auth_roles
         ORDER BY is_system DESC, id = :admin DESC, name COLLATE NOCASE",
    )?;
    let rows = statement.query_map(named_params! { ":admin": ADMIN_ROLE_ID }, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
            row.get::<_, u32>(3)?,
        ))
    })?;
    let mut roles = Vec::new();
    for row in rows {
        let (id, name, is_system, max_discount) = row?;
        let permissions = role_permissions(connection, &id)?;
        roles.push(RoleSummary {
            is_admin: id == ADMIN_ROLE_ID,
            id: Id::parse(&id).map_err(|_| AuthError::RoleNotFound { role: id.clone() })?,
            name,
            is_system,
            max_discount: Bps(max_discount),
            permissions,
        });
    }
    Ok(roles)
}

/// What a user may do. Admin gets every permission; a disabled user gets none.
pub fn permissions_of(connection: &Connection, user: &Id) -> Result<PermissionSet, AuthError> {
    let found: Option<(String, bool)> = connection
        .query_row(
            "SELECT role_id, active FROM auth_users WHERE id = :id",
            named_params! { ":id": user.as_str() },
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((role_id, active)) = found else {
        return Err(AuthError::UserNotFound {
            user: user.to_string(),
        });
    };
    if !active {
        return Ok(PermissionSet::default());
    }
    role_permissions(connection, &role_id)
}

/// Reads a role's name, discount limit and permissions.
pub(crate) fn role_info(connection: &Connection, role_id: &str) -> Result<RoleInfo, AuthError> {
    let found: Option<(String, u32)> = connection
        .query_row(
            "SELECT name, max_discount_bps FROM auth_roles WHERE id = :id",
            named_params! { ":id": role_id },
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((name, max_discount)) = found else {
        return Err(AuthError::RoleNotFound {
            role: role_id.to_string(),
        });
    };
    Ok(RoleInfo {
        name,
        max_discount: Bps(max_discount),
        permissions: role_permissions(connection, role_id)?,
    })
}

/// A role's permissions. Admin's aren't stored: it always has all of them,
/// including permissions added in later versions.
fn role_permissions(connection: &Connection, role_id: &str) -> Result<PermissionSet, AuthError> {
    if role_id == ADMIN_ROLE_ID {
        return Ok(PermissionSet::all());
    }
    let mut statement =
        connection.prepare("SELECT permission FROM auth_role_permissions WHERE role_id = :id")?;
    let rows = statement.query_map(named_params! { ":id": role_id }, |row| {
        row.get::<_, String>(0)
    })?;
    let mut permissions = BTreeSet::new();
    for text in rows {
        // Text this version doesn't know (from a newer version) is skipped.
        if let Some(permission) = Permission::parse(&text?) {
            permissions.insert(permission);
        }
    }
    Ok(PermissionSet(permissions))
}

fn store_permissions(
    transaction: &Transaction,
    role: &Id,
    permissions: &[Permission],
) -> Result<(), AuthError> {
    let permission_set = PermissionSet::from_list(permissions);
    if permission_set.is_empty() {
        return Ok(());
    }

    let mut sql = String::from("INSERT INTO auth_role_permissions (role_id, permission) VALUES ");
    let mut params: Vec<&dyn rusqlite::ToSql> = vec![];
    let role_id_str = role.as_str();

    // We can't push &role_id_str directly into params in the loop if we want to also
    // push something with a shorter lifetime from within the loop or similar,
    // but here we can just push it once and reference it in SQL, or push it multiple times.
    // Actually, it's easier to use position parameters: `(?1, ?2), (?1, ?3)...`
    // SQLite supports parameter reusing by index: `?1` is the first parameter.

    params.push(&role_id_str);

    let permissions_vec: Vec<&str> = permission_set.iter().map(|p| p.as_str()).collect();

    use std::fmt::Write;
    for (i, p) in permissions_vec.iter().enumerate() {
        if i > 0 {
            sql.push_str(", ");
        }
        // role_id is ?1, permission is ?(i + 2)
        let _ = write!(&mut sql, "(?1, ?{})", i + 2);
        params.push(p);
    }

    transaction.execute(&sql, rusqlite::params_from_iter(params))?;
    Ok(())
}

fn check_discount(max_discount: Bps) -> Result<(), AuthError> {
    if max_discount > FULL_DISCOUNT {
        return Err(AuthError::MaxDiscountTooHigh);
    }
    Ok(())
}
