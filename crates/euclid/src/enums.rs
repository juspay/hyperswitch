pub use common_enums::{
    AuthenticationType, CaptureMethod, CardNetwork, Country, Currency,
    FutureUsage as SetupFutureUsage, PaymentMethod, PaymentMethodType,
};
use serde::{Deserialize, Serialize};
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
collect_variants!(PaymentType);
collect_variants!(MandateType);
collect_variants!(MandateAcceptanceType);
collect_variants!(PaymentMethodType);
collect_variants!(CardNetwork);
collect_variants!(AuthenticationType);
collect_variants!(CaptureMethod);
collect_variants!(Currency);
collect_variants!(Country);
collect_variants!(Connector);
collect_variants!(SetupFutureUsage);

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumVariantNames,
    strum::EnumIter,
    strum::EnumString,
    frunk::LabelledGeneric,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Connector {
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "phonypay")]
    #[strum(serialize = "phonypay")]
    DummyConnector1,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "fauxpay")]
    #[strum(serialize = "fauxpay")]
    DummyConnector2,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "pretendpay")]
    #[strum(serialize = "pretendpay")]
    DummyConnector3,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "stripe_test")]
    #[strum(serialize = "stripe_test")]
    DummyConnector4,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "adyen_test")]
    #[strum(serialize = "adyen_test")]
    DummyConnector5,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "checkout_test")]
    #[strum(serialize = "checkout_test")]
    DummyConnector6,
    #[cfg(feature = "dummy_connector")]
    #[serde(rename = "paypal_test")]
    #[strum(serialize = "paypal_test")]
    DummyConnector7,
    Aci,
    Adyen,
    Airwallex,
    Authorizedotnet,
    Bitpay,
    Bambora,
    Bluesnap,
    Boku,
    Braintree,
    Cashtocode,
    Checkout,
    Coinbase,
    Cryptopay,
    Cybersource,
    Dlocal,
    Fiserv,
    Forte,
    Globalpay,
    Globepay,
    Gocardless,
    Helcim,
    Iatapay,
    Klarna,
    Mollie,
    Multisafepay,
    Nexinets,
    Nmi,
    Noon,
    Nuvei,
    Opennode,
    Payme,
    Paypal,
    Payu,
    Powertranz,
    Rapyd,
    Shift4,
    Square,
    Stax,
    Stripe,
    Trustpay,
    Tsys,
    Volt,
    Wise,
    Worldline,
    Worldpay,
    Zen,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumVariantNames,
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
    strum::EnumVariantNames,
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
    strum::EnumVariantNames,
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
