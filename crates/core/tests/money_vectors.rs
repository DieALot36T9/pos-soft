//! Checks `format_money` and `parse_money` against the shared file
//! `money_vectors.json`. The UI's `formatMoney` reads the same file.

use pos_core::{AppError, CurrencyFormat, Money, format_money, parse_money};
use serde::Deserialize;

#[derive(Deserialize)]
struct Vectors {
    format: Vec<FormatCase>,
    parse: Vec<ParseCase>,
}

#[derive(Deserialize)]
struct FormatCase {
    about: String,
    minor: i64,
    format: CurrencyFormat,
    text: String,
}

#[derive(Deserialize)]
struct ParseCase {
    text: String,
    minor_digits: u8,
    minor: Option<i64>,
    error: Option<String>,
}

fn vectors() -> Vectors {
    let file = include_str!("money_vectors.json");
    serde_json::from_str(file).expect("money_vectors.json must be valid")
}

#[test]
fn set_05_format_money_matches_every_shared_example() {
    for case in vectors().format {
        assert_eq!(
            format_money(Money(case.minor), &case.format),
            case.text,
            "{}",
            case.about
        );
    }
}

#[test]
fn parse_money_matches_every_shared_example() {
    for case in vectors().parse {
        let result = parse_money(&case.text, case.minor_digits);
        match (case.minor, case.error) {
            (Some(minor), None) => assert_eq!(result, Ok(Money(minor)), "{:?}", case.text),
            (None, Some(code)) => {
                let error = result.expect_err(&case.text);
                match AppError::from(error) {
                    AppError::Validation { code: actual, .. } => {
                        assert_eq!(actual, code, "{:?}", case.text)
                    }
                    other => panic!("{:?} gave {other:?}", case.text),
                }
            }
            _ => panic!("case {:?} needs either minor or error", case.text),
        }
    }
}
