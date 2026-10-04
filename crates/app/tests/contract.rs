//! Workflow tests for the app. Only start-up wiring exists so far.

#[test]
fn every_module_migration_applies_to_an_empty_database() {
    let database = db::Db::open_in_memory().expect("memory database opens");
    db::migrate(&database, &app::migrations::all()).expect("all migrations run");
    let waiting = db::pending(&database, &app::migrations::all()).expect("pending works");
    assert!(waiting.is_empty());
}
