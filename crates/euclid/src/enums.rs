pub use common_enums::{
    AuthenticationType, CaptureMethod, CardNetwork, Country, Currency,
    FutureUsage as SetupFutureUsage, PaymentMethod, PaymentMethodType, RoutableConnectors,
};
use strum::VariantNames;

pub trait CollectVariants {
    fn variants<T: FromIterator<String>>() -> T;
}
macro_rules! collect_variants {
    ($the_enum:ident) => {
        impl $crate::enums::CollectVariants for $the_enum {
            fn variants<T>() -> T
            where
                T: FromIterator<String>,
            {
                Self::VARIANTS.iter().map(|s| String::from(*s)).collect()
            }
        }
    };
}

pub(crate) use collect_variants;

collect_variants!(PaymentMethod);
collect_variants!(RoutableConnectors);
collect_variants!(PaymentType);
collect_variants!(MandateType);
collect_variants!(MandateAcceptanceType);
collect_variants!(PaymentMethodType);
collect_variants!(CardNetwork);
collect_variants!(AuthenticationType);
collect_variants!(CaptureMethod);
collect_variants!(Currency);
collect_variants!(Country);
collect_variants!(SetupFutureUsage);
#[cfg(feature = "payouts")]
collect_variants!(PayoutType);
#[cfg(feature = "payouts")]
collect_variants!(PayoutBankTransferType);
#[cfg(feature = "payouts")]
collect_variants!(PayoutWalletType);

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MandateAcceptanceType {
    Online,
    Offline,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PaymentType {
    SetupMandate,
    NonMandate,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MandateType {
    SingleUse,
    MultiUse,
}

#[cfg(feature = "payouts")]
#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutBankTransferType {
    Ach,
    Bacs,
    Sepa,
}

#[cfg(feature = "payouts")]
#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutWalletType {
    Paypal,
}

#[cfg(feature = "payouts")]
#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::VariantNames,
    strum::EnumIter,
    strum::EnumString,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PayoutType {
    Card,
    BankTransfer,
    Wallet,
}
