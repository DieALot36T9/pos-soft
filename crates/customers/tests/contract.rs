//! Contract tests: customer records, search and the published views behave as the
//! module page says. Ledger tests are in `ledgers.rs`; random tests in `properties.rs`.

mod support;

use customers::{CustomerDetails, CustomersError, PointsKind};
use pos_core::{AppError, Id, Money, Timestamp};
use support::{COUNTRY, ctx, quick_customer, test_db};

#[test]
fn migrations_belong_to_this_module() {
    assert_eq!(customers::migrations().module, "customers");
}

#[test]
fn cus_01_phone_is_cleaned_to_e164() {
    assert_eq!(
        customers::normalize_phone("+880 1711-123456", COUNTRY).unwrap(),
        "+8801711123456"
    );
    assert_eq!(
        customers::normalize_phone("01711-123456", COUNTRY).unwrap(),
        "+8801711123456"
    );
}

#[test]
fn cus_01_a_number_that_cannot_exist_is_refused() {
    let error = customers::normalize_phone("123", COUNTRY).unwrap_err();
    assert!(matches!(error, CustomersError::PhoneInvalid));
    assert_eq!(
        AppError::from(error),
        AppError::Validation {
            field: Some("phone".to_string()),
            code: "phone_invalid".to_string(),
            params: Default::default(),
        }
    );
}

#[test]
fn cus_01_an_unknown_country_is_refused() {
    let error = customers::normalize_phone("01711123456", "XX").unwrap_err();
    assert!(matches!(error, CustomersError::CountryInvalid(_)));
}

#[test]
fn cus_01_phone_is_unique_whatever_way_it_is_typed() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    quick_customer(&tx, "01711-123456", "Rahim");

    let second = customers::create(
        &tx,
        CustomerDetails::quick("+880 1711 123456", "Karim"),
        COUNTRY,
        Timestamp(2),
    );

    match second {
        Err(CustomersError::PhoneInUse { name }) => assert_eq!(name, "Rahim"),
        other => panic!("expected phone_in_use, got {other:?}"),
    }
}

#[test]
fn cus_01_create_stores_every_field_and_starts_with_empty_balances() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let tier = Id::new();
    let details = CustomerDetails {
        phone: "01711123456".to_string(),
        name: "  Rahim Uddin ".to_string(),
        email: Some("rahim@example.com".to_string()),
        address: Some(" ".to_string()),
        tier_id: Some(tier.clone()),
        credit_limit: Money(500_000),
        loyalty_opt_out: true,
        notes: Some("Pays on Fridays".to_string()),
    };

    let id = customers::create(&tx, details, COUNTRY, Timestamp(10)).unwrap();
    let found = customers::get(&tx, &id).unwrap();

    assert_eq!(found.customer.phone, "+8801711123456");
    assert_eq!(found.customer.name, "Rahim Uddin");
    assert_eq!(found.customer.address, None);
    assert_eq!(found.customer.tier_id, Some(tier));
    assert_eq!(found.customer.credit_limit, Money(500_000));
    assert!(found.customer.loyalty_opt_out);
    assert_eq!(found.customer.created_at, Timestamp(10));
    assert_eq!(found.points, 0);
    assert_eq!(found.credit, Money::zero());
    assert_eq!(found.oldest_unpaid_at, None);
}

#[test]
fn cus_01_a_name_is_required_and_a_credit_limit_cannot_be_negative() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let no_name = customers::create(
        &tx,
        CustomerDetails::quick("01711123456", "   "),
        COUNTRY,
        Timestamp(1),
    );
    assert!(matches!(no_name, Err(CustomersError::NameMissing)));

    let mut details = CustomerDetails::quick("01711123456", "Rahim");
    details.credit_limit = Money(-1);
    let negative = customers::create(&tx, details, COUNTRY, Timestamp(1));
    assert!(matches!(negative, Err(CustomersError::CreditLimitNegative)));
}

#[test]
fn cus_01_update_changes_the_record_and_keeps_the_phone_unique() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    quick_customer(&tx, "01811123456", "Karim");

    let mut details = CustomerDetails::quick("01911123456", "Rahim Uddin");
    details.credit_limit = Money(100_000);
    customers::update(&tx, &rahim, details, COUNTRY, Timestamp(50)).unwrap();
    let found = customers::get(&tx, &rahim).unwrap().customer;
    assert_eq!(found.phone, "+8801911123456");
    assert_eq!(found.name, "Rahim Uddin");
    assert_eq!(found.credit_limit, Money(100_000));
    assert_eq!(found.updated_at, Timestamp(50));
    assert_eq!(found.created_at, Timestamp(1));

    let clash = customers::update(
        &tx,
        &rahim,
        CustomerDetails::quick("01811123456", "Rahim"),
        COUNTRY,
        Timestamp(60),
    );
    assert!(matches!(clash, Err(CustomersError::PhoneInUse { .. })));

    // Saving the same phone again is not a clash with itself.
    let same = CustomerDetails::quick("01911123456", "Rahim U.");
    customers::update(&tx, &rahim, same, COUNTRY, Timestamp(70)).unwrap();
}

