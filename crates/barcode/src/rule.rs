//! Scale-label rules (Rule CHK-08). A rule says which digits of a scale sticker's
//! barcode hold the item code and which hold the weight or price. The pattern
//! format is Odoo's barcode nomenclature, so shops can reuse their scale settings.
//!
//! Example: `21.....{NNDDD}` reads `2100123012343` as prefix `21`, item code `00123`,
//! value `01.234`, and check digit `3`. The check digit is not part of the pattern.

use std::collections::BTreeMap;

use pos_core::AppError;

/// What the number on a scale sticker means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    /// The weight (or volume), in the product's own unit.
    Weight,
    /// The price for the whole sticker.
    Price,
}

/// One scale-label rule, as the shop owner set it up in Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaleRule {
    /// A name the shop owner recognises. Example: "Weight".
    pub name: String,
    /// The Odoo-style pattern. Example: `21.....{NNDDD}`.
    pub pattern: String,
    pub kind: ScaleKind,
}

/// What one position of the barcode holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    /// This digit must be exactly this. Example: the `2` and `1` of `21`.
    Fixed(u8),
    /// A digit of the item code (`.` in the pattern).
    Item,
    /// A digit of the weight or price (`N` or `D` in the pattern).
    Value,
}

/// A pattern after it has been checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pattern {
    /// One slot per barcode digit, without the check digit: 12 for EAN-13, 7 for EAN-8.
    pub slots: Vec<Slot>,
    /// How many of the value digits come after the decimal point (the `D`s).
    pub decimals: u32,
}

/// The rules every new shop starts with. They match Odoo's defaults.
pub fn default_rules() -> Vec<ScaleRule> {
    vec![
        ScaleRule {
            name: "Weight".to_string(),
            pattern: "21.....{NNDDD}".to_string(),
            kind: ScaleKind::Weight,
        },
        ScaleRule {
            name: "Price".to_string(),
            pattern: "23.....{NNNDD}".to_string(),
            kind: ScaleKind::Price,
        },
    ]
}

/// Checks a rule before it is saved. The error code says what is wrong with it.
pub fn validate_rule(rule: &ScaleRule) -> Result<(), AppError> {
    if rule.name.trim().is_empty() {
        return Err(field_error("name", "scale_rule_name_missing"));
    }
    match read_pattern(&rule.pattern) {
        Ok(_) => Ok(()),
        Err(code) => Err(field_error("pattern", code)),
    }
}

/// Rule CAT-11: internal barcodes must never look like scale stickers, and two scale
/// rules must never claim the same sticker. Returns the name of the first rule that
/// clashes, or `None` if all is well. Broken rules are left out (`validate_rule` reports them).
pub fn rules_overlap(rules: &[ScaleRule], internal_prefix: &str) -> Option<String> {
    let mut checked: Vec<(&ScaleRule, Pattern)> = Vec::new();
    for rule in rules {
        if let Ok(pattern) = read_pattern(&rule.pattern) {
            checked.push((rule, pattern));
        }
    }

    // Internal barcodes are EAN-13, so only 12-slot (EAN-13) rules can clash with them.
    if let Some(internal_slots) = prefix_slots(internal_prefix) {
        for (rule, pattern) in &checked {
            if pattern.slots.len() == 12 && slots_can_agree(&pattern.slots, &internal_slots) {
                return Some(rule.name.clone());
            }
        }
    }

    for later in 1..checked.len() {
        for earlier in 0..later {
            let (later_rule, later_pattern) = &checked[later];
            let earlier_pattern = &checked[earlier].1;
            let same_length = later_pattern.slots.len() == earlier_pattern.slots.len();
            if same_length && slots_can_agree(&later_pattern.slots, &earlier_pattern.slots) {
                return Some(later_rule.name.clone());
            }
        }
    }
    None
}

