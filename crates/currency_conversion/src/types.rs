use std::collections::HashMap;

use api_models::enums;
use rust_decimal::Decimal;
use rusty_money::iso::{self, Currency};

use crate::error::CurrencyConversionError;

/// Cached currency store of base currency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeRates {
    pub base_currency: enums::Currency,
    pub conversion: HashMap<enums::Currency, CurrencyFactors>,
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
    pub fn new(
        base_currency: enums::Currency,
        conversion: HashMap<enums::Currency, CurrencyFactors>,
    ) -> Self {
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
        from_currency: enums::Currency,
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
        to_currency: enums::Currency,
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

pub fn currency_match(currency: enums::Currency) -> &'static Currency {
    match currency {
        enums::Currency::AED => iso::AED,
        enums::Currency::ALL => iso::ALL,
        enums::Currency::AMD => iso::AMD,
        enums::Currency::ANG => iso::ANG,
        enums::Currency::ARS => iso::ARS,
        enums::Currency::AUD => iso::AUD,
        enums::Currency::AWG => iso::AWG,
        enums::Currency::AZN => iso::AZN,
        enums::Currency::BBD => iso::BBD,
        enums::Currency::BDT => iso::BDT,
        enums::Currency::BHD => iso::BHD,
        enums::Currency::BIF => iso::BIF,
        enums::Currency::BMD => iso::BMD,
        enums::Currency::BND => iso::BND,
        enums::Currency::BOB => iso::BOB,
        enums::Currency::BRL => iso::BRL,
        enums::Currency::BSD => iso::BSD,
        enums::Currency::BWP => iso::BWP,
        enums::Currency::BZD => iso::BZD,
        enums::Currency::CAD => iso::CAD,
        enums::Currency::CHF => iso::CHF,
        enums::Currency::CLP => iso::CLP,
        enums::Currency::CNY => iso::CNY,
        enums::Currency::COP => iso::COP,
        enums::Currency::CRC => iso::CRC,
        enums::Currency::CUP => iso::CUP,
        enums::Currency::CZK => iso::CZK,
        enums::Currency::DJF => iso::DJF,
        enums::Currency::DKK => iso::DKK,
        enums::Currency::DOP => iso::DOP,
        enums::Currency::DZD => iso::DZD,
        enums::Currency::EGP => iso::EGP,
        enums::Currency::ETB => iso::ETB,
        enums::Currency::EUR => iso::EUR,
        enums::Currency::FJD => iso::FJD,
        enums::Currency::GBP => iso::GBP,
        enums::Currency::GHS => iso::GHS,
        enums::Currency::GIP => iso::GIP,
        enums::Currency::GMD => iso::GMD,
        enums::Currency::GNF => iso::GNF,
        enums::Currency::GTQ => iso::GTQ,
        enums::Currency::GYD => iso::GYD,
        enums::Currency::HKD => iso::HKD,
        enums::Currency::HNL => iso::HNL,
        enums::Currency::HRK => iso::HRK,
        enums::Currency::HTG => iso::HTG,
        enums::Currency::HUF => iso::HUF,
        enums::Currency::IDR => iso::IDR,
        enums::Currency::ILS => iso::ILS,
        enums::Currency::INR => iso::INR,
        enums::Currency::JMD => iso::JMD,
        enums::Currency::JOD => iso::JOD,
        enums::Currency::JPY => iso::JPY,
        enums::Currency::KES => iso::KES,
        enums::Currency::KGS => iso::KGS,
        enums::Currency::KHR => iso::KHR,
        enums::Currency::KMF => iso::KMF,
        enums::Currency::KRW => iso::KRW,
        enums::Currency::KWD => iso::KWD,
        enums::Currency::KYD => iso::KYD,
        enums::Currency::KZT => iso::KZT,
        enums::Currency::LAK => iso::LAK,
        enums::Currency::LBP => iso::LBP,
        enums::Currency::LKR => iso::LKR,
        enums::Currency::LRD => iso::LRD,
        enums::Currency::LSL => iso::LSL,
        enums::Currency::MAD => iso::MAD,
        enums::Currency::MDL => iso::MDL,
        enums::Currency::MGA => iso::MGA,
        enums::Currency::MKD => iso::MKD,
        enums::Currency::MMK => iso::MMK,
        enums::Currency::MNT => iso::MNT,
        enums::Currency::MOP => iso::MOP,
        enums::Currency::MUR => iso::MUR,
        enums::Currency::MVR => iso::MVR,
        enums::Currency::MWK => iso::MWK,
        enums::Currency::MXN => iso::MXN,
        enums::Currency::MYR => iso::MYR,
        enums::Currency::NAD => iso::NAD,
        enums::Currency::NGN => iso::NGN,
        enums::Currency::NIO => iso::NIO,
        enums::Currency::NOK => iso::NOK,
        enums::Currency::NPR => iso::NPR,
        enums::Currency::NZD => iso::NZD,
        enums::Currency::OMR => iso::OMR,
        enums::Currency::PEN => iso::PEN,
        enums::Currency::PGK => iso::PGK,
        enums::Currency::PHP => iso::PHP,
        enums::Currency::PKR => iso::PKR,
        enums::Currency::PLN => iso::PLN,
        enums::Currency::PYG => iso::PYG,
        enums::Currency::QAR => iso::QAR,
        enums::Currency::RON => iso::RON,
        enums::Currency::RUB => iso::RUB,
        enums::Currency::RWF => iso::RWF,
        enums::Currency::SAR => iso::SAR,
        enums::Currency::SCR => iso::SCR,
        enums::Currency::SEK => iso::SEK,
        enums::Currency::SGD => iso::SGD,
        enums::Currency::SLL => iso::SLL,
        enums::Currency::SOS => iso::SOS,
        enums::Currency::SSP => iso::SSP,
        enums::Currency::SVC => iso::SVC,
        enums::Currency::SZL => iso::SZL,
        enums::Currency::THB => iso::THB,
        enums::Currency::TTD => iso::TTD,
        enums::Currency::TRY => iso::TRY,
        enums::Currency::TWD => iso::TWD,
        enums::Currency::TZS => iso::TZS,
        enums::Currency::UGX => iso::UGX,
        enums::Currency::USD => iso::USD,
        enums::Currency::UYU => iso::UYU,
        enums::Currency::UZS => iso::UZS,
        enums::Currency::VND => iso::VND,
        enums::Currency::VUV => iso::VUV,
        enums::Currency::XAF => iso::XAF,
        enums::Currency::XOF => iso::XOF,
        enums::Currency::XPF => iso::XPF,
        enums::Currency::YER => iso::YER,
        enums::Currency::ZAR => iso::ZAR,
    }
}
