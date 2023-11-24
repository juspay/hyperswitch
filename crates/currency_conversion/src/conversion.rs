use common_enums::Currency;
use rust_decimal::Decimal;
use rusty_money::Money;

use crate::{
    error::CurrencyConversionError,
    types::{currency_match, ExchangeRates},
};

pub fn convert(
    ex_rates: &ExchangeRates,
    from_currency: Currency,
    to_currency: Currency,
    amount: i64,
) -> Result<Decimal, CurrencyConversionError> {
    let money_minor = Money::from_minor(amount, currency_match(from_currency));
    let base_currency = ex_rates.base_currency;
    if to_currency == base_currency {
        ex_rates.forward_conversion(*money_minor.amount(), from_currency)
    } else if from_currency == base_currency {
        ex_rates.backward_conversion(*money_minor.amount(), to_currency)
    } else {
        let base_conversion_amt =
            ex_rates.forward_conversion(*money_minor.amount(), from_currency)?;
        ex_rates.backward_conversion(base_conversion_amt, to_currency)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use std::collections::HashMap;

    use crate::types::CurrencyFactors;
    #[test]
    fn currency_to_currency_conversion() {
        use super::*;
        let mut conversion: HashMap<Currency, CurrencyFactors> = HashMap::new();
        let inr_conversion_rates =
            CurrencyFactors::new(Decimal::new(823173, 4), Decimal::new(1214, 5));
        let szl_conversion_rates =
            CurrencyFactors::new(Decimal::new(194423, 4), Decimal::new(514, 4));
        let convert_from = Currency::SZL;
        let convert_to = Currency::INR;
        let amount = 2000;
        let base_currency = Currency::USD;
        conversion.insert(convert_from, inr_conversion_rates);
        conversion.insert(convert_to, szl_conversion_rates);
        let sample_rate = ExchangeRates::new(base_currency, conversion);
        let res =
            convert(&sample_rate, convert_from, convert_to, amount).expect("converted_currency");
        println!(
            "The conversion from {} {} to {} is {:?}",
            amount, convert_from, convert_to, res
        );
    }

    #[test]
    fn currency_to_base_conversion() {
        use super::*;
        let mut conversion: HashMap<Currency, CurrencyFactors> = HashMap::new();
        let inr_conversion_rates =
            CurrencyFactors::new(Decimal::new(823173, 4), Decimal::new(1214, 5));
        let usd_conversion_rates = CurrencyFactors::new(Decimal::new(1, 0), Decimal::new(1, 0));
        let convert_from = Currency::INR;
        let convert_to = Currency::USD;
        let amount = 2000;
        let base_currency = Currency::USD;
        conversion.insert(convert_from, inr_conversion_rates);
        conversion.insert(convert_to, usd_conversion_rates);
        let sample_rate = ExchangeRates::new(base_currency, conversion);
        let res =
            convert(&sample_rate, convert_from, convert_to, amount).expect("converted_currency");
        println!(
            "The conversion from {} {} to {} is {:?}",
            amount, convert_from, convert_to, res
        );
    }

    #[test]
    fn base_to_currency_conversion() {
        use super::*;
        let mut conversion: HashMap<Currency, CurrencyFactors> = HashMap::new();
        let inr_conversion_rates =
            CurrencyFactors::new(Decimal::new(823173, 4), Decimal::new(1214, 5));
        let usd_conversion_rates = CurrencyFactors::new(Decimal::new(1, 0), Decimal::new(1, 0));
        let convert_from = Currency::USD;
        let convert_to = Currency::INR;
        let amount = 2000;
        let base_currency = Currency::USD;
        conversion.insert(convert_from, usd_conversion_rates);
        conversion.insert(convert_to, inr_conversion_rates);
        let sample_rate = ExchangeRates::new(base_currency, conversion);
        let res =
            convert(&sample_rate, convert_from, convert_to, amount).expect("converted_currency");
        println!(
            "The conversion from {} {} to {} is {:?}",
            amount, convert_from, convert_to, res
        );
    }
}
