//! Backups, restore and the startup check (rules BAK-01 to BAK-06).

mod common;

use app::commands::backup as backup_commands;
use app::commands::startup::{recovery_backups, recovery_restore, startup_status};
use app::state::DbHealth;
use app::workflows::backup;
use app::workflows::settings_change::SettingsChange;
use common::*;
use pos_core::AppError;
use settings::{BackupSettings, Setting, ShopProfile};

fn shop_name(state: &app::state::AppState) -> String {
    let shop: ShopProfile = state
        .db
        .read(|conn| Ok(settings::get(conn)?))
        .expect("read");
    shop.name
}

fn rename_shop(state: &app::state::AppState, name: &str) {
    app::commands::settings::settings_save(state, &SettingsChange::Shop(shop(name)))
        .expect("renamed");
}

#[test]
fn bak_04_back_up_now_makes_a_checked_backup_that_the_list_shows() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);

    let done = backup_commands::backup_now(&state).expect("backed up");

    assert!(done.warning.is_none());
    assert!(std::path::Path::new(&done.file).exists());
    let rows = backup_commands::backup_list(&state).expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reason, "manual");
    assert!(rows[0].usable);
    assert_eq!(rows[0].sale_count, 0);
    assert_eq!(rows[0].taken_at, "2026-10-05 14:00:00");
    assert_eq!(audit_count(&state, "backup.manual"), 1);
}

#[test]
fn bak_04_backups_need_a_signed_in_user_with_permission() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    app::session::sign_out(&state).expect("out");

    assert!(matches!(
        backup_commands::backup_now(&state),
        Err(AppError::PermissionDenied { .. })
    ));
    assert!(matches!(
        backup_commands::backup_restore(&state, "x.sqlite"),
        Err(AppError::PermissionDenied { .. })
    ));
}

#[test]
fn bak_01_the_timer_backs_up_only_when_something_changed() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    backup::make_backup(&state, "auto").expect("first");

    assert!(
        backup::backup_if_changed(&state)
            .expect("checked")
            .is_none()
    );

    rename_shop(&state, "New Name");
    assert!(
        backup::backup_if_changed(&state)
            .expect("checked")
            .is_some()
    );
}

#[test]
fn bak_02_a_broken_second_folder_warns_but_the_backup_is_made() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let blocker = folder.path().join("usb-file");
    std::fs::write(&blocker, b"x").expect("file");
    // Saved straight to the database: Settings would refuse this folder.
    let bad = BackupSettings {
        secondary_dir: Some(blocker.join("copies").to_string_lossy().into_owned()),
        ..BackupSettings::default_value()
    };
    state
        .db
        .write(|tx| Ok(settings::set(tx, &bad, None, pos_core::Timestamp(0)).map(|_| ())?))
        .expect("saved");

    let done = backup::make_backup(&state, "auto").expect("made anyway");

    assert_eq!(done.warning.as_deref(), Some("backup_second_folder_failed"));
    assert!(std::path::Path::new(&done.file).exists());
    let view = startup_status(&state).expect("status");
    assert_eq!(
        view.backup_warning.as_deref(),
        Some("backup_second_folder_failed")
    );
}

#[test]
fn bak_05_restore_puts_the_backup_back_at_the_next_start_and_is_audited() {
    let folder = tempfile::tempdir().expect("folder");
    let backup_file = {
        let state = start(folder.path());
        finish_setup(&state);
        let done = backup_commands::backup_now(&state).expect("backed up");
        rename_shop(&state, "Changed Later");

        backup_commands::backup_restore(&state, &done.file).expect("restore asked");
        done.file
    };

    let state = start(folder.path());

    assert_eq!(shop_name(&state), "Rahim Store");
    assert_eq!(audit_count(&state, "backup.restore"), 1);
    let reasons: Vec<String> = backup::list_backups(&state.paths.backups())
        .expect("list")
        .into_iter()
        .map(|row| row.reason)
        .collect();
    assert!(reasons.contains(&"pre-restore".to_string()), "{reasons:?}");
    // The backup file itself is left where it was.
    assert!(std::path::Path::new(&backup_file).exists());
}

#[test]
fn bak_05_a_file_that_is_not_a_backup_is_refused() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let junk = folder.path().join("pos-20261001-120000-manual.sqlite");
    std::fs::write(&junk, b"not a database").expect("file");

    assert!(backup_commands::backup_restore(&state, &junk.to_string_lossy()).is_err());
    assert!(!state.paths.restore_marker().exists());
}

#[test]
fn bak_06_a_damaged_database_opens_the_recovery_screen_which_restores_a_backup() {
    let folder = tempfile::tempdir().expect("folder");
    {
        let state = start(folder.path());
        finish_setup(&state);
        backup::make_backup(&state, "auto").expect("backed up");
    }
    let database = folder.path().join("pos.sqlite");
    for ending in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ending}", database.display()));
    }
    std::fs::write(&database, vec![7_u8; 8192]).expect("damaged");

    let state = start(folder.path());
    assert_eq!(state.health, DbHealth::Damaged);
    assert!(startup_status(&state).expect("status").damaged);
    let offered = recovery_backups(&state).expect("list");
    assert_eq!(offered.len(), 1);
    assert!(offered[0].usable);
    recovery_restore(&state, &offered[0].file).expect("restore asked");
    drop(state);

    let state = start(folder.path());
    assert_eq!(state.health, DbHealth::Ok);
    assert_eq!(shop_name(&state), "Rahim Store");
}

#[test]
fn bak_06_the_recovery_commands_work_only_on_a_damaged_database() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());

    assert!(matches!(
        recovery_backups(&state),
        Err(AppError::Conflict { .. })
    ));
}
