//! Sales, returns and their payments of the fixture shop (see `mod.rs` for the script).

use rusqlite::{Connection, named_params};

use super::{id, local_ms};

/// One sale line as the fixture writes it. Amounts in minor units.
pub struct Line {
    pub tag: &'static str,
    pub product: &'static str,
    pub qty: i64,
    pub base: i64,
    pub promo: i64,
    pub manual: i64,
    pub net: i64,
    pub tax: i64,
    pub unit_cost: i64,
    pub overridden: bool,
}

/// Standard 15% is split into CGST 7.5% + SGST 7.5% (TAX-03). Exempt has no parts.
fn rate_of(product: &str) -> (&'static str, &'static str, i64) {
    match product {
        "P2" | "P4" => ("TR0", "Exempt 0%", 0),
        _ => ("TR15", "Standard 15%", 1_500),
    }
}

fn product_facts(
    product: &str,
) -> (
    &'static str,
    &'static str,
    Option<&'static str>,
    &'static str,
) {
    match product {
        "P1" => ("Milk 1L", "CD", Some("BA"), "pc"),
        "P2" => ("Rice", "CG", None, "kg"),
        _ => ("Soap", "CH", Some("BX"), "pc"),
    }
}

/// Splits a line's tax like the tax module: each part rounded towards zero, the last gets the rest.
fn components_json(rate_bps: i64, tax: i64) -> String {
    if rate_bps == 0 {
        return "[]".to_string();
    }
    let first = tax * 750 / rate_bps;
    let second = tax - first;
    format!(
        r#"[{{"name":"CGST","bps":750,"tax_minor":{first}}},{{"name":"SGST","bps":750,"tax_minor":{second}}}]"#
    )
}

pub struct Sale {
    pub tag: &'static str,
    pub invoice_no: &'static str,
    pub till: &'static str,
    pub shift: &'static str,
    pub user: &'static str,
    pub date: &'static str,
    pub at: i64,
    pub rounding: i64,
    pub voided: bool,
}

pub fn add_sale(conn: &Connection, sale: &Sale, lines: &[Line]) {
    let mut sums = [0_i64; 6]; // base, promo, manual, net, tax, cost
    for line in lines {
        sums[0] += line.base;
        sums[1] += line.promo;
        sums[2] += line.manual;
        sums[3] += line.net;
        sums[4] += line.tax;
        sums[5] += line.unit_cost * line.qty / 1000;
    }
    let gross = sums[3] + sums[4];
    conn.execute(
        "INSERT INTO sales_sales (id, invoice_no, till_id, shift_id, user_id, business_date, completed_at,
           status, base_minor, promo_discount_minor, tier_discount_minor, manual_discount_minor,
           invoice_discount_minor, net_minor, tax_minor, gross_minor, rounding_minor, total_minor,
           change_minor, cost_minor, tax_mode)
         VALUES (:id, :no, :till, :shift, :user, :date, :at, 'completed', :base, :promo, 0, :manual, 0,
           :net, :tax, :gross, :rounding, :total, 0, :cost, 'inclusive')",
        named_params! {
            ":id": id(sale.tag).as_str(), ":no": sale.invoice_no, ":till": id(sale.till).as_str(),
            ":shift": id(sale.shift).as_str(), ":user": id(sale.user).as_str(), ":date": sale.date,
            ":at": sale.at, ":base": sums[0], ":promo": sums[1], ":manual": sums[2], ":net": sums[3],
            ":tax": sums[4], ":gross": gross, ":rounding": sale.rounding,
            ":total": gross + sale.rounding, ":cost": sums[5],
        },
    )
    .unwrap();
    for (seq, line) in lines.iter().enumerate() {
        add_sale_line(conn, sale.tag, seq, line);
    }
    if sale.voided {
        conn.execute(
            "UPDATE sales_sales SET status = 'voided', voided_at = :at, voided_by = :user,
               void_approved_by = :admin, void_reason = 'Wrong item' WHERE id = :id",
            named_params! {
                ":at": sale.at + 600_000, ":user": id(sale.user).as_str(),
                ":admin": id("A").as_str(), ":id": id(sale.tag).as_str(),
            },
        )
        .unwrap();
    }
}

