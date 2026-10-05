//! The guard against spreadsheet formulas in CSV files (OWASP "CSV Injection").
//! Excel runs a cell that starts with = + - @ (or a tab or carriage return) as
//! a formula. So when writing, such a cell gets a leading apostrophe, which
//! makes Excel show it as text. When reading, the apostrophe is taken off
//! again, so an export can be imported back unchanged (IO-03).
//! Plain numbers such as -12.50 or +8801711000000 are not formulas and are left alone.

/// Adds the apostrophe if Excel could run this cell as a formula.
/// Example: "=SUM(A1)" → "'=SUM(A1)". "-12.50" stays as it is.
pub(crate) fn guard(cell: &str) -> String {
    if looks_guarded(cell) {
        format!("'{cell}")
    } else {
        cell.to_string()
    }
}

/// Takes off the apostrophe that `guard` added. Example: "'=SUM(A1)" → "=SUM(A1)".
pub(crate) fn unguard(cell: &str) -> &str {
    match cell.strip_prefix('\'') {
        Some(rest) if looks_guarded(rest) => rest,
        _ => cell,
    }
}

/// True for a cell `guard` must change: a possible formula, or text that
/// already looks like a guarded cell (so a real leading apostrophe survives
/// a round trip too).
fn looks_guarded(cell: &str) -> bool {
    if could_be_formula(cell) {
        return true;
    }
    match cell.strip_prefix('\'') {
        Some(rest) => looks_guarded(rest),
        None => false,
    }
}

/// True when Excel would run the cell as a formula.
fn could_be_formula(cell: &str) -> bool {
    let starts_risky = cell.starts_with(['=', '+', '-', '@', '\t', '\r']);
    starts_risky && !is_plain_number(cell)
}

/// True for a signed decimal number like "-12.50" or "+880171".
fn is_plain_number(cell: &str) -> bool {
    let unsigned = cell.strip_prefix(['-', '+']).unwrap_or(cell);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, "0"));
    let all_digits =
        |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    all_digits(whole) && all_digits(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_03_guard_then_unguard_gives_the_cell_back() {
        let cells = [
            "=1+1", "+A1", "-A1", "@x", "\tx", "\rx", "'=x", "''=x", "'", "'-5", "-5", "plain", "",
        ];
        for cell in cells {
            assert_eq!(unguard(&guard(cell)), cell, "{cell:?}");
        }
    }

    #[test]
    fn io_03_only_possible_formulas_get_the_apostrophe() {
        assert_eq!(guard("=1+1"), "'=1+1");
        assert_eq!(guard("'=1+1"), "''=1+1");
        assert_eq!(guard("'-5"), "'-5");
        assert_eq!(guard("-5.25"), "-5.25");
        assert_eq!(guard("-"), "'-");
        assert_eq!(guard("Milk"), "Milk");
    }
}
