//! Reports through the app (C-M11, Req - Reports): every report runs from the screen's
//! request, a cashier sees only their own shifts' X and Z reports (RPT-10), reports
//! export to Excel with the shop's header (RPT-09) and print on receipt paper (RPT-01).

mod common;

use app::commands::checkout as till;
use app::commands::reports as report_commands;
use app::session;
use app::state::AppState;
use app::workflows::customer_records::CustomerSave;
use app::workflows::purchasing_admin::SupplierSave;
use app::workflows::report_runs::{ReportGroup, ReportRequest, SalesBy};
use common::*;
use pos_core::{AppError, BusinessDate, Cell, Id, Money, Permission, Qty, ReportTable};

/// A shop with bread in stock, a supplier, a customer, and one bread sold by the
/// owner for 50.00 cash on the owner's open shift.
fn shop_with_a_sale() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    let bread = till_product(
        &state,
        "Bread",
        "BREAD-1",
        5_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &bread, Qty(100_000));
    open_shift(&state, 100_000);
    till::checkout_scan(&state, "BREAD-1").expect("bread");
    till::checkout_complete(&state, devices(), &[cash(&state, 5_000)]).expect("sold");
    (folder, state)
}

fn today(state: &AppState) -> BusinessDate {
    report_commands::report_choices(state)
        .expect("choices")
        .today
}

fn daily(state: &AppState) -> ReportRequest {
    ReportRequest::DailySummary {
        date: today(state),
        till_id: None,
    }
}

/// The cells of the row whose first cell is `label`.
fn row<'a>(table: &'a ReportTable, label: &str) -> &'a [Cell] {
    table
        .rows
        .iter()
        .find(|cells| cells.first() == Some(&Cell::Text(label.into())))
        .unwrap_or_else(|| panic!("no row {label} in {}", table.title))
}

fn supplier(state: &AppState) -> Id {
    let save = SupplierSave {
        id: None,
        input: purchasing::SupplierInput {
            name: "Fresh Foods".into(),
            ..purchasing::SupplierInput::default()
        },
    };
    app::commands::purchasing::supplier_save(state, &save).expect("supplier")
}

fn customer(state: &AppState) -> Id {
    let input = CustomerSave {
        id: None,
        details: customers::CustomerDetails::quick("01711123456", "Asha"),
    };
    app::commands::customers::customer_save(state, &input).expect("customer")
}

#[test]
fn rpt_01_daily_summary_runs_from_the_screen_request() {
    let (_folder, state) = shop_with_a_sale();
    let table = report_commands::report_run(&state, &daily(&state)).expect("report");
    assert_eq!(table.title, "Daily summary");
    assert!(row(&table, "Net sales").contains(&Cell::Money(Money(5_000))));
    assert!(row(&table, "Total taken").contains(&Cell::Money(Money(5_000))));
}

#[test]
fn rpt_every_report_runs_through_the_app() {
    let (_folder, state) = shop_with_a_sale();
    let day = today(&state);
    let (supplier_id, customer_id) = (supplier(&state), customer(&state));
    let shift_id = report_commands::report_shifts(&state)
        .expect("shifts")
        .shifts[0]
        .id
        .clone();
    let requests = vec![
        daily(&state),
        ReportRequest::Shift { shift_id },
        ReportRequest::SalesBy {
            by: SalesBy::Product,
            from: day,
            to: day,
            category_id: None,
            brand_id: None,
            cashier_id: None,
            top: Some(10),
        },
        ReportRequest::Profit {
            by: ReportGroup::Category,
            from: day,
            to: day,
        },
        ReportRequest::StockValue {
            by: ReportGroup::Product,
        },
        ReportRequest::LowStock,
        ReportRequest::NearExpiry { days: 30 },
        ReportRequest::StockMovement { from: day, to: day },
        ReportRequest::Tax { from: day, to: day },
        ReportRequest::PayablesAging,
        ReportRequest::ReceivablesAging,
        ReportRequest::SupplierStatement {
            supplier_id,
            from: day,
            to: day,
        },
        ReportRequest::CustomerStatement {
            customer_id,
            from: day,
            to: day,
        },
        ReportRequest::DiscountsOverrides { from: day, to: day },
    ];
    for request in requests {
        let table = report_commands::report_run(&state, &request)
            .unwrap_or_else(|error| panic!("{request:?}: {error:?}"));
        assert!(!table.title.is_empty(), "{request:?}");
    }

    let sales = report_commands::report_run(
        &state,
        &ReportRequest::SalesBy {
            by: SalesBy::Product,
            from: day,
            to: day,
            category_id: None,
            brand_id: None,
            cashier_id: None,
            top: None,
        },
    )
    .expect("sales by product");
    assert_eq!(row(&sales, "Bread")[0], Cell::Text("Bread".into()));
}