fn add_sale_line(conn: &Connection, sale: &str, seq: usize, line: &Line) {
    let (rate, rate_name, rate_bps) = rate_of(line.product);
    let (name, category, brand, unit) = product_facts(line.product);
    conn.execute(
        "INSERT INTO sales_lines (id, sale_id, seq, product_id, name, sku, unit_symbol, category_id,
           brand_id, qty_milli, unit_price_minor, retail_price_minor, base_minor, promo_discount_minor,
           tier_discount_minor, manual_discount_minor, invoice_discount_minor, net_minor, tax_minor,
           gross_minor, tax_rate_id, tax_name, tax_bps, tax_components_json, unit_cost_minor,
           cost_minor, price_overridden)
         VALUES (:id, :sale, :seq, :product, :name, :sku, :unit, :category, :brand, :qty, :price,
           :price, :base, :promo, 0, :manual, 0, :net, :tax, :gross, :rate, :rate_name, :bps,
           :components, :unit_cost, :cost, :overridden)",
        named_params! {
            ":id": id(line.tag).as_str(), ":sale": id(sale).as_str(), ":seq": seq as i64 + 1,
            ":product": id(line.product).as_str(), ":name": name, ":sku": line.product,
            ":unit": unit, ":category": id(category).as_str(),
            ":brand": brand.map(|brand| id(brand).to_string()), ":qty": line.qty,
            ":price": line.base * 1000 / line.qty, ":base": line.base, ":promo": line.promo,
            ":manual": line.manual, ":net": line.net, ":tax": line.tax,
            ":gross": line.net + line.tax, ":rate": id(rate).as_str(), ":rate_name": rate_name,
            ":bps": rate_bps, ":components": components_json(rate_bps, line.tax),
            ":unit_cost": line.unit_cost, ":cost": line.unit_cost * line.qty / 1000,
            ":overridden": line.overridden,
        },
    )
    .unwrap();
}

pub struct Return {
    pub tag: &'static str,
    pub return_no: &'static str,
    pub sale: &'static str,
    pub sale_line: &'static str,
    pub product: &'static str,
    pub shift: &'static str,
    pub date: &'static str,
    pub at: i64,
    pub qty: i64,
    pub net: i64,
    pub tax: i64,
    pub unit_cost: i64,
}

/// A return of one line, made by Rina on till T1.
pub fn add_return(conn: &Connection, ret: &Return) {
    let (rate, _, rate_bps) = rate_of(ret.product);
    let (name, ..) = product_facts(ret.product);
    let gross = ret.net + ret.tax;
    let cost = ret.unit_cost * ret.qty / 1000;
    conn.execute(
        "INSERT INTO sales_returns (id, return_no, sale_id, till_id, shift_id, user_id, business_date,
           at, net_minor, tax_minor, gross_minor, rounding_minor, total_minor, cost_minor)
         VALUES (:id, :no, :sale, :till, :shift, :user, :date, :at, :net, :tax, :gross, 0, :gross, :cost)",
        named_params! {
            ":id": id(ret.tag).as_str(), ":no": ret.return_no, ":sale": id(ret.sale).as_str(),
            ":till": id("T1").as_str(), ":shift": id(ret.shift).as_str(), ":user": id("R").as_str(),
            ":date": ret.date, ":at": ret.at, ":net": ret.net, ":tax": ret.tax, ":gross": gross,
            ":cost": cost,
        },
    )
    .unwrap();
    conn.execute(
        "INSERT INTO sales_return_lines (id, return_id, sale_line_id, product_id, name, qty_milli,
           net_minor, tax_minor, gross_minor, tax_rate_id, tax_bps, unit_cost_minor, cost_minor, restock)
         VALUES (:id, :ret, :line, :product, :name, :qty, :net, :tax, :gross, :rate, :bps, :unit_cost, :cost, 1)",
        named_params! {
            ":id": id(&format!("{}X", ret.tag)).as_str(), ":ret": id(ret.tag).as_str(),
            ":line": id(ret.sale_line).as_str(), ":product": id(ret.product).as_str(), ":name": name,
            ":qty": ret.qty, ":net": ret.net, ":tax": ret.tax, ":gross": gross,
            ":rate": id(rate).as_str(), ":bps": rate_bps, ":unit_cost": ret.unit_cost, ":cost": cost,
        },
    )
    .unwrap();
}