/// Reads a pattern into slots. On a mistake, returns the error code for the screen.
/// Checks run in this order: characters and braces, the value group, length, prefix, item code.
pub(crate) fn read_pattern(pattern: &str) -> Result<Pattern, &'static str> {
    let mut slots = Vec::new();
    let mut inside_value = false;
    let mut value_groups = 0;
    let mut value_digits = 0;
    let mut decimals = 0;

    for character in pattern.chars() {
        match (character, inside_value) {
            ('{', true) => return Err("scale_rule_pattern_braces"),
            ('{', false) if value_groups > 0 => return Err("scale_rule_value_twice"),
            ('{', false) => {
                inside_value = true;
                value_groups += 1;
            }
            ('}', false) => return Err("scale_rule_pattern_braces"),
            ('}', true) if value_digits == 0 => return Err("scale_rule_value_missing"),
            ('}', true) => inside_value = false,
            // Whole-number digits (N) always come before decimals (D).
            ('N', true) if decimals > 0 => return Err("scale_rule_value_order"),
            ('N', true) => {
                slots.push(Slot::Value);
                value_digits += 1;
            }
            ('D', true) => {
                slots.push(Slot::Value);
                value_digits += 1;
                decimals += 1;
            }
            ('0'..='9', false) => slots.push(Slot::Fixed(character as u8 - b'0')),
            ('.', false) => slots.push(Slot::Item),
            _ => return Err("scale_rule_pattern_character"),
        }
    }

    if inside_value {
        return Err("scale_rule_pattern_braces");
    }
    if value_groups == 0 {
        return Err("scale_rule_value_missing");
    }
    check_layout(&slots)?;
    Ok(Pattern { slots, decimals })
}

/// Checks the length, the prefix and the item code of a pattern read into slots.
fn check_layout(slots: &[Slot]) -> Result<(), &'static str> {
    // The pattern covers every digit except the check digit: 12 for EAN-13, 7 for EAN-8.
    if slots.len() != 12 && slots.len() != 7 {
        return Err("scale_rule_pattern_length");
    }
    // Without a fixed first digit the rule would catch ordinary product barcodes.
    if !matches!(slots.first(), Some(Slot::Fixed(_))) {
        return Err("scale_rule_prefix_missing");
    }
    if !slots.contains(&Slot::Item) {
        return Err("scale_rule_item_code_missing");
    }
    Ok(())
}

/// The internal prefix as fixed slots. `None` if it isn't one or more digits: such a
/// prefix makes no barcodes at all (`internal_ean13` refuses it), so it clashes with nothing.
fn prefix_slots(prefix: &str) -> Option<Vec<Slot>> {
    if prefix.is_empty() {
        return None;
    }
    let mut slots = Vec::new();
    for byte in prefix.bytes() {
        if !byte.is_ascii_digit() {
            return None;
        }
        slots.push(Slot::Fixed(byte - b'0'));
    }
    Some(slots)
}

/// True if one barcode could fit both slot lists: they never ask for two
/// different fixed digits in the same position.
fn slots_can_agree(first: &[Slot], second: &[Slot]) -> bool {
    for (one, other) in first.iter().zip(second.iter()) {
        if let (Slot::Fixed(a), Slot::Fixed(b)) = (one, other)
            && a != b
        {
            return false;
        }
    }
    true
}

fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chk08_default_weight_pattern_reads_into_slots() {
        let pattern = read_pattern("21.....{NNDDD}").unwrap();
        assert_eq!(pattern.decimals, 3);
        assert_eq!(pattern.slots.len(), 12);
        assert_eq!(pattern.slots[0..2], [Slot::Fixed(2), Slot::Fixed(1)]);
        assert_eq!(pattern.slots[2..7], [Slot::Item; 5]);
        assert_eq!(pattern.slots[7..12], [Slot::Value; 5]);
    }

    #[test]
    fn chk08_value_without_decimals_is_allowed() {
        assert_eq!(read_pattern("21.....{NNNNN}").unwrap().decimals, 0);
    }

    #[test]
    fn cat11_an_invalid_internal_prefix_clashes_with_nothing() {
        assert_eq!(rules_overlap(&default_rules(), "2x"), None);
    }
}
