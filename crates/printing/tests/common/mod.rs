//! Sample data shared by the printing tests: English receipt words, a taka
//! currency format, a sample sale, return, Z report, label and shelf tag.
//! Every test builds its own copy, so no test can change another's data.

// Each test file uses only some of these helpers.
#![allow(dead_code, unused_imports)]

mod report_and_tags;

pub use report_and_tags::*;

use std::io::Cursor;

use image::{ImageFormat, Rgba, RgbaImage};
use pos_core::{Bitmap1, CurrencyFormat, Grouping, Money, Qty};
use printing::{
    CustomerOnReceipt, LineDiscount, PaperWidth, PaymentLine, ReceiptData, ReceiptLabels,
    ReceiptLayout, ReceiptLine, ReturnData, ShopHeader, TaxSummaryLine,
};

/// Every printed word in English, as `app` will fill it.
pub fn english_labels() -> ReceiptLabels {
    ReceiptLabels {
        invoice_no: "Invoice".into(),
        date: "Date".into(),
        till: "Till".into(),
        cashier: "Cashier".into(),
        customer: "Customer".into(),
        points: "Points".into(),
        credit_balance: "Balance due".into(),
        subtotal: "Subtotal".into(),
        discount: "Discount".into(),
        rounding: "Rounding".into(),
        total: "Total".into(),
        change: "Change".into(),
        reference: "Ref".into(),
        tax_summary: "Tax summary".into(),
        tax_net: "Net".into(),
        tax_amount: "Tax".into(),
        prices_include_tax: "Prices include {tax}".into(),
        void_heading: "VOID".into(),
        reason: "Reason".into(),
        return_heading: "RETURN".into(),
        return_no: "Return".into(),
        original_invoice: "Original invoice".into(),
        refund_total: "Refund".into(),
        period: "Period".into(),
        date_range: "{from} to {to}".into(),
        packed_on: "Packed".into(),
        expires_on: "Expires".into(),
        unit_price: "{price} per {unit}".into(),
        regular_price: "Was {price}".into(),
        offer_dates: "Offer {from} to {to}".into(),
    }
}

/// Bangladeshi taka: "৳12,34,567.50".
pub fn taka() -> CurrencyFormat {
    CurrencyFormat {
        code: "BDT".into(),
        symbol: "৳".into(),
        symbol_before: true,
        minor_digits: 2,
        decimal_sep: ".".into(),
        group_sep: ",".into(),
        grouping: Grouping::Indian,
    }
}

