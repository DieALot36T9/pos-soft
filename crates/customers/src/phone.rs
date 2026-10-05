//! Phone number cleaning. The only place phone numbers are turned into stored form.

use phonenumber::{Mode, country};

use crate::CustomersError;

/// Cleans a typed phone number into E.164 form, using the shop's country for local numbers.
/// Example: "01711-123456" in Bangladesh ("BD") becomes "+8801711123456".
/// The rules come from Google's libphonenumber, through the `phonenumber` crate (Reuse Map).
pub fn normalize_phone(raw: &str, default_country: &str) -> Result<String, CustomersError> {
    let country: country::Id = default_country
        .trim()
        .to_uppercase()
        .parse()
        .map_err(|_| CustomersError::CountryInvalid(default_country.to_string()))?;
    let number =
        phonenumber::parse(Some(country), raw).map_err(|_| CustomersError::PhoneInvalid)?;
    if !phonenumber::is_valid(&number) {
        return Err(CustomersError::PhoneInvalid);
    }
    Ok(number.format().mode(Mode::E164).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cus_01_numbers_from_other_countries_keep_their_own_code() {
        assert_eq!(
            normalize_phone("+44 20 7946 0958", "BD").unwrap(),
            "+442079460958"
        );
    }

    #[test]
    fn cus_01_lower_case_country_codes_work() {
        assert_eq!(
            normalize_phone("01711123456", "bd").unwrap(),
            "+8801711123456"
        );
    }

    #[test]
    fn cus_01_text_that_is_not_a_number_is_refused() {
        assert!(matches!(
            normalize_phone("call me", "BD"),
            Err(CustomersError::PhoneInvalid)
        ));
        assert!(matches!(
            normalize_phone("", "BD"),
            Err(CustomersError::PhoneInvalid)
        ));
    }
}
