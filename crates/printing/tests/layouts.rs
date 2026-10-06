//! What each layout prints: the words, numbers and pictures, and the layout options.
//! How it is drawn is tested in `drawing.rs` and `golden.rs`.

mod common;

use common::*;
use pos_core::Money;
use printing::{
    Block, Doc, PaperWidth, ReceiptLabels, label, receipt, return_receipt, shelf_tag, shift_report,
};

/// All words the doc will print, one block per line, so tests can look for them.
fn printed_text(doc: &Doc) -> String {
    let mut lines = Vec::new();
    for block in &doc.blocks {
        match block {
            Block::Text { text, .. } => lines.push(text.clone()),
            Block::Row { left, right, .. } => lines.push(format!("{left} | {right}")),
            Block::Columns { cells, .. } => lines.push(
                cells
                    .iter()
                    .map(|(text, _, _)| text.as_str())
                    .collect::<Vec<_>>()
                    .join(" | "),
            ),
            Block::Qr { data, .. } => lines.push(format!("[QR {data}]")),
            Block::Image { .. } => lines.push("[image]".into()),
            Block::Barcode { .. } => lines.push("[barcode]".into()),
            Block::Rule { .. } | Block::Feed { .. } => {}
        }
    }
    lines.join("\n")
}

#[test]
fn hw_02_sale_receipt_shows_header_lines_totals_and_payments() {
    let doc = receipt(
        &sample_sale(),
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    );
    let text = printed_text(&doc);
    for expected in [
        "Rahman Super Shop",
        "12 Lake Road, Dhanmondi",
        "Open every day 8am to 11pm",
        "Invoice | T1-000123",
        "Date | 2026-10-04 14:32",
        "Till | T1",
        "Cashier | Rina",
        "Customer | Karim Ahmed",
        "Milk 1L | 180.00",
        "2 × 90.00",
        "1.234 kg × 750.00",
        "Buy 2 get 1 | -50.00",
        "Subtotal | 1,855.50",
        "Discount | -50.00",
        "Rounding | -0.50",
        "Total | ৳1,805.00",
        "Cash | 2,000.00",
        "Change | 195.00",
        "Points | 240",
        "Thank you for shopping with us",
        "[barcode]",
        "[QR https://example.com/r/T1-000123]",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
    assert!(
        !text.contains("1 × 650.00"),
        "no quantity line for one item"
    );
}

#[test]
fn tax_08_receipt_shows_registration_tax_summary_and_inclusive_note() {
    let doc = receipt(
        &sample_sale(),
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    );
    let text = printed_text(&doc);
    for expected in [
        "BIN: 000123456-0101",
        "Tax summary",
        " | Net | Tax",
        "VAT 15% | 1,570.00 | 235.50",
        "Prices include VAT",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn set_14_layout_options_hide_parts_of_the_receipt() {
    let mut layout = sample_layout(PaperWidth::Mm80, Some(fake_barcode(64, 32)));
    let with_everything = printed_text(&receipt(&sample_sale(), &layout, &english_labels()));
    assert!(with_everything.contains("[image]"));

    layout.show_logo = false;
    layout.show_tax_breakdown = false;
    layout.show_tax_reg_no = false;
    layout.show_invoice_barcode = false;
    layout.show_customer_points = false;
    let text = printed_text(&receipt(&sample_sale(), &layout, &english_labels()));
    assert!(!text.contains("[image]"));
    assert!(!text.contains("Tax summary"));
    assert!(!text.contains("BIN"));
    assert!(!text.contains("[barcode]"));
    assert!(!text.contains("Points"));
    assert!(
        text.contains("Prices include VAT"),
        "always shown in inclusive mode"
    );
}

#[test]
fn zero_discount_rounding_and_change_are_left_out() {
    let mut sale = sample_sale();
    sale.discount_total = Money(0);
    sale.rounding = Money(0);
    sale.change = Money(0);
    let text = printed_text(&receipt(
        &sale,
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    ));
    assert!(!text.contains("Discount |"));
    assert!(!text.contains("Rounding"));
    assert!(!text.contains("Change"));
}

#[test]
fn ret_04_void_receipt_has_a_void_heading_and_reason() {
    let text = printed_text(&receipt(
        &sample_void(),
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    ));
    assert!(text.contains("VOID"));
    assert!(text.contains("Reason: Wrong customer"));
    assert!(text.contains("Invoice | T1-000123"), "keeps its number");
}

#[test]
fn pay_10_a_reprint_is_marked_copy_and_the_original_is_not() {
    let layout = sample_layout(PaperWidth::Mm80, None);
    let original = printed_text(&receipt(&sample_sale(), &layout, &english_labels()));
    assert!(!original.contains("COPY"));
    let mut reprint = sample_sale();
    reprint.copy = true;
    let text = printed_text(&receipt(&reprint, &layout, &english_labels()));
    assert!(text.contains("COPY"));
    assert!(text.contains("Invoice | T1-000123"), "the same sale");
}

#[test]
fn ret_06_return_receipt_shows_heading_numbers_lines_and_refund() {
    let text = printed_text(&return_receipt(
        &sample_return(),
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    ));
    for expected in [
        "RETURN",
        "Return | T1-R000045",
        "Original invoice | T1-000123",
        "Milk 1L | 90.00",
        "Refund | ৳90.00",
        "Cash | 90.00",
        "Reason: Damaged pack",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn shf_08_z_report_shows_title_period_filters_rows_totals_and_sections() {
    let text = printed_text(&shift_report(
        &sample_z_report(),
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    ));
    for expected in [
        "Rahman Super Shop",
        "Z report",
        "Period | 2026-10-04 to 2026-10-04",
        "Till | T1",
        "Cashier | Rina",
        "Item | Amount",
        "Gross sales | 54,321.00",
        "Net sales | 54,108.00",
        "Counts",
        "Cancelled carts | 4",
        "Tax by rate",
        "VAT | 15% | 7,057.57",
        "Difference | -10.00",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn cat_12_label_shows_name_price_unit_price_barcode_and_dates() {
    let text = printed_text(&label(
        &sample_label(),
        &label_template(58, 40),
        &english_labels(),
    ));
    for expected in [
        "Beef, boneless",
        "৳925.50",
        "৳750.00 per kg",
        "[barcode]",
        "2100123012343",
        "Packed: 2026-10-04 | Expires: 2026-10-07",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn cat_12_label_template_hides_fields() {
    let mut template = label_template(58, 40);
    template.show_unit_price = false;
    template.show_barcode = false;
    template.show_packed_on = false;
    template.show_expires_on = false;
    let text = printed_text(&label(&sample_label(), &template, &english_labels()));
    assert!(text.contains("Beef, boneless"));
    assert!(!text.contains("per kg"));
    assert!(!text.contains("[barcode]"));
    assert!(!text.contains("Packed"));
    assert!(!text.contains("Expires"));
}

#[test]
fn cat_13_shelf_tag_shows_promotion_price_old_price_and_dates() {
    let text = printed_text(&shelf_tag(
        &sample_shelf_tag(),
        &tag_template(80, 40),
        &english_labels(),
    ));
    for expected in [
        "Teer Soybean Oil 5L",
        "৳850.00",
        "Was ৳920.00",
        "Offer 2026-10-01 to 2026-10-15",
        "৳170.00 per L",
        "8941100500019",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn cat_13_shelf_tag_without_promotion_shows_the_price_only() {
    let mut tag = sample_shelf_tag();
    tag.promotion = None;
    let text = printed_text(&shelf_tag(&tag, &tag_template(80, 40), &english_labels()));
    assert!(text.contains("৳920.00"));
    assert!(!text.contains("Was"));
    assert!(!text.contains("Offer"));
}

/// Words in another "language": every word starts with "~".
fn marked_labels() -> ReceiptLabels {
    let english = english_labels();
    let mark = |word: &String| format!("~{word}");
    ReceiptLabels {
        invoice_no: mark(&english.invoice_no),
        date: mark(&english.date),
        till: mark(&english.till),
        cashier: mark(&english.cashier),
        customer: mark(&english.customer),
        points: mark(&english.points),
        credit_balance: mark(&english.credit_balance),
        subtotal: mark(&english.subtotal),
        discount: mark(&english.discount),
        rounding: mark(&english.rounding),
        total: mark(&english.total),
        change: mark(&english.change),
        reference: mark(&english.reference),
        tax_summary: mark(&english.tax_summary),
        tax_net: mark(&english.tax_net),
        tax_amount: mark(&english.tax_amount),
        prices_include_tax: mark(&english.prices_include_tax),
        void_heading: mark(&english.void_heading),
        copy_heading: mark(&english.copy_heading),
        reason: mark(&english.reason),
        return_heading: mark(&english.return_heading),
        return_no: mark(&english.return_no),
        original_invoice: mark(&english.original_invoice),
        refund_total: mark(&english.refund_total),
        period: mark(&english.period),
        date_range: mark(&english.date_range),
        packed_on: mark(&english.packed_on),
        expires_on: mark(&english.expires_on),
        unit_price: mark(&english.unit_price),
        regular_price: mark(&english.regular_price),
        offer_dates: mark(&english.offer_dates),
    }
}

#[test]
fn adr_022_every_printed_word_comes_from_receipt_labels() {
    let labels = marked_labels();
    let layout = sample_layout(PaperWidth::Mm80, None);
    let mut void = sample_void();
    void.customer.as_mut().unwrap().credit_balance = Some(Money(1000));
    void.copy = true;
    // The report's filter names are data from `reports`; mark them too.
    let mut z_report = sample_z_report();
    for filter in &mut z_report.params.filters {
        filter.name = format!("~{}", filter.name);
    }
    let printed = [
        printed_text(&receipt(&void, &layout, &labels)),
        printed_text(&return_receipt(&sample_return(), &layout, &labels)),
        printed_text(&shift_report(&z_report, &layout, &labels)),
        printed_text(&label(&sample_label(), &label_template(58, 40), &labels)),
        printed_text(&shelf_tag(
            &sample_shelf_tag(),
            &tag_template(80, 40),
            &labels,
        )),
    ]
    .join("\n");
    let english = english_labels();
    for word in [
        &english.invoice_no,
        &english.till,
        &english.cashier,
        &english.subtotal,
        &english.total,
        &english.change,
        &english.points,
        &english.credit_balance,
        &english.tax_summary,
        &english.void_heading,
        &english.copy_heading,
        &english.return_heading,
        &english.refund_total,
        &english.period,
        &english.packed_on,
        &english.expires_on,
    ] {
        for line in printed.lines() {
            if let Some(at) = line.find(word.as_str()) {
                assert!(
                    at > 0 && line[..at].ends_with('~'),
                    "{word:?} is typed into a layout: {line:?}"
                );
            }
        }
    }
}