#[test]
fn cus_01_unknown_customers_are_not_found() {
    let db = test_db();
    let error = customers::get(&db, &Id::new()).unwrap_err();
    assert!(matches!(error, CustomersError::CustomerNotFound(_)));
}

#[test]
fn cus_01_archived_customers_are_kept_but_hidden_from_lookup() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");

    customers::set_archived(&tx, &rahim, true, Timestamp(99)).unwrap();
    assert_eq!(
        customers::get(&tx, &rahim).unwrap().customer.archived_at,
        Some(Timestamp(99))
    );
    assert!(customers::lookup(&tx, "Rahim", 10).unwrap().is_empty());

    customers::set_archived(&tx, &rahim, false, Timestamp(100)).unwrap();
    assert_eq!(
        customers::get(&tx, &rahim).unwrap().customer.archived_at,
        None
    );
    assert_eq!(customers::lookup(&tx, "Rahim", 10).unwrap().len(), 1);
}

#[test]
fn cus_02_lookup_finds_part_of_a_phone_number() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    quick_customer(&tx, "01811999888", "Karim");

    for typed in ["01711", "+8801711", "1711-12", "3456"] {
        let hits = customers::lookup(&tx, typed, 10).unwrap();
        assert_eq!(hits.len(), 1, "typed {typed}");
        assert_eq!(hits[0].id, rahim);
    }
}

#[test]
fn cus_02_lookup_finds_the_start_of_any_word_in_the_name() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim Uddin");
    quick_customer(&tx, "01811999888", "Karim Mia");

    let hits = customers::lookup(&tx, "udd", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, rahim);
    assert_eq!(hits[0].phone, "+8801711123456");

    assert_eq!(customers::lookup(&tx, "rahim udd", 10).unwrap().len(), 1);
    assert_eq!(customers::lookup(&tx, "RAHIM", 10).unwrap().len(), 1);
    assert!(customers::lookup(&tx, "rahim mia", 10).unwrap().is_empty());
}

#[test]
fn cus_02_lookup_follows_name_changes() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    let renamed = CustomerDetails::quick("01711123456", "Abdul");
    customers::update(&tx, &rahim, renamed, COUNTRY, Timestamp(2)).unwrap();

    assert!(customers::lookup(&tx, "Rahim", 10).unwrap().is_empty());
    assert_eq!(customers::lookup(&tx, "Abd", 10).unwrap().len(), 1);
}

#[test]
fn cus_02_lookup_respects_the_limit_and_ignores_odd_input() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    quick_customer(&tx, "01711123456", "Rahim");
    quick_customer(&tx, "01811123456", "Rahima");
    quick_customer(&tx, "01911123456", "Rahman");

    assert_eq!(customers::lookup(&tx, "rah", 2).unwrap().len(), 2);
    assert!(customers::lookup(&tx, "", 10).unwrap().is_empty());
    assert!(customers::lookup(&tx, "  ", 10).unwrap().is_empty());
    // Search syntax characters are treated as plain text, never as an error.
    assert!(
        customers::lookup(&tx, "\" * - ( ) :", 10)
            .unwrap()
            .is_empty()
    );
    assert!(customers::lookup(&tx, "rah\"im", 10).is_ok());
}

#[test]
fn cus_01_view_customers_v_customers_shows_records_with_balances() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let rahim = quick_customer(&tx, "01711123456", "Rahim");
    customers::add_points(&tx, &rahim, 40, PointsKind::Earn, None, None, &ctx()).unwrap();

    let row: (String, String, String, i64, i64, i64) = tx
        .query_row(
            "SELECT id, phone, name, credit_limit_minor, points, credit_minor
             FROM customers_v_customers",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            rahim.to_string(),
            "+8801711123456".to_string(),
            "Rahim".to_string(),
            0,
            40,
            0
        )
    );
    // The other published columns exist.
    tx.execute_batch(
        "SELECT tier_id, oldest_unpaid_at, archived_at FROM customers_v_customers;
         SELECT id, customer_id, amount_minor, kind, method_kind, reference, ref_type, ref_id,
                at, business_date, user_id FROM customers_v_credit_ledger;
         SELECT id, customer_id, points_delta, kind, ref_type, ref_id, reason, at, user_id
         FROM customers_v_points_ledger;",
    )
    .unwrap();
}
