//! Receipts and shift reports on the receipt printer (Req - Hardware and Printing,
//! PAY-09, PAY-10, SHF-05, SHF-06). Everything printed is read back from the saved
//! sale or shift, so a reprint is the same as the first printout (ADR-011).
//!
//! Printing happens after the commit. A failure never undoes anything: it comes back
//! in `PrintResult`, and the printout waits for "Retry" or the system printer (HW-08).

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pos_core::{AppError, Bitmap1, CurrencyFormat, Id, Money, TaxMode, Timestamp};
use printing::escpos::{self, DrawerPin, EscPosOpts};
use printing::{
    CustomerOnReceipt, Doc, LineDiscount, PaymentLine, ReceiptData, ReceiptLayout, ReceiptLine,
    ShopHeader, TaxSummaryLine, logo_from_image, rasterize, receipt, shift_report, to_png,
};
use rusqlite::Connection;
use sales::{PaymentKind, Sale, SaleStatus};
use serde::{Deserialize, Serialize};
use settings::{HardwareSettings, ReceiptSettings, ShopProfile, TaxSettings, TillSettings};
use ts_rs::TS;

use crate::audit_actions::RECEIPT_REPRINT;
use crate::audit_log;
use crate::devices::Devices;
use crate::receipt_words;
use crate::state::AppState;
use crate::workflows::till::{conflict, till_settings, till_shift};

/// Something the receipt printer prints, named so it can be printed again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum Printable {
    /// A sale's receipt.
    Receipt { sale_id: Id },
    /// A shift's X report (still open) or Z report (closed).
    ShiftReport { shift_id: Id },
}

/// How a printout or a drawer pulse went (HW-08). The screen shows a banner when
/// `printed` is false, with "Retry" and "Print on the system printer" for a printout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PrintResult {
    /// What was printed. None for a drawer pulse on its own.
    pub printable: Option<Printable>,
    pub printed: bool,
    /// Why it failed. Example: a `Hardware` error with code "timeout".
    pub problem: Option<AppError>,
}

/// Prints a sale's receipt, as many copies as the settings say. With `open_drawer`
/// the first copy also opens the cash drawer (HW-04). `copy` marks a reprint (PAY-10).
pub fn print_receipt(
    state: &AppState,
    devices: &Devices,
    sale_id: &Id,
    copy: bool,
    open_drawer: bool,
) -> PrintResult {
    let printable = Printable::Receipt {
        sale_id: sale_id.clone(),
    };
    print(state, devices, printable, copy, open_drawer)
}

/// Prints a shift's report: X while the shift is open, Z once it is closed (SHF-05, SHF-06).
pub fn print_shift_report(state: &AppState, devices: &Devices, shift_id: &Id) -> PrintResult {
    let printable = Printable::ShiftReport {
        shift_id: shift_id.clone(),
    };
    print(state, devices, printable, false, false)
}

/// Opens the cash drawer with a pulse through the receipt printer (HW-04).
pub fn open_drawer(state: &AppState, devices: &Devices) -> PrintResult {
    let sent = state
        .db
        .read(|conn| Ok(settings::get::<HardwareSettings>(conn)?))
        .and_then(|hardware| {
            let bytes = escpos::drawer_pulse(drawer_pin(&hardware))?;
            devices.send(devices.receipt_printer(&hardware).as_ref(), &bytes)
        });
    match sent {
        Ok(()) => PrintResult {
            printable: None,
            printed: true,
            problem: None,
        },
        Err(problem) => {
            tracing::warn!(%problem, "the drawer did not open");
            PrintResult {
                printable: None,
                printed: false,
                problem: Some(problem),
            }
        }
    }
}

/// "Reprint last receipt" (PAY-10): the newest sale on this till, marked COPY.
pub fn reprint_last(
    state: &AppState,
    devices: &Devices,
    user: &Id,
) -> Result<PrintResult, AppError> {
    let sale = state.db.read(|conn| {
        let till = till_settings(conn)?;
        Ok(sales::last_sale(conn, &till.id)?)
    })?;
    let sale = sale.ok_or_else(|| conflict("no_sale_to_reprint"))?;
    reprint(state, devices, user, &sale)
}

/// Reprints any receipt by its invoice number (PAY-10), marked COPY.
pub fn reprint_invoice(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    invoice_no: &str,
) -> Result<PrintResult, AppError> {
    let sale = state
        .db
        .read(|conn| Ok(sales::find_by_invoice(conn, invoice_no)?))?
        .ok_or_else(|| AppError::NotFound {
            entity: "sale".into(),
            key: invoice_no.trim().into(),
        })?;
    reprint(state, devices, user, &sale)
}

