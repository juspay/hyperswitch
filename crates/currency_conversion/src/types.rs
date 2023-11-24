use std::collections::HashMap;

use common_enums::Currency;
use rust_decimal::Decimal;
use rusty_money::iso;

use crate::error::CurrencyConversionError;

/// Cached currency store of base currency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeRates {
    pub base_currency: Currency,
    pub conversion: HashMap<Currency, CurrencyFactors>,
}

/// Stores the multiplicative factor for  conversion between currency to base and vice versa
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurrencyFactors {
    /// The factor that will be multiplied to provide Currency output
    pub to_factor: Decimal,
    /// The factor that will be multiplied to provide for the base output
    pub from_factor: Decimal,
}

impl CurrencyFactors {
    pub fn new(to_factor: Decimal, from_factor: Decimal) -> Self {
        Self {
            to_factor,
            from_factor,
        }
    }
}

impl ExchangeRates {
    pub fn new(base_currency: Currency, conversion: HashMap<Currency, CurrencyFactors>) -> Self {
        Self {
            base_currency,
            conversion,
        }
    }

    /// The flow here is from_currency -> base_currency -> to_currency
    /// from to_currency -> base currency
    pub fn forward_conversion(
        &self,
        amt: Decimal,
        from_currency: Currency,
    ) -> Result<Decimal, CurrencyConversionError> {
        let from_factor = self
            .conversion
            .get(&from_currency)
            .ok_or_else(|| {
                CurrencyConversionError::ConversionNotSupported(from_currency.to_string())
            })?
            .from_factor;
        amt.checked_mul(from_factor)
            .ok_or(CurrencyConversionError::DecimalMultiplicationFailed)
    }

    /// from base_currency -> to_currency
    pub fn backward_conversion(
        &self,
        amt: Decimal,
        to_currency: Currency,
    ) -> Result<Decimal, CurrencyConversionError> {
        let to_factor = self
            .conversion
            .get(&to_currency)
            .ok_or_else(|| {
                CurrencyConversionError::ConversionNotSupported(to_currency.to_string())
            })?
            .to_factor;
        amt.checked_mul(to_factor)
            .ok_or(CurrencyConversionError::DecimalMultiplicationFailed)
    }
}

pub fn currency_match(currency: Currency) -> &'static iso::Currency {
    match currency {
        Currency::AED => iso::AED,
        Currency::ALL => iso::ALL,
        Currency::AMD => iso::AMD,
        Currency::ANG => iso::ANG,
        Currency::ARS => iso::ARS,
        Currency::AUD => iso::AUD,
        Currency::AWG => iso::AWG,
        Currency::AZN => iso::AZN,
        Currency::BBD => iso::BBD,
        Currency::BDT => iso::BDT,
        Currency::BHD => iso::BHD,
        Currency::BIF => iso::BIF,
        Currency::BMD => iso::BMD,
        Currency::BND => iso::BND,
        Currency::BOB => iso::BOB,
        Currency::BRL => iso::BRL,
        Currency::BSD => iso::BSD,
        Currency::BWP => iso::BWP,
        Currency::BZD => iso::BZD,
        Currency::CAD => iso::CAD,
        Currency::CHF => iso::CHF,
        Currency::CLP => iso::CLP,
        Currency::CNY => iso::CNY,
        Currency::COP => iso::COP,
        Currency::CRC => iso::CRC,
        Currency::CUP => iso::CUP,
        Currency::CZK => iso::CZK,
        Currency::DJF => iso::DJF,
        Currency::DKK => iso::DKK,
        Currency::DOP => iso::DOP,
        Currency::DZD => iso::DZD,
        Currency::EGP => iso::EGP,
        Currency::ETB => iso::ETB,
        Currency::EUR => iso::EUR,
        Currency::FJD => iso::FJD,
        Currency::GBP => iso::GBP,
        Currency::GHS => iso::GHS,
        Currency::GIP => iso::GIP,
        Currency::GMD => iso::GMD,
        Currency::GNF => iso::GNF,
        Currency::GTQ => iso::GTQ,
        Currency::GYD => iso::GYD,
        Currency::HKD => iso::HKD,
        Currency::HNL => iso::HNL,
        Currency::HRK => iso::HRK,
        Currency::HTG => iso::HTG,
        Currency::HUF => iso::HUF,
        Currency::IDR => iso::IDR,
        Currency::ILS => iso::ILS,
        Currency::INR => iso::INR,
        Currency::JMD => iso::JMD,
        Currency::JOD => iso::JOD,
        Currency::JPY => iso::JPY,
        Currency::KES => iso::KES,
        Currency::KGS => iso::KGS,
        Currency::KHR => iso::KHR,
        Currency::KMF => iso::KMF,
        Currency::KRW => iso::KRW,
        Currency::KWD => iso::KWD,
        Currency::KYD => iso::KYD,
        Currency::KZT => iso::KZT,
        Currency::LAK => iso::LAK,
        Currency::LBP => iso::LBP,
        Currency::LKR => iso::LKR,
        Currency::LRD => iso::LRD,
        Currency::LSL => iso::LSL,
        Currency::MAD => iso::MAD,
        Currency::MDL => iso::MDL,
        Currency::MGA => iso::MGA,
        Currency::MKD => iso::MKD,
        Currency::MMK => iso::MMK,
        Currency::MNT => iso::MNT,
        Currency::MOP => iso::MOP,
        Currency::MUR => iso::MUR,
        Currency::MVR => iso::MVR,
        Currency::MWK => iso::MWK,
        Currency::MXN => iso::MXN,
        Currency::MYR => iso::MYR,
        Currency::NAD => iso::NAD,
        Currency::NGN => iso::NGN,
        Currency::NIO => iso::NIO,
        Currency::NOK => iso::NOK,
        Currency::NPR => iso::NPR,
        Currency::NZD => iso::NZD,
        Currency::OMR => iso::OMR,
        Currency::PEN => iso::PEN,
        Currency::PGK => iso::PGK,
        Currency::PHP => iso::PHP,
        Currency::PKR => iso::PKR,
        Currency::PLN => iso::PLN,
        Currency::PYG => iso::PYG,
        Currency::QAR => iso::QAR,
        Currency::RON => iso::RON,
        Currency::RUB => iso::RUB,
        Currency::RWF => iso::RWF,
        Currency::SAR => iso::SAR,
        Currency::SCR => iso::SCR,
        Currency::SEK => iso::SEK,
        Currency::SGD => iso::SGD,
        Currency::SLL => iso::SLL,
        Currency::SOS => iso::SOS,
        Currency::SSP => iso::SSP,
        Currency::SVC => iso::SVC,
        Currency::SZL => iso::SZL,
        Currency::THB => iso::THB,
        Currency::TTD => iso::TTD,
        Currency::TRY => iso::TRY,
        Currency::TWD => iso::TWD,
        Currency::TZS => iso::TZS,
        Currency::UGX => iso::UGX,
        Currency::USD => iso::USD,
        Currency::UYU => iso::UYU,
        Currency::UZS => iso::UZS,
        Currency::VND => iso::VND,
        Currency::VUV => iso::VUV,
        Currency::XAF => iso::XAF,
        Currency::XOF => iso::XOF,
        Currency::XPF => iso::XPF,
        Currency::YER => iso::YER,
        Currency::ZAR => iso::ZAR,
    }
}
