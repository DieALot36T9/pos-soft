//! Users and roles screens, changing your own PIN or password, and the recovery
//! code (USR-04, USR-06, USR-07, USR-09).

mod common;

use app::audit_actions::{
    RECOVERY_USED, ROLE_CHANGE, ROLE_CREATE, USER_CHANGE, USER_CREATE, USER_OWN_SECRET_CHANGE,
};
use app::commands::{auth as auth_commands, backup as backup_commands, users as user_commands};
use app::session::{self, LoginResult, SecretCheck};
use app::workflows::users::{RoleInput, UserInput};
use auth::{ADMIN_ROLE_ID, CASHIER_ROLE_ID, SecretKind};
use common::*;
use pos_core::{AppError, Bps, Id, Permission, Secret};

fn new_cashier(pin: &str) -> UserInput {
    UserInput {
        id: None,
        username: "rina".into(),
        display_name: "Rina Akter".into(),
        role_id: Id::parse(CASHIER_ROLE_ID).expect("id"),
        active: true,
        pin: Some(Secret::new(pin.into())),
        password: Some(Secret::new(String::new())),
    }
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. } | AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn usr_06_an_admin_adds_a_cashier_who_then_logs_in() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);

    let rina = user_commands::user_save(&state, new_cashier("1234")).expect("saved");

    let users = user_commands::user_list(&state).expect("list");
    let row = users.iter().find(|user| user.id == rina).expect("listed");
    assert_eq!(row.display_name, "Rina Akter");
    assert!(row.has_pin && !row.has_password);
    let (by, _, details) = last_audit(&state, USER_CREATE);
    assert_eq!(by, Some(user_id(&state, "owner").to_string()));
    assert!(!details.contains("1234"), "{details}");
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
}

#[test]
fn usr_06_an_admin_resets_a_pin_and_disables_a_user() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let rina = user_commands::user_save(&state, new_cashier("1234")).expect("saved");

    let reset = UserInput {
        id: Some(rina.clone()),
        pin: Some(Secret::new("5678".into())),
        ..new_cashier("")
    };
    user_commands::user_save(&state, reset).expect("reset");
    let (_, _, details) = last_audit(&state, USER_CHANGE);
    assert!(details.contains(r#""pin_reset":true"#), "{details}");
    assert!(details.contains(r#""password_reset":false"#), "{details}");
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "5678");
    session::logout(&state).expect("out");
    sign_in(&state, "owner", ADMIN_PASSWORD);

    let off = UserInput {
        id: Some(rina.clone()),
        active: false,
        ..new_cashier("")
    };
    user_commands::user_save(&state, off).expect("disabled");
    let tiles = auth_commands::login_tiles(&state).expect("tiles");
    assert!(tiles.iter().all(|tile| tile.id != rina));
    let users = user_commands::user_list(&state).expect("list");
    assert!(users.iter().any(|user| user.id == rina && !user.active));
}

#[test]
fn usr_06_the_last_active_admin_cant_be_disabled_and_nothing_is_audited() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let owner = user_id(&state, "owner");
    let before = audit_count(&state, USER_CHANGE);

    let off = UserInput {
        id: Some(owner),
        username: "owner".into(),
        display_name: "Shop Owner".into(),
        role_id: Id::parse(ADMIN_ROLE_ID).expect("id"),
        active: false,
        pin: None,
        password: None,
    };

    assert_eq!(
        code_of(user_commands::user_save(&state, off)),
        "last_active_admin"
    );
    assert_eq!(audit_count(&state, USER_CHANGE), before);
}

#[test]
fn usr_06_a_bad_pin_saves_nothing() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);

    assert_eq!(
        code_of(user_commands::user_save(&state, new_cashier("12"))),
        "pin_format"
    );
    assert_eq!(user_commands::user_list(&state).expect("list").len(), 1);
    assert_eq!(audit_count(&state, USER_CREATE), 0);
}

#[test]
fn usr_04_an_admin_adds_and_changes_a_custom_role() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);

    let supervisor = RoleInput {
        id: None,
        name: "Supervisor".into(),
        permissions: vec![Permission::CheckoutSell, Permission::SaleVoid],
        max_discount: Bps(1000),
    };
    let id = user_commands::role_save(&state, supervisor.clone()).expect("added");
    let view = user_commands::role_list(&state).expect("roles");
    assert_eq!(view.all_permissions.len(), Permission::ALL.len());
    let role = view
        .roles
        .iter()
        .find(|role| role.id == id)
        .expect("listed");
    assert_eq!(role.max_discount, Bps(1000));
    assert!(role.permissions.contains(Permission::SaleVoid));
    let (_, _, details) = last_audit(&state, ROLE_CREATE);
    assert!(details.contains("sale_void"), "{details}");

    let changed = RoleInput {
        id: Some(id.clone()),
        permissions: vec![Permission::CheckoutSell],
        max_discount: Bps(500),
        ..supervisor
    };
    user_commands::role_save(&state, changed).expect("changed");
    let view = user_commands::role_list(&state).expect("roles");
    let role = view
        .roles
        .iter()
        .find(|role| role.id == id)
        .expect("listed");
    assert!(!role.permissions.contains(Permission::SaleVoid));
    assert_eq!(audit_count(&state, ROLE_CHANGE), 1);
}