/// The X report of the user's own open shift (SHF-05).
pub fn x_report(state: &AppState, devices: &Devices, user: &Id) -> Result<PrintResult, AppError> {
    let shift = state
        .db
        .read(till_shift)?
        .ok_or_else(|| conflict("shift_not_open"))?;
    if shift.user_id != *user {
        return Err(conflict("shift_not_yours"));
    }
    Ok(print_shift_report(state, devices, &shift.id))
}

/// "Retry" on the banner: sends a failed printout again. A receipt that took cash
/// opens the drawer too, because the first pulse failed with the printout.
pub fn retry(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
) -> Result<PrintResult, AppError> {
    require_unprinted(devices, printable)?;
    Ok(match printable {
        Printable::Receipt { sale_id } => {
            let took_cash = state
                .db
                .read(|conn| Ok(took_cash(&sales::get_sale(conn, sale_id)?)))?;
            print_receipt(state, devices, sale_id, false, took_cash)
        }
        Printable::ShiftReport { shift_id } => print_shift_report(state, devices, shift_id),
    })
}

/// "Print on the system printer" (HW-07): the failed printout as PNG pictures for the
/// screen, which prints them through the computer's print dialog.
pub fn system_printer_pictures(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
) -> Result<Vec<String>, AppError> {
    require_unprinted(devices, printable)?;
    let picture = picture(state, devices, printable, false)?;
    let png = to_png(&picture)?;
    Ok(vec![format!(
        "data:image/png;base64,{}",
        STANDARD.encode(&png)
    )])
}

/// Hides the banner: the printout no longer waits for "Retry".
pub fn dismiss(devices: &Devices, printable: &Printable) -> Result<(), AppError> {
    devices.mark_printed(printable)
}

/// A failed printout may be sent again; anything else needs the reprint permission.
fn require_unprinted(devices: &Devices, printable: &Printable) -> Result<(), AppError> {
    if devices.is_unprinted(printable)? {
        Ok(())
    } else {
        Err(conflict("print_not_waiting"))
    }
}

/// Prints the sale marked COPY and records who reprinted which invoice.
fn reprint(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    sale: &Sale,
) -> Result<PrintResult, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        audit_log::record(
            tx,
            clock,
            RECEIPT_REPRINT,
            serde_json::json!({ "sale_id": sale.id, "invoice_no": sale.invoice_no }),
            Some(user.clone()),
        )
    })?;
    Ok(print_receipt(state, devices, &sale.id, true, false))
}

/// Draws and sends a printout. Any failure is remembered for "Retry" and returned.
fn print(
    state: &AppState,
    devices: &Devices,
    printable: Printable,
    copy: bool,
    open_drawer: bool,
) -> PrintResult {
    match send(state, devices, &printable, copy, open_drawer) {
        Ok(()) => {
            // A retry that worked takes the printout off the waiting list.
            if let Err(problem) = devices.mark_printed(&printable) {
                tracing::warn!(%problem, "the print list could not be updated");
            }
            PrintResult {
                printable: Some(printable),
                printed: true,
                problem: None,
            }
        }
        Err(problem) => {
            tracing::warn!(%problem, ?printable, "a printout failed");
            if let Err(error) = devices.mark_unprinted(&printable) {
                tracing::warn!(%error, "the print list could not be updated");
            }
            PrintResult {
                printable: Some(printable),
                printed: false,
                problem: Some(problem),
            }
        }
    }
}

fn send(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
    copy: bool,
    open_drawer: bool,
) -> Result<(), AppError> {
    let picture = picture(state, devices, printable, copy)?;
    let (hardware, receipt_settings) = state.db.read(|conn| {
        let hardware: HardwareSettings = settings::get(conn)?;
        let receipt_settings: ReceiptSettings = settings::get(conn)?;
        Ok((hardware, receipt_settings))
    })?;
    // Reports and reprints print once; a new receipt as many times as the settings say.
    let copies = match printable {
        Printable::Receipt { .. } if !copy => receipt_settings.copies.clamp(1, 3),
        _ => 1,
    };
    let printer = devices.receipt_printer(&hardware);
    for number in 0..copies {
        let opts = EscPosOpts {
            drawer: (open_drawer && number == 0).then(|| drawer_pin(&hardware)),
            ..EscPosOpts::default()
        };
        devices.send(printer.as_ref(), &escpos::encode(&picture, &opts)?)?;
    }
    Ok(())
}

/// The printout as the picture the printer prints.
fn picture(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
    copy: bool,
) -> Result<Bitmap1, AppError> {
    let offset = state.clock.local_offset_minutes();
    let doc = state.db.read(|conn| match printable {
        Printable::Receipt { sale_id } => receipt_doc(conn, offset, sale_id, copy),
        Printable::ShiftReport { shift_id } => report_doc(conn, offset, shift_id),
    })?;
    Ok(rasterize(&doc, &devices.fonts)?)
}

