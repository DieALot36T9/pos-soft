//! Variants: every combination of a parent's attributes becomes its own product (rule CAT-06).

use pos_core::{Id, Timestamp};
use rusqlite::{Transaction, named_params};

use crate::error::{CatalogError, invalid};
use crate::products::{VariantOf, insert_product};
use crate::read::get;
use crate::types::{NewProduct, ProductKind};

/// Makes one variant for each combination of attribute values, copying the parent's
/// fields (they can be edited afterwards). Combinations that already exist are skipped,
/// so adding a value and generating again only adds the new ones. Returns the new ids.
///
/// Example: Size [200 ml, 400 ml] × Flavour [Mango, Lemon] makes four variants,
/// the first named "Shampoo 200 ml Mango" with the label "200 ml / Mango".
pub fn generate_variants(
    tx: &Transaction,
    parent_id: &Id,
    attributes: &[(String, Vec<String>)],
    now: Timestamp,
) -> Result<Vec<Id>, CatalogError> {
    let parent = get(tx, parent_id)?;
    if parent.kind != ProductKind::VariantParent {
        return Err(invalid("parent", "not_a_variant_parent"));
    }
    let attributes = clean_attributes(attributes)?;
    let existing_labels: Vec<String> = tx
        .prepare("SELECT variant_label FROM catalog_products WHERE parent_id = :id")?
        .query_map(named_params! { ":id": parent_id.as_str() }, |row| {
            Ok(row.get::<_, Option<String>>(0)?.unwrap_or_default())
        })?
        .collect::<Result<_, _>>()?;

    let template = NewProduct {
        kind: ProductKind::Variant,
        category_id: Some(parent.category_id.clone()),
        brand_id: parent.brand_id.clone(),
        sale_price: parent.sale_price,
        cost_price: parent.cost_price,
        is_weighed: parent.is_weighed,
        track_expiry: parent.track_expiry,
        preferred_supplier_id: parent.preferred_supplier_id.clone(),
        description: parent.description.clone(),
        ..NewProduct::new(
            ProductKind::Variant,
            "",
            parent.unit_id.clone(),
            parent.tax_rate_id.clone(),
        )
    };

    let mut created = Vec::new();
    for combination in combinations(&attributes) {
        let values: Vec<&str> = combination
            .iter()
            .map(|(_, value)| value.as_str())
            .collect();
        let label = values.join(" / ");
        if existing_labels.contains(&label) {
            continue;
        }
        let variant = NewProduct {
            name: format!("{} {}", parent.name, values.join(" ")),
            ..template.clone()
        };
        let variant_of = VariantOf {
            parent_id,
            label: &label,
        };
        let variant_id = insert_product(tx, &variant, Some(variant_of), now)?;
        save_attributes(tx, &variant_id, &combination)?;
        created.push(variant_id);
    }
    Ok(created)
}

/// Trims names and values. Each attribute needs a name and at least one value,
/// and no attribute may be listed twice.
fn clean_attributes(
    attributes: &[(String, Vec<String>)],
) -> Result<Vec<(String, Vec<String>)>, CatalogError> {
    let mut cleaned: Vec<(String, Vec<String>)> = Vec::new();
    for (name, values) in attributes {
        let name = name.trim().to_string();
        let values: Vec<String> = values
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if name.is_empty() || values.is_empty() {
            return Err(invalid("attributes", "variant_attribute_missing"));
        }
        if cleaned.iter().any(|(earlier, _)| *earlier == name) {
            return Err(invalid("attributes", "variant_attribute_repeated"));
        }
        cleaned.push((name, values));
    }
    if cleaned.is_empty() {
        return Err(invalid("attributes", "variant_attribute_missing"));
    }
    Ok(cleaned)
}

/// Every combination of one value per attribute, first attribute changing slowest.
/// Example: [Size: 200, 400] × [Flavour: Mango, Lemon] →
/// (200, Mango), (200, Lemon), (400, Mango), (400, Lemon).
fn combinations(attributes: &[(String, Vec<String>)]) -> Vec<Vec<(String, String)>> {
    let mut result: Vec<Vec<(String, String)>> = vec![Vec::new()];
    for (name, values) in attributes {
        let mut longer = Vec::new();
        for start in &result {
            for value in values {
                let mut combination = start.clone();
                combination.push((name.clone(), value.clone()));
                longer.push(combination);
            }
        }
        result = longer;
    }
    result
}

fn save_attributes(
    tx: &Transaction,
    product_id: &Id,
    combination: &[(String, String)],
) -> Result<(), CatalogError> {
    let mut insert = tx.prepare_cached(
        "INSERT INTO catalog_product_attributes (product_id, name, value) VALUES (:id, :name, :value)",
    )?;
    for (name, value) in combination {
        insert.execute(
            named_params! { ":id": product_id.as_str(), ":name": name, ":value": value },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attribute(name: &str, values: &[&str]) -> (String, Vec<String>) {
        (
            name.to_string(),
            values.iter().map(|value| value.to_string()).collect(),
        )
    }

    #[test]
    fn combinations_are_the_cartesian_product_in_order() {
        let found = combinations(&[
            attribute("Size", &["200", "400"]),
            attribute("Flavour", &["Mango", "Lemon"]),
        ]);
        let values: Vec<Vec<&str>> = found
            .iter()
            .map(|combination| {
                combination
                    .iter()
                    .map(|(_, value)| value.as_str())
                    .collect()
            })
            .collect();
        assert_eq!(
            values,
            vec![
                vec!["200", "Mango"],
                vec!["200", "Lemon"],
                vec!["400", "Mango"],
                vec!["400", "Lemon"]
            ]
        );
    }

    #[test]
    fn attributes_need_a_name_a_value_and_no_repeats() {
        assert!(clean_attributes(&[]).is_err());
        assert!(clean_attributes(&[attribute(" ", &["a"])]).is_err());
        assert!(clean_attributes(&[attribute("Size", &[" "])]).is_err());
        let repeated = [attribute("Size", &["a"]), attribute("Size", &["b"])];
        assert_eq!(
            clean_attributes(&repeated),
            Err(invalid("attributes", "variant_attribute_repeated"))
        );
    }
}
