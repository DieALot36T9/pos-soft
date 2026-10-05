//! Kits: a bundle of other products with its own price and no stock (rules CAT-07, CHK-14).

use pos_core::{Id, Qty};
use rusqlite::{Connection, Transaction, named_params};

use crate::error::{CatalogError, invalid};
use crate::read::get;
use crate::rows::read_id;
use crate::types::ProductKind;

/// A kit's components and how many of each go in one kit.
/// Example: Eid Box = 2 × Sugar 1kg + 1 × Oil 1L.
pub fn kit_components(
    connection: &Connection,
    kit_id: &Id,
) -> Result<Vec<(Id, Qty)>, CatalogError> {
    let components = connection
        .prepare_cached(
            "SELECT component_id, qty_milli FROM catalog_kit_components
             WHERE kit_id = :kit_id ORDER BY rowid",
        )?
        .query_map(named_params! { ":kit_id": kit_id.as_str() }, |row| {
            Ok((read_id(row, 0)?, Qty(row.get(1)?)))
        })?
        .collect::<Result<_, _>>()?;
    Ok(components)
}

/// Replaces a kit's components. A kit can't contain itself or another kit, each
/// component is listed once, and every quantity is above zero.
pub fn set_kit_components(
    tx: &Transaction,
    kit_id: &Id,
    components: &[(Id, Qty)],
) -> Result<(), CatalogError> {
    if get(tx, kit_id)?.kind != ProductKind::Kit {
        return Err(invalid("kit", "not_a_kit"));
    }
    for (index, (component_id, qty)) in components.iter().enumerate() {
        check_component(tx, kit_id, component_id, *qty)?;
        let listed_before = components[..index]
            .iter()
            .any(|(earlier, _)| earlier == component_id);
        if listed_before {
            return Err(invalid("components", "kit_component_repeated"));
        }
    }

    tx.execute(
        "DELETE FROM catalog_kit_components WHERE kit_id = :kit_id",
        named_params! { ":kit_id": kit_id.as_str() },
    )?;
    let mut insert = tx.prepare_cached(
        "INSERT INTO catalog_kit_components (kit_id, component_id, qty_milli)
         VALUES (:kit_id, :component_id, :qty)",
    )?;
    for (component_id, qty) in components {
        insert.execute(named_params! {
            ":kit_id": kit_id.as_str(),
            ":component_id": component_id.as_str(),
            ":qty": qty.0,
        })?;
    }
    Ok(())
}

fn check_component(
    tx: &Transaction,
    kit_id: &Id,
    component_id: &Id,
    qty: Qty,
) -> Result<(), CatalogError> {
    if qty.0 <= 0 {
        return Err(invalid("components", "kit_qty_not_positive"));
    }
    if component_id == kit_id {
        return Err(invalid("components", "kit_contains_itself"));
    }
    match get(tx, component_id)?.kind {
        ProductKind::Kit => Err(invalid("components", "kit_contains_kit")),
        ProductKind::VariantParent => Err(invalid("components", "parent_not_sellable")),
        ProductKind::Standard | ProductKind::Variant => Ok(()),
    }
}