/// A sale's receipt, laid out with the shop's receipt settings (SET-14).
pub fn receipt_doc(
    conn: &Connection,
    utc_offset_minutes: i32,
    sale_id: &Id,
    copy: bool,
) -> Result<Doc, AppError> {
    let sale = sales::get_sale(conn, sale_id)?;
    let layout = receipt_layout(conn)?;
    let data = receipt_data(conn, utc_offset_minutes, &sale, &layout, copy)?;
    Ok(receipt(&data, &layout, &receipt_words::english()))
}

/// A shift's X or Z report (SHF-08), from `reports::shift_report`.
pub fn report_doc(
    conn: &Connection,
    utc_offset_minutes: i32,
    shift_id: &Id,
) -> Result<Doc, AppError> {
    let mut till_names = BTreeMap::new();
    let till: TillSettings = settings::get(conn)?;
    if let Some(till_id) = till.till_id {
        till_names.insert(till_id, till.till_code);
    }
    let context = reports::ShopContext {
        user_names: user_names(conn)?,
        till_names,
        payment_method_names: sales::payment_methods(conn)?
            .into_iter()
            .map(|method| (method.id, method.name))
            .collect(),
        utc_offset_minutes,
    };
    let table = reports::shift_report(conn, shift_id, &context)?;
    let layout = receipt_layout(conn)?;
    Ok(shift_report(&table, &layout, &receipt_words::english()))
}

/// The receipt options, shop header and logo from the settings.
fn receipt_layout(conn: &Connection) -> Result<ReceiptLayout, AppError> {
    let shop: ShopProfile = settings::get(conn)?;
    let options: ReceiptSettings = settings::get(conn)?;
    let currency: CurrencyFormat = settings::get(conn)?;
    let paper = match options.paper_width {
        settings::PaperWidth::Mm58 => printing::PaperWidth::Mm58,
        settings::PaperWidth::Mm80 => printing::PaperWidth::Mm80,
    };
    let logo = if options.show_logo {
        shop.logo_path.as_deref().and_then(|path| logo(path, paper))
    } else {
        None
    };
    let mut footer_lines = shop.receipt_footer.clone();
    footer_lines.extend(options.footer_lines.iter().cloned());
    Ok(ReceiptLayout {
        paper,
        currency,
        shop: ShopHeader {
            name: shop.name,
            address_lines: shop.address_lines,
            phone: shop.phone,
            tax_reg_label: shop.tax_reg_label,
            tax_reg_no: shop.tax_reg_no,
        },
        show_logo: logo.is_some(),
        logo,
        header_lines: options.header_lines,
        footer_lines,
        show_tax_breakdown: options.show_tax_breakdown,
        show_tax_reg_no: options.show_tax_reg_no,
        show_invoice_barcode: options.show_invoice_barcode,
        show_customer_points: options.show_customer_balance,
    })
}

/// The shop's logo at most two thirds of the paper wide. A missing or broken file
/// prints no logo rather than no receipt.
fn logo(path: &str, paper: printing::PaperWidth) -> Option<Bitmap1> {
    let file = std::fs::read(path)
        .map_err(|error| tracing::warn!(path, %error, "the shop logo could not be read"))
        .ok()?;
    logo_from_image(&file, paper.dots() * 2 / 3)
        .map_err(|error| tracing::warn!(path, %error, "the shop logo could not be drawn"))
        .ok()
}

/// Everything the receipt prints, from the saved sale. No money is worked out here
/// beyond adding the saved line amounts.
fn receipt_data(
    conn: &Connection,
    utc_offset_minutes: i32,
    sale: &Sale,
    layout: &ReceiptLayout,
    copy: bool,
) -> Result<ReceiptData, AppError> {
    let till: TillSettings = settings::get(conn)?;
    let tax: TaxSettings = settings::get(conn)?;
    let items = receipt_lines(conn, sale)?;
    let mut payments = Vec::with_capacity(sale.payments.len());
    for payment in &sale.payments {
        let method = sales::payment_method(conn, &payment.method_id)?;
        payments.push(PaymentLine {
            method: method.name,
            // Cash prints what the customer handed over; the change is on its own line.
            amount: payment.tendered.unwrap_or(payment.amount),
            reference: payment.reference.clone(),
        });
    }
    let till_name = if till.till_id.as_ref() == Some(&sale.till_id) {
        till.till_code
    } else {
        sale.till_id.to_string()
    };
    let invoice_barcode = if layout.show_invoice_barcode {
        Some(barcode::bitmap(
            &sale.invoice_no,
            barcode::Symbology::Code128,
            layout.paper.dots() * 3 / 4,
            60,
        )?)
    } else {
        None
    };
    Ok(ReceiptData {
        invoice_no: sale.invoice_no.clone(),
        printed_at: local_time(sale.completed_at, utc_offset_minutes),
        till: till_name,
        cashier: user_names(conn)?
            .remove(&sale.user_id)
            .unwrap_or_else(|| sale.user_id.to_string()),
        customer: sale.customer.as_ref().map(|customer| CustomerOnReceipt {
            name: customer.name.clone(),
            points_balance: None,
            credit_balance: None,
        }),
        lines: items.lines,
        subtotal: items.subtotal,
        discount_total: sale.totals.invoice_discount.checked_neg()?,
        tax_summary: items.tax_summary,
        tax_label: tax.label,
        prices_include_tax: sale.tax_mode == TaxMode::Inclusive,
        rounding: sale.totals.rounding,
        total: sale.totals.total,
        payments,
        change: sale.totals.change,
        void_reason: match sale.status {
            SaleStatus::Voided => sale.void.as_ref().map(|void| void.reason.clone()),
            SaleStatus::Completed => None,
        },
        copy,
        invoice_barcode,
        qr_data: None,
    })
}

