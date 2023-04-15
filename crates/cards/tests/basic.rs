#![allow(dead_code, clippy::unwrap_used, clippy::panic_in_result_fn)]

use cards::CardSecurityCode;
use cards::CardExpirationMonth;
use cards::CardExpirationYear;
use cards::CardExpiration;

use common_utils::{date_time};
use masking::PeekInterface;

#[test]
fn test_card_security_code() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    // no panic
    let valid_card_security_code = CardSecurityCode::try_from(1234).unwrap();
    
    // will panic on unwrap
    let invalid_card_security_code = CardSecurityCode::try_from(12);

    assert_eq!(*valid_card_security_code.peek(), 1234);
    assert!(invalid_card_security_code.is_err());

    Ok(())
}

#[test]
fn test_card_expiration_month() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    // no panic
    let card_exp_month = CardExpirationMonth::try_from(12).unwrap();

    // will panic on unwrap
    let invalid_card_exp_month = CardExpirationMonth::try_from(13);

    assert_eq!(*card_exp_month.peek(), 12);
    assert!(invalid_card_exp_month.is_err());

    Ok(())
}

#[test]
fn test_card_expiration_year() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let curr_date = date_time::now();
    let curr_year = u16::try_from(curr_date.year()).expect("valid year");

    // no panic
    let card_exp_year = CardExpirationYear::try_from(curr_year).unwrap();

    // will panic on unwrap
    let invalid_card_exp_year = CardExpirationYear::try_from(curr_year - 1);

    assert_eq!(*card_exp_year.peek(), curr_year);
    assert!(invalid_card_exp_year.is_err());

    Ok(())
}

#[test]
fn test_card_expiration() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let curr_date = date_time::now();
    let curr_year = u16::try_from(curr_date.year()).expect("valid year");

    // no panic
    let card_exp = CardExpiration::try_from((3, curr_year + 1)).unwrap();
    
    // will panic on unwrap
    let invalid_card_exp = CardExpiration::try_from((13, curr_year + 1));

    assert_eq!(*card_exp.get_month().peek(), 3);
    assert_eq!(*card_exp.get_year().peek(), curr_year + 1);
    assert_eq!(card_exp.is_expired(), false);

    assert!(invalid_card_exp.is_err());

    Ok(())
}