#![allow(clippy::unwrap_used, clippy::expect_used)]

use cards::{CardExpiration, CardExpirationMonth, CardExpirationYear, CardSecurityCode};
use common_utils::date_time;
use masking::PeekInterface;

#[test]
/// This method tests the functionality of the CardSecurityCode type. It creates a valid and an invalid CardSecurityCode, performs various operations on them, and asserts the expected results. It also tests the serialization and deserialization of CardSecurityCode using serde_json.
fn test_card_security_code() {
    // no panic
    let valid_card_security_code = CardSecurityCode::try_from(1234).unwrap();

    // will panic on unwrap
    let invalid_card_security_code = CardSecurityCode::try_from(00);

    assert_eq!(*valid_card_security_code.peek(), 1234);
    assert!(invalid_card_security_code.is_err());

    let serialized = serde_json::to_string(&valid_card_security_code).unwrap();
    assert_eq!(serialized, "1234");

    let derialized = serde_json::from_str::<CardSecurityCode>(&serialized).unwrap();
    assert_eq!(*derialized.peek(), 1234);

    let invalid_deserialization = serde_json::from_str::<CardSecurityCode>("00");
    assert!(invalid_deserialization.is_err());
}

#[test]
/// This method tests the functionality of the CardExpirationMonth type, including its conversion from an integer, serialization to JSON, and deserialization from JSON. It also checks for error handling in case of invalid input.
fn test_card_expiration_month() {
    // no panic
    let card_exp_month = CardExpirationMonth::try_from(12).unwrap();

    // will panic on unwrap
    let invalid_card_exp_month = CardExpirationMonth::try_from(13);

    assert_eq!(*card_exp_month.peek(), 12);
    assert!(invalid_card_exp_month.is_err());

    let serialized = serde_json::to_string(&card_exp_month).unwrap();
    assert_eq!(serialized, "12");

    let derialized = serde_json::from_str::<CardExpirationMonth>(&serialized).unwrap();
    assert_eq!(*derialized.peek(), 12);

    let invalid_deserialization = serde_json::from_str::<CardExpirationMonth>("13");
    assert!(invalid_deserialization.is_err());
}

#[test]
/// This method tests the functionality of the CardExpirationYear type by creating a valid card expiration year from the current year, creating an invalid card expiration year from the previous year, and performing various assertions and validations on the created card expiration years. It also tests the serialization and deserialization of the card expiration year using serde_json.
fn test_card_expiration_year() {
    let curr_date = date_time::now();
    let curr_year = u16::try_from(curr_date.year()).expect("valid year");

    // no panic
    let card_exp_year = CardExpirationYear::try_from(curr_year).unwrap();

    // will panic on unwrap
    let invalid_card_exp_year = CardExpirationYear::try_from(curr_year - 1);

    assert_eq!(*card_exp_year.peek(), curr_year);
    assert!(invalid_card_exp_year.is_err());

    let serialized = serde_json::to_string(&card_exp_year).unwrap();
    assert_eq!(serialized, curr_year.to_string());

    let derialized = serde_json::from_str::<CardExpirationYear>(&serialized).unwrap();
    assert_eq!(*derialized.peek(), curr_year);

    let invalid_deserialization = serde_json::from_str::<CardExpirationYear>("123");
    assert!(invalid_deserialization.is_err());
}

#[test]
/// Tests the functionality of the CardExpiration type, including conversion from a tuple of month and year, checking for expiration, serialization to JSON, and deserialization from JSON.
fn test_card_expiration() {
    let curr_date = date_time::now();
    let curr_year = u16::try_from(curr_date.year()).expect("valid year");

    // no panic
    let card_exp = CardExpiration::try_from((3, curr_year)).unwrap();

    // will panic on unwrap
    let invalid_card_exp = CardExpiration::try_from((13, curr_year));

    assert_eq!(*card_exp.get_month().peek(), 3);
    assert_eq!(*card_exp.get_year().peek(), curr_year);
    assert!(card_exp.is_expired().unwrap());

    assert!(invalid_card_exp.is_err());

    let serialized = serde_json::to_string(&card_exp).unwrap();
    let expected_string = format!(r#"{{"month":{},"year":{}}}"#, 3, curr_year);
    assert_eq!(serialized, expected_string);

    let derialized = serde_json::from_str::<CardExpiration>(&serialized).unwrap();
    assert_eq!(*derialized.get_month().peek(), 3);
    assert_eq!(*derialized.get_year().peek(), curr_year);

    let invalid_serialized_string = r#"{"month":13,"year":123}"#;
    let invalid_deserialization = serde_json::from_str::<CardExpiration>(invalid_serialized_string);
    assert!(invalid_deserialization.is_err());
}
