//! Barcode labels and shelf tags (CAT-12, CAT-13): choosing products, the preview
//! picture, printing to the Preview printer, and the promotion price on a shelf tag.

mod common;

use app::commands::labels as commands;
use app::state::AppState;
use app::workflows::labels::{LabelCopies, LabelKind, LabelPick, LabelRequest, LabelSettings};
use app::workflows::pricing_admin::PromotionSave;
use app::workflows::receiving::{GrnInput, GrnLineInput};
use base64::Engine;
use common::*;
use pos_core::{AppError, Bps, Id, Money, Qty};
use pricing::{EVERY_DAY, PromoRule, PromoTarget, PromotionInput};

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    (folder, state)
}

fn settings(kind: LabelKind, width_mm: u32, height_mm: u32) -> LabelSettings {
    LabelSettings {
        kind,
        width_mm,
        height_mm,
        gap_mm: 2,
        show_name: true,
        show_price: true,
        show_unit_price: true,
        show_barcode: true,
        packed_on: None,
        expires_on: None,
    }
}

fn one(product: &Id, copies: u32, settings: LabelSettings) -> LabelRequest {
    LabelRequest {
        settings,
        items: vec![LabelCopies {
            product_id: product.clone(),
            copies,
        }],
    }
}

fn picture(state: &AppState, request: &LabelRequest) -> pos_core::Bitmap1 {
    let pictures = commands::labels_preview(state, devices(), request).expect("preview");
    let data = pictures[0]
        .picture
        .strip_prefix("data:image/png;base64,")
        .expect("a PNG data address");
    let png = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("base64");
    picture_from_png(&png)
}

#[test]
fn cat_12_products_are_picked_by_hand_by_category_or_by_price_change() {
    let (_folder, state) = shop();
    let milk = till_product(
        &state,
        "Milk",
        "8901234567890",
        9000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let rice = till_product(&state, "Rice", "", 6000, catalog::UNIT_KG, true, None);

    let picked = commands::labels_pick(
        &state,
        &LabelPick::Products {
            ids: vec![milk.clone(), rice.clone(), milk.clone()],
        },
    )
    .expect("picked");
    let copies: Vec<(Id, u32)> = picked
        .items
        .iter()
        .map(|item| (item.product_id.clone(), item.copies))
        .collect();
    assert_eq!(copies, vec![(milk.clone(), 2), (rice.clone(), 1)]);

    let everything = commands::labels_pick(
        &state,
        &LabelPick::Category {
            id: Id::parse(catalog::UNCATEGORISED).expect("id"),
        },
    )
    .expect("category");
    assert_eq!(everything.items.len(), 2);

    // Nothing changed price since today; a new price shows up.
    let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 5).expect("date");
    let changed = LabelPick::PriceChangedSince { date: today };
    assert!(
        commands::labels_pick(&state, &changed)
            .expect("none")
            .items
            .is_empty()
    );
    let mut form = app::commands::catalog::product_get(&state, &rice)
        .expect("rice")
        .product;
    form.sale_price = Money(6500);
    let input = app::workflows::products::ProductInput {
        id: Some(rice.clone()),
        kind: form.kind,
        name: form.name,
        sku: form.sku,
        category_id: Some(form.category_id),
        brand_id: form.brand_id,
        unit_id: form.unit_id,
        sale_price: form.sale_price,
        cost_price: form.cost_price,
        tax_rate_id: Some(form.tax_rate_id),
        is_weighed: form.is_weighed,
        scale_item_code: form.scale_item_code,
        track_stock: form.track_stock,
        track_expiry: form.track_expiry,
        preferred_supplier_id: form.preferred_supplier_id,
        description: form.description,
        barcodes: form.barcodes,
        reorder: None,
    };
    app::commands::catalog::product_save(&state, input).expect("new price");
    let picked = commands::labels_pick(&state, &changed).expect("changed");
    assert_eq!(picked.items.len(), 1);
    assert_eq!(picked.items[0].product_id, rice);
}

