//! "Test print": a sample receipt with text, the currency symbol, a logo, a barcode
//! and a QR code, at 58 or 80 mm (rules HW-02, HW-03). Until the shop's settings are
//! saved (C-M2), the shop details and the logo are samples.

use hardware::RawPrinter;
use pos_core::{AppError, Bitmap1, CurrencyFormat, Money, Qty};
use printing::escpos::{self, EscPosOpts};
use printing::{
    Fonts, PaymentLine, ReceiptData, ReceiptLayout, ReceiptLine, ShopHeader, TaxSummaryLine,
    logo_from_image, rasterize, receipt,
};
use settings::{PaperWidth, Setting};

use crate::receipt_words;

/// A sample shop logo, shown until the shop adds its own in Settings.
const SAMPLE_LOGO: &[u8] = include_bytes!("../../assets/sample-logo.png");

/// The invoice number on the test receipt. Also the text of its barcode.
const SAMPLE_INVOICE: &str = "TEST-01";

/// Draws the test receipt and sends it to the printer. A test print never opens
/// the drawer. A printer error comes back as `AppError::Hardware` (rule HW-08).
pub fn run(printer: &dyn RawPrinter, paper: PaperWidth, fonts: &Fonts) -> Result<(), AppError> {
    let picture = picture(paper, fonts)?;
    let bytes = escpos::encode(&picture, &EscPosOpts::default())?;
    printer.send(&bytes)?;
    Ok(())
}

/// The test receipt as the picture the printer prints.
pub fn picture(paper: PaperWidth, fonts: &Fonts) -> Result<Bitmap1, AppError> {
    let paper = paper_for_printing(paper);
    let layout = sample_layout(paper)?;
    let data = sample_sale(paper)?;
    let doc = receipt(&data, &layout, &receipt_words::english());
    Ok(rasterize(&doc, fonts)?)
}

/// The settings module and the printing module each name the two paper widths.
fn paper_for_printing(paper: PaperWidth) -> printing::PaperWidth {
    match paper {
        PaperWidth::Mm58 => printing::PaperWidth::Mm58,
        PaperWidth::Mm80 => printing::PaperWidth::Mm80,
    }
}

/// The receipt options with every part switched on, so the test shows everything.
fn sample_layout(paper: printing::PaperWidth) -> Result<ReceiptLayout, AppError> {
    // The logo uses at most two thirds of the paper, like a real shop logo would.
    let logo = logo_from_image(SAMPLE_LOGO, paper.dots() * 2 / 3)?;
    Ok(ReceiptLayout {
        paper,
        currency: CurrencyFormat::default_value(),
        shop: ShopHeader {
            name: "Sample Shop".into(),
            address_lines: vec!["1 Market Road".into()],
            phone: "01700-000000".into(),
            tax_reg_label: "BIN".into(),
            tax_reg_no: "000000000-0000".into(),
        },
        logo: Some(logo),
        header_lines: vec!["TEST PRINT".into()],
        footer_lines: vec!["If you can read this, the printer works.".into()],
        show_logo: true,
        show_tax_breakdown: true,
        show_tax_reg_no: true,
        show_invoice_barcode: true,
        show_customer_points: false,
    })
}

/// A short sale: pieces, a weighed item, VAT, a cash payment with change.
fn sample_sale(paper: printing::PaperWidth) -> Result<ReceiptData, AppError> {
    let barcode = barcode::bitmap(
        SAMPLE_INVOICE,
        barcode::Symbology::Code128,
        paper.dots() * 3 / 4,
        60,
    )?;
    Ok(ReceiptData {
        invoice_no: SAMPLE_INVOICE.into(),
        printed_at: "2026-01-01 09:00".into(),
        till: "T1".into(),
        cashier: "Test".into(),
        customer: None,
        lines: vec![
            line("Milk 1L", Qty(2000), "", Money(9000), Money(18000)),
            line("Rice 5kg", Qty(1000), "", Money(65000), Money(65000)),
            line("Beef", Qty(1234), "kg", Money(75000), Money(92550)),
        ],
        subtotal: Money(175550),
        discount_total: Money(0),
        tax_summary: vec![TaxSummaryLine {
            name: "VAT 15%".into(),
            net: Money(152652),
            tax: Money(22898),
        }],
        tax_label: "VAT".into(),
        prices_include_tax: true,
        rounding: Money(-50),
        total: Money(175500),
        payments: vec![PaymentLine {
            method: "Cash".into(),
            amount: Money(200000),
            reference: None,
        }],
        change: Money(24500),
        void_reason: None,
        copy: false,
        invoice_barcode: Some(barcode),
        qr_data: Some(format!("TEST PRINT {SAMPLE_INVOICE}")),
    })
}

fn line(name: &str, qty: Qty, unit: &str, unit_price: Money, amount: Money) -> ReceiptLine {
    ReceiptLine {
        name: name.into(),
        qty,
        unit: unit.into(),
        unit_price,
        amount,
        discount: None,
    }
}
