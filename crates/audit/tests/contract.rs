//! Contract tests: the public API and views behave as the module page says.
//! The module isn't built yet, so this only checks its migrations are labelled.

#[test]
fn migrations_belong_to_this_module() {
    assert_eq!(audit::migrations().module, "audit");
}