#[test]
fn rpt_a_backwards_range_or_an_unknown_supplier_is_refused() {
    let (_folder, state) = shop_with_a_sale();
    let day = today(&state);
    let backwards = ReportRequest::Tax {
        from: day,
        to: BusinessDate(day.0.pred_opt().expect("yesterday")),
    };
    assert!(matches!(
        report_commands::report_run(&state, &backwards),
        Err(AppError::Validation { code, .. }) if code == "date_range_backwards"
    ));
    let unknown = ReportRequest::SupplierStatement {
        supplier_id: Id::new(),
        from: day,
        to: day,
    };
    assert!(matches!(
        report_commands::report_run(&state, &unknown),
        Err(AppError::NotFound { entity, .. }) if entity == "supplier"
    ));
}

#[test]
fn rpt_10_a_cashier_sees_only_the_x_and_z_reports_of_their_own_shifts() {
    let (_folder, state) = shop_with_a_sale();
    let owners_daily = daily(&state);
    let owners_shift = report_commands::report_shifts(&state)
        .expect("shifts")
        .shifts[0]
        .id
        .clone();
    app::commands::shifts::shift_close(
        &state,
        devices(),
        &[app::workflows::shifts::CountLine {
            denomination: Money(100_000),
            count: 1,
        }],
        None,
    )
    .expect("owner's shift closed");

    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    open_shift(&state, 50_000);

    let listed = report_commands::report_shifts(&state)
        .expect("own shifts")
        .shifts;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].cashier, "rina name");
    assert!(!listed[0].closed);
    let own = ReportRequest::Shift {
        shift_id: listed[0].id.clone(),
    };
    let x_report = report_commands::report_run(&state, &own).expect("own X report");
    assert!(x_report.title.starts_with("X report"), "{}", x_report.title);
    assert!(report_commands::report_print(&state, devices(), &own).is_ok());

    let denied = |result: Result<_, AppError>| matches!(result, Err(AppError::PermissionDenied { permission }) if permission == Permission::ReportsViewAll);
    let others = ReportRequest::Shift {
        shift_id: owners_shift,
    };
    assert!(denied(
        report_commands::report_run(&state, &others).map(|_| ())
    ));
    assert!(denied(
        report_commands::report_run(&state, &owners_daily).map(|_| ())
    ));
    assert!(denied(
        report_commands::report_print(&state, devices(), &others).map(|_| ())
    ));

    session::logout(&state).expect("out");
    sign_in(&state, "owner", ADMIN_PASSWORD);
    assert_eq!(
        report_commands::report_shifts(&state)
            .expect("all")
            .shifts
            .len(),
        2
    );
}

#[test]
fn rpt_09_excel_export_has_the_shop_header_and_real_numbers() {
    let (folder, state) = shop_with_a_sale();
    let file = folder.path().join("daily.xlsx");
    report_commands::report_export_xlsx(&state, &daily(&state), file.to_str().expect("path"))
        .expect("exported");

    let read = io::read_file(&file, None).expect("read back");
    assert_eq!(read.headers[0], "Rahim Store");
    let texts: Vec<&str> = read.rows.iter().map(|cells| cells[0].as_str()).collect();
    assert!(texts.contains(&"Daily summary"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text.starts_with("Period: ")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with("Generated: ")),
        "{texts:?}"
    );
    let net = read
        .rows
        .iter()
        .find(|cells| cells[0] == "Net sales")
        .expect("net sales row");
    assert!(net.iter().any(|cell| cell == "50"), "{net:?}");
}

#[test]
fn rpt_01_a_report_prints_on_receipt_paper() {
    let (_folder, state) = shop_with_a_sale();
    let printed =
        report_commands::report_print(&state, devices(), &daily(&state)).expect("printed");
    assert!(printed.printed, "{printed:?}");
    assert!(printed.problem.is_none());
}