/// One payment of a sale or return. `method` is "1" (cash) or "CARD".
pub fn add_payment(
    conn: &Connection,
    doc_type: &str,
    doc: &str,
    method: &str,
    amount: i64,
    at: i64,
) {
    let kind = if method == "1" { "cash" } else { "card" };
    conn.execute(
        "INSERT INTO sales_payments (id, doc_type, doc_id, method_id, kind, amount_minor, at)
         VALUES (:id, :doc_type, :doc, :method, :kind, :amount, :at)",
        named_params! {
            ":id": id(&format!("PY{doc}")).as_str(), ":doc_type": doc_type, ":doc": id(doc).as_str(),
            ":method": id(method).as_str(), ":kind": kind, ":amount": amount, ":at": at,
        },
    )
    .unwrap();
}

pub fn add_sales(conn: &Connection) {
    conn.execute(
        "INSERT INTO sales_payment_methods (id, name, kind) VALUES (:id, 'Card', 'card')",
        named_params! { ":id": id("CARD").as_str() },
    )
    .unwrap();
    let rina = |tag, invoice_no, at| Sale {
        tag,
        invoice_no,
        till: "T1",
        shift: "SH1",
        user: "R",
        date: "2026-03-05",
        at,
        rounding: 0,
        voided: false,
    };
    let milk = |tag, qty, base, manual, net, tax| Line {
        tag,
        product: "P1",
        qty,
        base,
        promo: 0,
        manual,
        net,
        tax,
        unit_cost: 8_000,
        overridden: false,
    };
    let soap = |tag, promo, net, tax| Line {
        tag,
        product: "P3",
        qty: 1_000,
        base: 4_000,
        promo,
        manual: 0,
        net,
        tax,
        unit_cost: 2_500,
        overridden: false,
    };
    let rice = |tag, qty, base, overridden| Line {
        tag,
        product: "P2",
        qty,
        base,
        promo: 0,
        manual: 0,
        net: base,
        tax: 0,
        unit_cost: 4_000,
        overridden,
    };

    let first = rina("S1", "T1-000001", local_ms(0, 10, 15));
    add_sale(
        conn,
        &first,
        &[
            milk("S1A", 2_000, 20_000, 0, 17_391, 2_609),
            rice("S1B", 1_000, 5_000, false),
        ],
    );
    add_payment(conn, "sale", "S1", "1", 25_000, first.at);

    let second = rina("S2", "T1-000002", local_ms(0, 11, 30));
    add_sale(
        conn,
        &second,
        &[
            soap("S2A", 500, 3_043, 457),
            milk("S2B", 1_000, 10_000, 1_000, 7_826, 1_174),
        ],
    );
    add_payment(conn, "sale", "S2", "CARD", 12_500, second.at);

    let third = Sale {
        rounding: 3,
        ..rina("S3", "T1-000003", local_ms(0, 12, 0))
    };
    add_sale(conn, &third, &[rice("S3A", 3_000, 13_497, true)]);
    add_payment(conn, "sale", "S3", "1", 13_500, third.at);

    let fourth = Sale {
        voided: true,
        ..rina("S4", "T1-000004", local_ms(0, 13, 0))
    };
    add_sale(conn, &fourth, &[soap("S4A", 0, 3_478, 522)]);
    add_payment(conn, "sale", "S4", "1", 4_000, fourth.at);

    let karim = Sale {
        tag: "S6",
        invoice_no: "T2-000001",
        till: "T2",
        shift: "SH3",
        user: "K",
        ..rina("S6", "T2-000001", local_ms(0, 16, 0))
    };
    add_sale(conn, &karim, &[soap("S6A", 0, 3_478, 522)]);
    add_payment(conn, "sale", "S6", "1", 4_000, karim.at);
}

/// The two returns: one milk bottle on 03-05 and the soap on 03-06.
pub fn add_returns(conn: &Connection) {
    add_return(
        conn,
        &Return {
            tag: "RT1",
            return_no: "T1-R000001",
            sale: "S1",
            sale_line: "S1A",
            product: "P1",
            shift: "SH1",
            date: "2026-03-05",
            at: local_ms(0, 15, 0),
            qty: 1_000,
            net: 8_696,
            tax: 1_304,
            unit_cost: 8_000,
        },
    );
    add_payment(conn, "return", "RT1", "1", 10_000, local_ms(0, 15, 0));
    add_return(
        conn,
        &Return {
            tag: "RT2",
            return_no: "T1-R000002",
            sale: "S2",
            sale_line: "S2A",
            product: "P3",
            shift: "SH2",
            date: "2026-03-06",
            at: local_ms(1, 10, 0),
            qty: 1_000,
            net: 3_043,
            tax: 457,
            unit_cost: 2_500,
        },
    );
    add_payment(conn, "return", "RT2", "CARD", 3_500, local_ms(1, 10, 0));
}
