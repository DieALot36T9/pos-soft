//! First-run setup through the app's workflows (rules SET-01 to SET-12).

mod common;

use app::commands::startup::startup_status;
use app::session;
use app::workflows::setup::{self, AdminStep, TaxStep};
use common::*;
use pos_core::{AppError, Permission, TaxMode};
use settings::{BackupSettings, Setting, SetupStep, ShopProfile, TillSettings};

fn field_and_code(error: AppError) -> (Option<String>, String) {
    match error {
        AppError::Validation { field, code, .. } => (field, code),
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn set_01_a_fresh_install_opens_the_wizard_at_the_first_step() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());

    let view = startup_status(&state).expect("status");

    assert!(!view.damaged);
    assert!(!view.setup.is_complete);
    assert_eq!(view.setup.next_step, Some(SetupStep::Shop));
    assert_eq!(view.session, None);
}

#[test]
fn set_02_setup_resumes_after_the_app_is_closed() {
    let folder = tempfile::tempdir().expect("folder");
    {
        let state = start(folder.path());
        setup::save_shop(&state, &shop("Rahim Store")).expect("shop");
        setup::save_money(&state, &money()).expect("money");
    }
    let state = start(folder.path());

    let status = setup::status(&state).expect("status");

    assert_eq!(status.completed, vec![SetupStep::Shop, SetupStep::Currency]);
    assert_eq!(status.next_step, Some(SetupStep::Tax));
    let saved: ShopProfile = state
        .db
        .read(|conn| Ok(settings::get(conn)?))
        .expect("read");
    assert_eq!(saved.name, "Rahim Store");
}

#[test]
fn set_04_a_shop_without_a_name_is_refused_and_the_step_stays_open() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());

    let error = setup::save_shop(&state, &shop("  ")).expect_err("refused");

    assert_eq!(field_and_code(error).1, "shop_name_missing");
    assert!(setup::status(&state).expect("status").completed.is_empty());
}

#[test]
fn set_07_the_tax_step_needs_at_least_one_rate() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    let step = TaxStep {
        label: "VAT".into(),
        mode: TaxMode::Exclusive,
        default_rate_id: None,
    };

    let error = setup::save_tax(&state, &step).expect_err("refused");

    assert_eq!(
        field_and_code(error),
        (Some("rates".into()), "tax_rate_missing".into())
    );
}

#[test]
fn set_08_the_admin_password_must_be_typed_the_same_twice() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    let step = AdminStep {
        password_again: "something-else".into(),
        ..admin()
    };

    let error = setup::save_admin(&state, step).expect_err("refused");

    assert_eq!(
        field_and_code(error),
        (Some("password_again".into()), "passwords_differ".into())
    );
}

#[test]
fn set_08_the_admin_step_shows_a_recovery_code_and_signs_the_admin_in() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());

    let created = setup::save_admin(&state, admin()).expect("admin");

    // 20 characters in groups of 4: "7KQ2-M9XD-4RTW-PZ8C-H3NB".
    assert_eq!(created.recovery_code.len(), 24);
    assert!(created.session.is_admin);
    assert!(session::require(&state, Permission::SettingsManage).is_ok());
    // A second admin step is refused.
    assert!(matches!(
        setup::save_admin(&state, admin()),
        Err(AppError::Conflict { .. })
    ));
}

#[test]
fn set_09_the_till_step_gives_the_till_its_id_and_a_clean_code() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    let step = app::workflows::setup::TillStep {
        till_code: " t2 ".into(),
        business_day_cutoff_hour: 3,
    };

    setup::save_till(&state, &step).expect("till");

    let till: TillSettings = state
        .db
        .read(|conn| Ok(settings::get(conn)?))
        .expect("read");
    assert!(till.till_id.is_some());
    assert_eq!(till.till_code, "T2");
    assert_eq!(till.business_day_cutoff_hour, 3);
}

#[test]
fn set_11_a_backup_folder_that_cannot_be_written_is_refused() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    // A file where the folder should be: nothing can be written inside it.
    let blocker = folder.path().join("not-a-folder");
    std::fs::write(&blocker, b"x").expect("file");
    let step = BackupSettings {
        primary_dir: blocker.join("backups").to_string_lossy().into_owned(),
        ..BackupSettings::default_value()
    };

    let error = setup::save_backup(&state, &step).expect_err("refused");

    assert_eq!(
        field_and_code(error),
        (Some("primary_dir".into()), "folder_not_writable".into())
    );
}

#[test]
fn set_10_setup_completes_without_the_hardware_step_and_then_closes() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());

    finish_setup(&state);

    let status = setup::status(&state).expect("status");
    assert!(status.is_complete);
    assert_eq!(status.next_step, Some(SetupStep::Hardware));
    let again = setup::save_shop(&state, &shop("Another")).expect_err("setup is closed");
    assert!(matches!(again, AppError::Conflict { code, .. } if code == "setup_already_done"));
}