/// The receipt's item lines, their subtotal after line discounts, and the tax per rate.
struct ReceiptItems {
    lines: Vec<ReceiptLine>,
    subtotal: Money,
    tax_summary: Vec<TaxSummaryLine>,
}

fn receipt_lines(conn: &Connection, sale: &Sale) -> Result<ReceiptItems, AppError> {
    let words = receipt_words::english();
    let decimals = unit_decimals(conn)?;
    let mut items = ReceiptItems {
        lines: Vec::with_capacity(sale.lines.len()),
        subtotal: Money::zero(),
        tax_summary: Vec::new(),
    };
    for line in &sale.lines {
        let snapshot = &line.snapshot;
        let line_discount = snapshot
            .promo_discount
            .checked_add(snapshot.tier_discount)?
            .checked_add(snapshot.manual_discount)?;
        items.subtotal = items
            .subtotal
            .checked_add(snapshot.base)?
            .checked_sub(line_discount)?;
        let discount = if line_discount == Money::zero() {
            None
        } else {
            Some(LineDiscount {
                label: snapshot
                    .promotion
                    .as_ref()
                    .map_or_else(|| words.discount.clone(), |promo| promo.name.clone()),
                amount: line_discount.checked_neg()?,
            })
        };
        // Pieces print without a unit ("2 × 90.00"); kg, litres and the like with it.
        let shows_unit = decimals
            .get(&snapshot.unit_symbol)
            .is_none_or(|places| *places > 0);
        items.lines.push(ReceiptLine {
            name: snapshot.name.clone(),
            qty: snapshot.qty,
            unit: if shows_unit {
                snapshot.unit_symbol.clone()
            } else {
                String::new()
            },
            unit_price: snapshot.unit_price,
            amount: snapshot.base,
            discount,
        });
        match items
            .tax_summary
            .iter_mut()
            .find(|each| each.name == snapshot.tax_name)
        {
            Some(each) => {
                each.net = each.net.checked_add(snapshot.net)?;
                each.tax = each.tax.checked_add(snapshot.tax)?;
            }
            None => items.tax_summary.push(TaxSummaryLine {
                name: snapshot.tax_name.clone(),
                net: snapshot.net,
                tax: snapshot.tax,
            }),
        }
    }
    Ok(items)
}

/// True when a payment on the sale was cash: the drawer opens for it (HW-04).
pub(crate) fn took_cash(sale: &Sale) -> bool {
    sale.payments
        .iter()
        .any(|payment| payment.kind == PaymentKind::Cash)
}

/// Every unit symbol with its number of decimals. Example: "pc" → 0, "kg" → 3.
fn unit_decimals(conn: &Connection) -> Result<BTreeMap<String, u8>, AppError> {
    Ok(catalog::list_units(conn, true)?
        .into_iter()
        .map(|unit| (unit.symbol, unit.decimals))
        .collect())
}

/// Each user's display name, for the cashier on receipts and reports.
fn user_names(conn: &Connection) -> Result<BTreeMap<Id, String>, AppError> {
    Ok(auth::list_users(conn)?
        .into_iter()
        .map(|user| (user.id, user.display_name))
        .collect())
}

fn drawer_pin(hardware: &HardwareSettings) -> DrawerPin {
    match hardware.drawer_pin {
        settings::DrawerPin::Pin2 => DrawerPin::Pin2,
        settings::DrawerPin::Pin5 => DrawerPin::Pin5,
    }
}

/// The shop's local date and time. Example: "2026-10-05 14:00".
fn local_time(at: Timestamp, utc_offset_minutes: i32) -> String {
    let Some(offset) = chrono::FixedOffset::east_opt(utc_offset_minutes * 60) else {
        return String::new();
    };
    chrono::DateTime::from_timestamp_millis(at.0)
        .map(|utc| {
            utc.with_timezone(&offset)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}