#[test]
fn usr_04_the_admin_role_cant_be_changed() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let admin = RoleInput {
        id: Some(Id::parse(ADMIN_ROLE_ID).expect("id")),
        name: "Admin".into(),
        permissions: vec![],
        max_discount: Bps(0),
    };

    assert_eq!(
        code_of(user_commands::role_save(&state, admin)),
        "admin_role_fixed"
    );
}

#[test]
fn usr_04_a_role_change_counts_for_a_signed_in_user_at_once() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    assert!(backup_commands::backup_list(&state).is_ok());

    state
        .db
        .write(|tx| {
            let cashier = Id::parse(CASHIER_ROLE_ID)?;
            Ok(auth::set_role_permissions(
                tx,
                &cashier,
                &[Permission::CheckoutSell],
                Bps(0),
            )?)
        })
        .expect("changed");

    assert!(matches!(
        backup_commands::backup_list(&state),
        Err(AppError::PermissionDenied { .. })
    ));
}

#[test]
fn usr_07_a_user_changes_their_own_pin_after_typing_the_current_one() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");

    let wrong =
        auth_commands::change_own_secret(&state, SecretKind::Pin, "0000".into(), "4321".into())
            .expect("runs");
    assert_eq!(wrong, SecretCheck::WrongSecret { attempts_left: 4 });
    let right =
        auth_commands::change_own_secret(&state, SecretKind::Pin, "1234".into(), "4321".into())
            .expect("runs");
    assert_eq!(right, SecretCheck::Accepted);
    let (_, _, details) = last_audit(&state, USER_OWN_SECRET_CHANGE);
    assert_eq!(details, r#"{"kind":"pin"}"#);

    session::logout(&state).expect("out");
    sign_in(&state, "rina", "4321");
    assert!(matches!(
        auth_commands::change_own_secret(&state, SecretKind::Pin, "4321".into(), "12".into()),
        Err(AppError::Validation { code, .. }) if code == "pin_format"
    ));
}

#[test]
fn usr_09_the_recovery_code_sets_a_new_admin_password_and_is_replaced() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    let code = finish_setup(&state);
    session::logout(&state).expect("out");

    let wrong = auth_commands::recover_admin(
        &state,
        "AAAA-AAAA-AAAA-AAAA-AAAA".into(),
        "owner",
        "new-owner-pass".into(),
        "new-owner-pass",
    );
    assert_eq!(code_of(wrong), "recovery_code_wrong");
    let differ = auth_commands::recover_admin(
        &state,
        code.clone(),
        "owner",
        "new-owner-pass".into(),
        "other-pass-1",
    );
    assert_eq!(code_of(differ), "passwords_differ");

    let done = auth_commands::recover_admin(
        &state,
        code.to_lowercase(),
        "Owner",
        "new-owner-pass".into(),
        "new-owner-pass",
    )
    .expect("recovered");
    assert_ne!(done.recovery_code, code);
    let (user, _, _) = last_audit(&state, RECOVERY_USED);
    assert_eq!(user, Some(user_id(&state, "owner").to_string()));
    sign_in(&state, "owner", "new-owner-pass");
    session::logout(&state).expect("out");

    let old_code_again = auth_commands::recover_admin(
        &state,
        code,
        "owner",
        "third-password".into(),
        "third-password",
    );
    assert_eq!(code_of(old_code_again), "recovery_code_wrong");
}

#[test]
fn usr_09_recovery_is_only_for_an_admin() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    let code = finish_setup(&state);
    add_cashier(&state);
    session::logout(&state).expect("out");

    for username in ["rina", "nobody"] {
        let result = auth_commands::recover_admin(
            &state,
            code.clone(),
            username,
            "new-password".into(),
            "new-password",
        );
        assert_eq!(code_of(result), "recovery_user_not_admin", "{username}");
    }
    // A refused try changes nothing: the owner's password still works.
    let owner = user_id(&state, "owner");
    assert!(matches!(
        session::login(&state, &owner, ADMIN_PASSWORD.into()).expect("runs"),
        LoginResult::LoggedIn { .. }
    ));
}
