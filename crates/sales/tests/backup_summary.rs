//! What the backup restore list shows about each backup: number of sales and the
//! last invoice number (BAK-05). `app` opens the backup read-only and asks sales.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::Id;
use rusqlite::{Connection, OpenFlags};
use support::{Till, now, run_migrations, sell, test_db};

#[test]
fn bak_05_a_database_without_sales_shows_none() {
    let conn = test_db();
    let summary = sales::sales_summary(&conn).unwrap();
    assert_eq!(summary.sale_count, 0);
    assert_eq!(summary.last_invoice_no, None);
}

#[test]
fn bak_05_a_backup_made_before_the_sales_tables_existed_shows_none() {
    // For example, a backup taken before the first setup finished.
    let conn = Connection::open_in_memory().unwrap();
    let summary = sales::sales_summary(&conn).unwrap();
    assert_eq!(summary.sale_count, 0);
    assert_eq!(summary.last_invoice_no, None);
}

#[test]
fn bak_05_counts_every_sale_and_gives_the_last_invoice_number() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let first = sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    // A voided sale keeps its number, so it still counts.
    let tx = conn.transaction().unwrap();
    sales::void(&tx, &first.id, &till.user, None, "mistake", now()).unwrap();
    tx.commit().unwrap();

    let summary = sales::sales_summary(&conn).unwrap();
    assert_eq!(summary.sale_count, 3);
    assert_eq!(summary.last_invoice_no.as_deref(), Some("T1-000003"));
}

#[test]
fn bak_05_works_on_a_backup_opened_read_only() {
    let folder = std::env::temp_dir().join(format!("pos-sales-backup-{}", Id::new()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("pos-backup.sqlite");
    {
        let mut conn = Connection::open(&path).unwrap();
        run_migrations(&conn);
        sell(&mut conn, &Till::new("T1"), &[(&Id::new(), 1_000)]);
    }

    // The same flags as `db::open_read_only`.
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let backup = Connection::open_with_flags(&path, flags).unwrap();
    let summary = sales::sales_summary(&backup).unwrap();
    assert_eq!(summary.sale_count, 1);
    assert_eq!(summary.last_invoice_no.as_deref(), Some("T1-000001"));

    drop(backup);
    std::fs::remove_dir_all(&folder).unwrap();
}