/// A small shop logo, drawn in colour and turned black and white by `logo_from_image`.
pub fn sample_logo_png() -> Vec<u8> {
    let mut picture = RgbaImage::from_pixel(240, 80, Rgba([255, 255, 255, 0]));
    for y in 0..80u32 {
        for x in 0..240u32 {
            // A dark frame, a grey middle band (dithered) and a dark block on the left.
            let frame = !(4..236).contains(&x) || !(4..76).contains(&y);
            let block = (16..72).contains(&x) && (16..64).contains(&y);
            let band = (90..220).contains(&x) && (30..50).contains(&y);
            if frame || block {
                picture.put_pixel(x, y, Rgba([20, 40, 120, 255]));
            } else if band {
                picture.put_pixel(x, y, Rgba([150, 150, 150, 255]));
            }
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    picture
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("the test logo encodes");
    bytes.into_inner()
}

/// Stripes that stand in for a barcode picture. The real one comes from the
/// `barcode` crate, which `app` calls; printing only places the picture.
pub fn fake_barcode(width: u32, height: u32) -> Bitmap1 {
    let mut picture = Bitmap1::new(width, height);
    let bytes_per_row = width.div_ceil(8) as usize;
    for y in 0..height as usize {
        for x in 0..width as usize {
            // Bars of 2, 3 and 4 dots, so the pattern isn't regular.
            let bar = (x / 3 + x / 7) % 2 == 0;
            if bar {
                picture.bits[y * bytes_per_row + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    picture
}

/// The shop's header and options, as Settings will hold them.
pub fn sample_layout(paper: PaperWidth, logo: Option<Bitmap1>) -> ReceiptLayout {
    ReceiptLayout {
        paper,
        currency: taka(),
        shop: ShopHeader {
            name: "Rahman Super Shop".into(),
            address_lines: vec!["12 Lake Road, Dhanmondi".into(), "Dhaka 1205".into()],
            phone: "01711-000000".into(),
            tax_reg_label: "BIN".into(),
            tax_reg_no: "000123456-0101".into(),
        },
        show_logo: logo.is_some(),
        logo,
        header_lines: vec!["Open every day 8am to 11pm".into()],
        footer_lines: vec!["Thank you for shopping with us".into()],
        show_tax_breakdown: true,
        show_tax_reg_no: true,
        show_invoice_barcode: true,
        show_customer_points: true,
    }
}

fn line(name: &str, qty: i64, unit: &str, unit_price: i64, amount: i64) -> ReceiptLine {
    ReceiptLine {
        name: name.into(),
        qty: Qty(qty),
        unit: unit.into(),
        unit_price: Money(unit_price),
        amount: Money(amount),
        discount: None,
    }
}

/// A sale: a long product name, a weighed item, a promotion and a cash payment.
pub fn sample_sale() -> ReceiptData {
    let mut soap = line("Lux soap 100g", 3000, "", 5000, 15000);
    soap.discount = Some(LineDiscount {
        label: "Buy 2 get 1".into(),
        amount: Money(-5000),
    });
    ReceiptData {
        invoice_no: "T1-000123".into(),
        printed_at: "2026-10-04 14:32".into(),
        till: "T1".into(),
        cashier: "Rina".into(),
        customer: Some(CustomerOnReceipt {
            name: "Karim Ahmed".into(),
            points_balance: Some(240),
            credit_balance: None,
        }),
        lines: vec![
            line("Milk 1L", 2000, "", 9000, 18000),
            line(
                "Fresh Basmati Rice Premium Long Grain Family Pack 5kg",
                1000,
                "",
                65000,
                65000,
            ),
            line("Beef", 1234, "kg", 75000, 92550),
            soap,
        ],
        subtotal: Money(185550),
        discount_total: Money(-5000),
        tax_summary: vec![TaxSummaryLine {
            name: "VAT 15%".into(),
            net: Money(157000),
            tax: Money(23550),
        }],
        tax_label: "VAT".into(),
        prices_include_tax: true,
        rounding: Money(-50),
        total: Money(180500),
        payments: vec![PaymentLine {
            method: "Cash".into(),
            amount: Money(200000),
            reference: None,
        }],
        change: Money(19500),
        void_reason: None,
        invoice_barcode: Some(fake_barcode(300, 60)),
        qr_data: Some("https://example.com/r/T1-000123".into()),
    }
}

/// The same sale, voided (rule RET-04).
pub fn sample_void() -> ReceiptData {
    let mut sale = sample_sale();
    sale.void_reason = Some("Wrong customer".into());
    sale.qr_data = None;
    sale
}

/// A return of one item from the sample sale (rule RET-06).
pub fn sample_return() -> ReturnData {
    ReturnData {
        return_no: "T1-R000045".into(),
        original_invoice_no: Some("T1-000123".into()),
        printed_at: "2026-10-05 10:05".into(),
        till: "T1".into(),
        cashier: "Rina".into(),
        customer: None,
        lines: vec![line("Milk 1L", 1000, "", 9000, 9000)],
        tax_summary: vec![TaxSummaryLine {
            name: "VAT 15%".into(),
            net: Money(7826),
            tax: Money(1174),
        }],
        tax_label: "VAT".into(),
        prices_include_tax: true,
        refund_total: Money(9000),
        refunds: vec![PaymentLine {
            method: "Cash".into(),
            amount: Money(9000),
            reference: None,
        }],
        reason: Some("Damaged pack".into()),
    }
}

/// Is the dot at (x, y) black?
pub fn is_black(picture: &Bitmap1, x: u32, y: u32) -> bool {
    let bytes_per_row = picture.width.div_ceil(8);
    let byte = picture.bits[(y * bytes_per_row + x / 8) as usize];
    byte & (0x80 >> (x % 8)) != 0
}

/// The number of black dots in the picture.
pub fn black_dots(picture: &Bitmap1) -> u32 {
    picture.bits.iter().map(|byte| byte.count_ones()).sum()
}
