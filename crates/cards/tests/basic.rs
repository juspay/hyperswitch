#![allow(dead_code, clippy::unwrap_used, clippy::panic_in_result_fn)]

use cards::CardSecurityCode;
/// use cards::CardExpirationMonth;
/// use cards::CardExpirationYear;
use cards::CardExpiration;

use masking::PeekInterface;
use masking::StrongSecret;

#[test]
fn basic() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    let csc =  StrongSecret::<u16>::new(1234);
    let expiry_month = StrongSecret::<u8>::new(12);
    let expiry_year = StrongSecret::<u16>::new(2023);
    

    let card_security_code = CardSecurityCode::new(csc).unwrap();
    let card_expiration = CardExpiration::new(expiry_month, expiry_year).unwrap();

    assert_eq!(*card_security_code.peek().peek(), 1234);
    assert_eq!(*card_expiration.get_month().peek().peek(), 12);
    assert_eq!(*card_expiration.get_year().peek().peek(), 2023);

    Ok(())
}