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
    use std::collections::HashMap;

    use common_enums::Currency;
    use rust_decimal::Decimal;

    use super::convert;
    use crate::types::{CurrencyFactors, ExchangeRates};

    // Base currency is USD for every case below. `CurrencyFactors::new` takes
    // `(to_factor, from_factor)`, where `to_factor` converts base -> currency
    // and `from_factor` converts currency -> base.
    //   INR: 1 USD = 80 INR  -> to_factor = 80,  from_factor = 1/80 = 0.0125
    //   SZL: 1 USD = 20 SZL  -> to_factor = 20,  from_factor = 1/20 = 0.05
    fn inr_factors() -> CurrencyFactors {
        CurrencyFactors::new(Decimal::new(80, 0), Decimal::new(125, 4))
    }
    fn szl_factors() -> CurrencyFactors {
        CurrencyFactors::new(Decimal::new(20, 0), Decimal::new(5, 2))
    }
    fn usd_factors() -> CurrencyFactors {
        CurrencyFactors::new(Decimal::new(1, 0), Decimal::new(1, 0))
    }

    #[test]
    fn currency_to_currency_conversion() {
        let mut conversion = HashMap::new();
        conversion.insert(Currency::SZL, szl_factors());
        conversion.insert(Currency::INR, inr_factors());
        let rates = ExchangeRates::new(Currency::USD, conversion);

        // 2000 minor SZL = 20 SZL -> 20 * 0.05 = 1 USD -> 1 * 80 = 80 INR
        let res = convert(&rates, Currency::SZL, Currency::INR, 2000).expect("conversion failed");
        assert_eq!(res, Decimal::from(80));
    }

    #[test]
    fn currency_to_base_conversion() {
        let mut conversion = HashMap::new();
        conversion.insert(Currency::INR, inr_factors());
        conversion.insert(Currency::USD, usd_factors());
        let rates = ExchangeRates::new(Currency::USD, conversion);

        // 2000 minor INR = 20 INR -> 20 * 0.0125 = 0.25 USD
        let res = convert(&rates, Currency::INR, Currency::USD, 2000).expect("conversion failed");
        assert_eq!(res, Decimal::new(25, 2));
    }

    #[test]
    fn base_to_currency_conversion() {
        let mut conversion = HashMap::new();
        conversion.insert(Currency::USD, usd_factors());
        conversion.insert(Currency::INR, inr_factors());
        let rates = ExchangeRates::new(Currency::USD, conversion);

        // 2000 minor USD = 20 USD -> 20 * 80 = 1600 INR
        let res = convert(&rates, Currency::USD, Currency::INR, 2000).expect("conversion failed");
        assert_eq!(res, Decimal::from(1600));
    }
}
