mod eval;
mod expr;
mod typeck;
mod utils;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::eval;
    use super::expr as cbr_expr;
    use cbr_macros::costexpr;
    use euclid::dirval;
    use euclid::frontend::dir::{self as euclid_dir, enums as euclid_dir_enums};

    use common_enums::Currency;
    use currency_conversion::{types::CurrencyFactors, ExchangeRates};
    use rust_decimal::{prelude::FromPrimitive, Decimal};
    use std::collections::HashMap;

    #[test]
    fn test() {
        let expr = costexpr!(
            branch PaymentMethod {
                Card => 49 mUSD + 2% * amount
                    + branch PaymentCurrency {
                        USD => 0 USD,
                        default => 1% * amount,
                    }
                    + branch BillingCountry {
                        UnitedStatesOfAmerica => 0 USD,
                        default => 1% * amount,
                    },

                BankDebit => branch BankDebitType {
                    Ach => 0.75% * amount,
                    default => 0 USD,
                },

                default => 0 USD,
            }
        );

        let ctx = eval::CostEvalContext::from_dir_values([
            dirval!(PaymentMethod = Card),
            dirval!(PaymentAmount = 100),
            dirval!(PaymentCurrency = GBP),
        ]);

        let rates = ExchangeRates {
            base_currency: Currency::USD,
            conversion: HashMap::from_iter([(
                Currency::GBP,
                CurrencyFactors {
                    to_factor: Decimal::from_u8(2).expect("Decimal"),
                    from_factor: Decimal::from_u8(3).expect("Decimal"),
                },
            )]),
        };

        let res = eval::evaluate(&rates, &expr, &ctx).expect("result");
        println!("{res:?}");
    }
}