#[test]
fn cat_12_a_delivery_gives_one_label_per_piece_received() {
    let (_folder, state) = shop();
    let milk = till_product(
        &state,
        "Milk",
        "8901234567890",
        9000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let rice = till_product(&state, "Rice", "", 6000, catalog::UNIT_KG, true, None);
    // As many labels as pieces received (2.5 kg → 3).
    let supplier = app::commands::purchasing::supplier_save(
        &state,
        &app::workflows::purchasing_admin::SupplierSave {
            id: None,
            input: purchasing::SupplierInput {
                name: "Rahim Traders".into(),
                ..purchasing::SupplierInput::default()
            },
        },
    )
    .expect("supplier");
    let grn = app::commands::purchasing::grn_save(
        &state,
        &GrnInput {
            supplier_id: supplier,
            order_id: None,
            supplier_invoice_no: None,
            invoice_total: Money(120_000),
            notes: None,
            lines: vec![
                GrnLineInput {
                    order_line_id: None,
                    product_id: milk.clone(),
                    qty: Qty(12_000),
                    unit_cost: Money(7_500),
                    batch_no: None,
                    expiry_date: None,
                },
                GrnLineInput {
                    order_line_id: None,
                    product_id: rice.clone(),
                    qty: Qty(2_500),
                    unit_cost: Money(12_000),
                    batch_no: None,
                    expiry_date: None,
                },
            ],
        },
    )
    .expect("delivery");
    let picked =
        commands::labels_pick(&state, &LabelPick::Delivery { id: grn.grn_id }).expect("delivery");
    let copies: Vec<u32> = picked.items.iter().map(|item| item.copies).collect();
    assert_eq!(copies, vec![12, 3]);
}

#[test]
fn cat_12_label_preview_is_the_printed_picture() {
    let (_folder, state) = shop();
    let milk = till_product(
        &state,
        "Milk 1L",
        "8901234567890",
        9000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let request = one(&milk, 1, settings(LabelKind::Label, 58, 40));
    let label = picture(&state, &request);
    assert_eq!(label.width, 58 * 8);
    check_golden("label_58x40mm", &label);
}

#[test]
fn cat_12_printing_without_a_label_printer_goes_to_the_preview_printer() {
    let (_folder, state) = shop();
    let milk = till_product(
        &state,
        "Milk 1L",
        "8901234567890",
        9000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    devices().preview.take_jobs();
    let printed = commands::labels_print(
        &state,
        devices(),
        &one(&milk, 3, settings(LabelKind::Label, 58, 40)),
    )
    .expect("printed");
    assert_eq!(printed.labels, 3);
    assert!(printed.to_preview);
    let jobs = devices().preview.take_jobs();
    assert_eq!(jobs.len(), 3);
}

#[test]
fn cat_12_sizes_and_copies_out_of_range_are_refused() {
    let (_folder, state) = shop();
    let milk = till_product(&state, "Milk", "", 9000, catalog::UNIT_PIECE, false, None);
    let tiny = one(&milk, 1, settings(LabelKind::Label, 5, 40));
    let refused = commands::labels_preview(&state, devices(), &tiny).expect_err("too small");
    assert!(
        matches!(&refused, AppError::Validation { code, .. } if code == "label_size_out_of_range"),
        "{refused:?}"
    );
    let none = one(&milk, 0, settings(LabelKind::Label, 58, 40));
    let refused = commands::labels_print(&state, devices(), &none).expect_err("no copies");
    assert!(
        matches!(&refused, AppError::Validation { code, .. } if code == "label_copies_out_of_range"),
        "{refused:?}"
    );
}

#[test]
fn cat_13_shelf_tag_shows_the_promotion_price() {
    let (_folder, state) = shop();
    let oil = till_product(
        &state,
        "Oil 1L",
        "8901234567890",
        20_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let tag = one(&oil, 1, settings(LabelKind::ShelfTag, 80, 40));
    let plain = picture(&state, &tag);

    let promotion = PromotionSave {
        id: None,
        promotion: PromotionInput {
            name: "Oil week".into(),
            rule: PromoRule::PercentOff { percent: Bps(1000) },
            starts_at: None,
            ends_at: Some(pos_core::Timestamp(START_MS + 7 * 86_400_000)),
            days_mask: EVERY_DAY,
            tier_id: None,
            targets: vec![PromoTarget::Product(oil.clone())],
        },
    };
    app::commands::pricing::promotion_save(&state, &promotion).expect("promotion");
    let offer = picture(&state, &tag);
    assert_ne!(plain, offer, "the tag shows the offer");
    check_golden("shelf_tag_80x40mm", &offer);
}
