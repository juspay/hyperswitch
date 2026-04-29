use std::num::NonZeroU8;

use common_types::payments::InstallmentInterestRate;
use common_utils::types::MinorUnit;

#[test]
fn test_calculate_emi_interest_with_zero_rate() {
    let rate = InstallmentInterestRate::try_from(0.0).unwrap();
    let amount = MinorUnit::new(10000);
    let installments = NonZeroU8::new(12).unwrap();

    let result = rate.calculate_emi_interest(amount, installments).unwrap();
    assert_eq!(result.get_amount_as_i64(), 0);
}

#[test]
fn test_calculate_emi_interest_consistency() {
    let rate = InstallmentInterestRate::try_from(10.0).unwrap();
    let amount = MinorUnit::new(50000);
    let installments = NonZeroU8::new(6).unwrap();

    let result1 = rate.calculate_emi_interest(amount, installments).unwrap();
    let result2 = rate.calculate_emi_interest(amount, installments).unwrap();
    assert_eq!(result1, result2);
}

#[test]
fn test_calculate_emi_interest_known_value() {
    let rate = InstallmentInterestRate::try_from(12.0).unwrap();
    let amount = MinorUnit::new(100000);
    let installments = NonZeroU8::new(12).unwrap();

    let result = rate.calculate_emi_interest(amount, installments).unwrap();
    assert_eq!(result.get_amount_as_i64(), 93725);
}

#[test]
fn test_installment_interest_rate_try_from_negative() {
    assert!(InstallmentInterestRate::try_from(-1.0).is_err());
}
