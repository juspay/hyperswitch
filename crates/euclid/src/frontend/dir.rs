//! Domain Intermediate Representation
pub mod enums;
pub mod lowering;
pub mod transformers;

use strum::IntoEnumIterator;

use crate::{enums as euclid_enums, frontend::ast, types};

#[macro_export]
#[cfg(feature = "connector_choice_mca_id")]
macro_rules! dirval {
    (Connector = $name:ident) => {
        $crate::frontend::dir::DirValue::Connector(Box::new(
            $crate::frontend::ast::ConnectorChoice {
                connector: $crate::frontend::dir::enums::Connector::$name,
            },
        ))
    };

    ($key:ident = $val:ident) => {{
        pub use $crate::frontend::dir::enums::*;

        $crate::frontend::dir::DirValue::$key($key::$val)
    }};

    ($key:ident = $num:literal) => {{
        $crate::frontend::dir::DirValue::$key($crate::types::NumValue {
            number: $num,
            refinement: None,
        })
    }};

    ($key:ident s= $str:literal) => {{
        $crate::frontend::dir::DirValue::$key($crate::types::StrValue {
            value: $str.to_string(),
        })
    }};

    ($key:literal = $str:literal) => {{
        $crate::frontend::dir::DirValue::MetaData($crate::types::MetadataValue {
            key: $key.to_string(),
            value: $str.to_string(),
        })
    }};
}

#[macro_export]
#[cfg(not(feature = "connector_choice_mca_id"))]
macro_rules! dirval {
    (Connector = $name:ident) => {
        $crate::frontend::dir::DirValue::Connector(Box::new(
            $crate::frontend::ast::ConnectorChoice {
                connector: $crate::frontend::dir::enums::Connector::$name,
                sub_label: None,
            },
        ))
    };

    (Connector = ($name:ident, $sub_label:literal)) => {
        $crate::frontend::dir::DirValue::Connector(Box::new(
            $crate::frontend::ast::ConnectorChoice {
                connector: $crate::frontend::dir::enums::Connector::$name,
                sub_label: Some($sub_label.to_string()),
            },
        ))
    };

    ($key:ident = $val:ident) => {{
        pub use $crate::frontend::dir::enums::*;

        $crate::frontend::dir::DirValue::$key($key::$val)
    }};

    ($key:ident = $num:literal) => {{
        $crate::frontend::dir::DirValue::$key($crate::types::NumValue {
            number: $num,
            refinement: None,
        })
    }};

    ($key:ident s= $str:literal) => {{
        $crate::frontend::dir::DirValue::$key($crate::types::StrValue {
            value: $str.to_string(),
        })
    }};
    ($key:literal = $str:literal) => {{
        $crate::frontend::dir::DirValue::MetaData($crate::types::MetadataValue {
            key: $key.to_string(),
            value: $str.to_string(),
        })
    }};
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize)]
pub struct DirKey {
    pub kind: DirKeyKind,
    pub value: Option<String>,
}

impl DirKey {
    pub fn new(kind: DirKeyKind, value: Option<String>) -> Self {
        Self { kind, value }
    }
}

#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumVariantNames,
    strum::EnumString,
    strum::EnumMessage,
    strum::EnumProperty,
)]
pub enum DirKeyKind {
    #[strum(
        serialize = "payment_method",
        detailed_message = "Different modes of payment - eg. cards, wallets, banks",
        props(Category = "Payment Methods")
    )]
    #[serde(rename = "payment_method")]
    PaymentMethod,
    #[strum(
        serialize = "card_bin",
        detailed_message = "First 4 to 6 digits of a payment card number",
        props(Category = "Payment Methods")
    )]
    #[serde(rename = "card_bin")]
    CardBin,
    #[strum(
        serialize = "card_type",
        detailed_message = "Type of the payment card - eg. credit, debit",
        props(Category = "Payment Methods")
    )]
    #[serde(rename = "card_type")]
    CardType,
    #[strum(
        serialize = "card_network",
        detailed_message = "Network that facilitates payment card transactions",
        props(Category = "Payment Methods")
    )]
    #[serde(rename = "card_network")]
    CardNetwork,
    #[strum(
        serialize = "pay_later",
        detailed_message = "Supported types of Pay Later payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "pay_later")]
    PayLaterType,
    #[strum(
        serialize = "gift_card",
        detailed_message = "Supported types of Gift Card payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "gift_card")]
    GiftCardType,
    #[strum(
        serialize = "mandate_acceptance_type",
        detailed_message = "Mode of customer acceptance for mandates - online and offline",
        props(Category = "Payments")
    )]
    #[serde(rename = "mandate_acceptance_type")]
    MandateAcceptanceType,
    #[strum(
        serialize = "mandate_type",
        detailed_message = "Type of mandate acceptance - single use and multi use",
        props(Category = "Payments")
    )]
    #[serde(rename = "mandate_type")]
    MandateType,
    #[strum(
        serialize = "payment_type",
        detailed_message = "Indicates if a payment is mandate or non-mandate",
        props(Category = "Payments")
    )]
    #[serde(rename = "payment_type")]
    PaymentType,
    #[strum(
        serialize = "wallet",
        detailed_message = "Supported types of Wallet payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "wallet")]
    WalletType,
    #[strum(
        serialize = "upi",
        detailed_message = "Supported types of UPI payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "upi")]
    UpiType,
    #[strum(
        serialize = "voucher",
        detailed_message = "Supported types of Voucher payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "voucher")]
    VoucherType,
    #[strum(
        serialize = "bank_transfer",
        detailed_message = "Supported types of Bank Transfer payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "bank_transfer")]
    BankTransferType,
    #[strum(
        serialize = "bank_redirect",
        detailed_message = "Supported types of Bank Redirect payment methods",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "bank_redirect")]
    BankRedirectType,
    #[strum(
        serialize = "bank_debit",
        detailed_message = "Supported types of Bank Debit payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "bank_debit")]
    BankDebitType,
    #[strum(
        serialize = "crypto",
        detailed_message = "Supported types of Crypto payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "crypto")]
    CryptoType,
    #[strum(
        serialize = "metadata",
        detailed_message = "Aribitrary Key and value pair",
        props(Category = "Metadata")
    )]
    #[serde(rename = "metadata")]
    MetaData,
    #[strum(
        serialize = "reward",
        detailed_message = "Supported types of Reward payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "reward")]
    RewardType,
    #[strum(
        serialize = "amount",
        detailed_message = "Value of the transaction",
        props(Category = "Payments")
    )]
    #[serde(rename = "amount")]
    PaymentAmount,
    #[strum(
        serialize = "currency",
        detailed_message = "Currency used for the payment",
        props(Category = "Payments")
    )]
    #[serde(rename = "currency")]
    PaymentCurrency,
    #[strum(
        serialize = "authentication_type",
        detailed_message = "Type of authentication for the payment",
        props(Category = "Payments")
    )]
    #[serde(rename = "authentication_type")]
    AuthenticationType,
    #[strum(
        serialize = "capture_method",
        detailed_message = "Modes of capturing a payment",
        props(Category = "Payments")
    )]
    #[serde(rename = "capture_method")]
    CaptureMethod,
    #[strum(
        serialize = "country",
        serialize = "business_country",
        detailed_message = "Country of the business unit",
        props(Category = "Merchant")
    )]
    #[serde(rename = "business_country", alias = "country")]
    BusinessCountry,
    #[strum(
        serialize = "billing_country",
        detailed_message = "Country of the billing address of the customer",
        props(Category = "Customer")
    )]
    #[serde(rename = "billing_country")]
    BillingCountry,
    #[serde(skip_deserializing, rename = "connector")]
    #[strum(disabled)]
    Connector,
    #[strum(
        serialize = "business_label",
        detailed_message = "Identifier for business unit",
        props(Category = "Merchant")
    )]
    #[serde(rename = "business_label")]
    BusinessLabel,
    #[strum(
        serialize = "setup_future_usage",
        detailed_message = "Identifier for recurring payments",
        props(Category = "Payments")
    )]
    #[serde(rename = "setup_future_usage")]
    SetupFutureUsage,
    #[strum(
        serialize = "card_redirect_type",
        detailed_message = "Supported types of Card Redirect payment method",
        props(Category = "Payment Method Types")
    )]
    #[serde(rename = "card_redirect")]
    CardRedirectType,
}

pub trait EuclidDirFilter: Sized
where
    Self: 'static,
{
    const ALLOWED: &'static [DirKeyKind];
    fn get_allowed_keys() -> &'static [DirKeyKind] {
        Self::ALLOWED
    }

    fn is_key_allowed(key: &DirKeyKind) -> bool {
        Self::ALLOWED.contains(key)
    }
}

impl DirKeyKind {
    pub fn get_type(&self) -> types::DataType {
        match self {
            Self::PaymentMethod => types::DataType::EnumVariant,
            Self::CardBin => types::DataType::StrValue,
            Self::CardType => types::DataType::EnumVariant,
            Self::CardNetwork => types::DataType::EnumVariant,
            Self::MetaData => types::DataType::MetadataValue,
            Self::MandateType => types::DataType::EnumVariant,
            Self::PaymentType => types::DataType::EnumVariant,
            Self::MandateAcceptanceType => types::DataType::EnumVariant,
            Self::PayLaterType => types::DataType::EnumVariant,
            Self::WalletType => types::DataType::EnumVariant,
            Self::UpiType => types::DataType::EnumVariant,
            Self::VoucherType => types::DataType::EnumVariant,
            Self::BankTransferType => types::DataType::EnumVariant,
            Self::GiftCardType => types::DataType::EnumVariant,
            Self::BankRedirectType => types::DataType::EnumVariant,
            Self::CryptoType => types::DataType::EnumVariant,
            Self::RewardType => types::DataType::EnumVariant,
            Self::PaymentAmount => types::DataType::Number,
            Self::PaymentCurrency => types::DataType::EnumVariant,
            Self::AuthenticationType => types::DataType::EnumVariant,
            Self::CaptureMethod => types::DataType::EnumVariant,
            Self::BusinessCountry => types::DataType::EnumVariant,
            Self::BillingCountry => types::DataType::EnumVariant,
            Self::Connector => types::DataType::EnumVariant,
            Self::BankDebitType => types::DataType::EnumVariant,
            Self::BusinessLabel => types::DataType::StrValue,
            Self::SetupFutureUsage => types::DataType::EnumVariant,
            Self::CardRedirectType => types::DataType::EnumVariant,
        }
    }
    pub fn get_value_set(&self) -> Option<Vec<DirValue>> {
        match self {
            Self::PaymentMethod => Some(
                enums::PaymentMethod::iter()
                    .map(DirValue::PaymentMethod)
                    .collect(),
            ),
            Self::CardBin => None,
            Self::CardType => Some(enums::CardType::iter().map(DirValue::CardType).collect()),
            Self::MandateAcceptanceType => Some(
                euclid_enums::MandateAcceptanceType::iter()
                    .map(DirValue::MandateAcceptanceType)
                    .collect(),
            ),
            Self::PaymentType => Some(
                euclid_enums::PaymentType::iter()
                    .map(DirValue::PaymentType)
                    .collect(),
            ),
            Self::MandateType => Some(
                euclid_enums::MandateType::iter()
                    .map(DirValue::MandateType)
                    .collect(),
            ),
            Self::CardNetwork => Some(
                enums::CardNetwork::iter()
                    .map(DirValue::CardNetwork)
                    .collect(),
            ),
            Self::PayLaterType => Some(
                enums::PayLaterType::iter()
                    .map(DirValue::PayLaterType)
                    .collect(),
            ),
            Self::MetaData => None,
            Self::WalletType => Some(
                enums::WalletType::iter()
                    .map(DirValue::WalletType)
                    .collect(),
            ),
            Self::UpiType => Some(enums::UpiType::iter().map(DirValue::UpiType).collect()),
            Self::VoucherType => Some(
                enums::VoucherType::iter()
                    .map(DirValue::VoucherType)
                    .collect(),
            ),
            Self::BankTransferType => Some(
                enums::BankTransferType::iter()
                    .map(DirValue::BankTransferType)
                    .collect(),
            ),
            Self::GiftCardType => Some(
                enums::GiftCardType::iter()
                    .map(DirValue::GiftCardType)
                    .collect(),
            ),
            Self::BankRedirectType => Some(
                enums::BankRedirectType::iter()
                    .map(DirValue::BankRedirectType)
                    .collect(),
            ),
            Self::CryptoType => Some(
                enums::CryptoType::iter()
                    .map(DirValue::CryptoType)
                    .collect(),
            ),
            Self::RewardType => Some(
                enums::RewardType::iter()
                    .map(DirValue::RewardType)
                    .collect(),
            ),
            Self::PaymentAmount => None,
            Self::PaymentCurrency => Some(
                enums::PaymentCurrency::iter()
                    .map(DirValue::PaymentCurrency)
                    .collect(),
            ),
            Self::AuthenticationType => Some(
                enums::AuthenticationType::iter()
                    .map(DirValue::AuthenticationType)
                    .collect(),
            ),
            Self::CaptureMethod => Some(
                enums::CaptureMethod::iter()
                    .map(DirValue::CaptureMethod)
                    .collect(),
            ),
            Self::BankDebitType => Some(
                enums::BankDebitType::iter()
                    .map(DirValue::BankDebitType)
                    .collect(),
            ),
            Self::BusinessCountry => Some(
                enums::Country::iter()
                    .map(DirValue::BusinessCountry)
                    .collect(),
            ),
            Self::BillingCountry => Some(
                enums::Country::iter()
                    .map(DirValue::BillingCountry)
                    .collect(),
            ),
            Self::Connector => Some(
                enums::Connector::iter()
                    .map(|connector| {
                        DirValue::Connector(Box::new(ast::ConnectorChoice {
                            connector,
                            #[cfg(not(feature = "connector_choice_mca_id"))]
                            sub_label: None,
                        }))
                    })
                    .collect(),
            ),
            Self::BusinessLabel => None,
            Self::SetupFutureUsage => Some(
                enums::SetupFutureUsage::iter()
                    .map(DirValue::SetupFutureUsage)
                    .collect(),
            ),
            Self::CardRedirectType => Some(
                enums::CardRedirectType::iter()
                    .map(DirValue::CardRedirectType)
                    .collect(),
            ),
        }
    }
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, strum::Display, strum::EnumVariantNames,
)]
#[serde(tag = "key", content = "value")]
pub enum DirValue {
    #[serde(rename = "payment_method")]
    PaymentMethod(enums::PaymentMethod),
    #[serde(rename = "card_bin")]
    CardBin(types::StrValue),
    #[serde(rename = "card_type")]
    CardType(enums::CardType),
    #[serde(rename = "card_network")]
    CardNetwork(enums::CardNetwork),
    #[serde(rename = "metadata")]
    MetaData(types::MetadataValue),
    #[serde(rename = "pay_later")]
    PayLaterType(enums::PayLaterType),
    #[serde(rename = "wallet")]
    WalletType(enums::WalletType),
    #[serde(rename = "acceptance_type")]
    MandateAcceptanceType(euclid_enums::MandateAcceptanceType),
    #[serde(rename = "mandate_type")]
    MandateType(euclid_enums::MandateType),
    #[serde(rename = "payment_type")]
    PaymentType(euclid_enums::PaymentType),
    #[serde(rename = "upi")]
    UpiType(enums::UpiType),
    #[serde(rename = "voucher")]
    VoucherType(enums::VoucherType),
    #[serde(rename = "bank_transfer")]
    BankTransferType(enums::BankTransferType),
    #[serde(rename = "bank_redirect")]
    BankRedirectType(enums::BankRedirectType),
    #[serde(rename = "bank_debit")]
    BankDebitType(enums::BankDebitType),
    #[serde(rename = "crypto")]
    CryptoType(enums::CryptoType),
    #[serde(rename = "reward")]
    RewardType(enums::RewardType),
    #[serde(rename = "gift_card")]
    GiftCardType(enums::GiftCardType),
    #[serde(rename = "amount")]
    PaymentAmount(types::NumValue),
    #[serde(rename = "currency")]
    PaymentCurrency(enums::PaymentCurrency),
    #[serde(rename = "authentication_type")]
    AuthenticationType(enums::AuthenticationType),
    #[serde(rename = "capture_method")]
    CaptureMethod(enums::CaptureMethod),
    #[serde(rename = "business_country", alias = "country")]
    BusinessCountry(enums::Country),
    #[serde(rename = "billing_country")]
    BillingCountry(enums::Country),
    #[serde(skip_deserializing, rename = "connector")]
    Connector(Box<ast::ConnectorChoice>),
    #[serde(rename = "business_label")]
    BusinessLabel(types::StrValue),
    #[serde(rename = "setup_future_usage")]
    SetupFutureUsage(enums::SetupFutureUsage),
    #[serde(rename = "card_redirect")]
    CardRedirectType(enums::CardRedirectType),
}

impl DirValue {
    pub fn get_key(&self) -> DirKey {
        let (kind, data) = match self {
            Self::PaymentMethod(_) => (DirKeyKind::PaymentMethod, None),
            Self::CardBin(_) => (DirKeyKind::CardBin, None),
            Self::RewardType(_) => (DirKeyKind::RewardType, None),
            Self::BusinessCountry(_) => (DirKeyKind::BusinessCountry, None),
            Self::BillingCountry(_) => (DirKeyKind::CardBin, None),
            Self::BankTransferType(_) => (DirKeyKind::BankTransferType, None),
            Self::UpiType(_) => (DirKeyKind::UpiType, None),
            Self::CardType(_) => (DirKeyKind::CardType, None),
            Self::CardNetwork(_) => (DirKeyKind::CardNetwork, None),
            Self::MetaData(met) => (DirKeyKind::MetaData, Some(met.key.clone())),
            Self::PayLaterType(_) => (DirKeyKind::PayLaterType, None),
            Self::WalletType(_) => (DirKeyKind::WalletType, None),
            Self::BankRedirectType(_) => (DirKeyKind::BankRedirectType, None),
            Self::CryptoType(_) => (DirKeyKind::CryptoType, None),
            Self::AuthenticationType(_) => (DirKeyKind::AuthenticationType, None),
            Self::CaptureMethod(_) => (DirKeyKind::CaptureMethod, None),
            Self::PaymentAmount(_) => (DirKeyKind::PaymentAmount, None),
            Self::PaymentCurrency(_) => (DirKeyKind::PaymentCurrency, None),
            Self::Connector(_) => (DirKeyKind::Connector, None),
            Self::BankDebitType(_) => (DirKeyKind::BankDebitType, None),
            Self::MandateAcceptanceType(_) => (DirKeyKind::MandateAcceptanceType, None),
            Self::MandateType(_) => (DirKeyKind::MandateType, None),
            Self::PaymentType(_) => (DirKeyKind::PaymentType, None),
            Self::BusinessLabel(_) => (DirKeyKind::BusinessLabel, None),
            Self::SetupFutureUsage(_) => (DirKeyKind::SetupFutureUsage, None),
            Self::CardRedirectType(_) => (DirKeyKind::CardRedirectType, None),
            Self::VoucherType(_) => (DirKeyKind::VoucherType, None),
            Self::GiftCardType(_) => (DirKeyKind::GiftCardType, None),
        };

        DirKey::new(kind, data)
    }
    pub fn get_metadata_val(&self) -> Option<types::MetadataValue> {
        match self {
            Self::MetaData(val) => Some(val.clone()),
            Self::PaymentMethod(_) => None,
            Self::CardBin(_) => None,
            Self::CardType(_) => None,
            Self::CardNetwork(_) => None,
            Self::PayLaterType(_) => None,
            Self::WalletType(_) => None,
            Self::BankRedirectType(_) => None,
            Self::CryptoType(_) => None,
            Self::AuthenticationType(_) => None,
            Self::CaptureMethod(_) => None,
            Self::GiftCardType(_) => None,
            Self::PaymentAmount(_) => None,
            Self::PaymentCurrency(_) => None,
            Self::BusinessCountry(_) => None,
            Self::BillingCountry(_) => None,
            Self::Connector(_) => None,
            Self::BankTransferType(_) => None,
            Self::UpiType(_) => None,
            Self::BankDebitType(_) => None,
            Self::RewardType(_) => None,
            Self::VoucherType(_) => None,
            Self::MandateAcceptanceType(_) => None,
            Self::MandateType(_) => None,
            Self::PaymentType(_) => None,
            Self::BusinessLabel(_) => None,
            Self::SetupFutureUsage(_) => None,
            Self::CardRedirectType(_) => None,
        }
    }

    pub fn get_str_val(&self) -> Option<types::StrValue> {
        match self {
            Self::CardBin(val) => Some(val.clone()),
            _ => None,
        }
    }

    pub fn get_num_value(&self) -> Option<types::NumValue> {
        match self {
            Self::PaymentAmount(val) => Some(val.clone()),
            _ => None,
        }
    }

    pub fn check_equality(v1: &Self, v2: &Self) -> bool {
        match (v1, v2) {
            (Self::PaymentMethod(pm1), Self::PaymentMethod(pm2)) => pm1 == pm2,
            (Self::CardType(ct1), Self::CardType(ct2)) => ct1 == ct2,
            (Self::CardNetwork(cn1), Self::CardNetwork(cn2)) => cn1 == cn2,
            (Self::MetaData(md1), Self::MetaData(md2)) => md1 == md2,
            (Self::PayLaterType(plt1), Self::PayLaterType(plt2)) => plt1 == plt2,
            (Self::WalletType(wt1), Self::WalletType(wt2)) => wt1 == wt2,
            (Self::BankDebitType(bdt1), Self::BankDebitType(bdt2)) => bdt1 == bdt2,
            (Self::BankRedirectType(brt1), Self::BankRedirectType(brt2)) => brt1 == brt2,
            (Self::BankTransferType(btt1), Self::BankTransferType(btt2)) => btt1 == btt2,
            (Self::GiftCardType(gct1), Self::GiftCardType(gct2)) => gct1 == gct2,
            (Self::CryptoType(ct1), Self::CryptoType(ct2)) => ct1 == ct2,
            (Self::AuthenticationType(at1), Self::AuthenticationType(at2)) => at1 == at2,
            (Self::CaptureMethod(cm1), Self::CaptureMethod(cm2)) => cm1 == cm2,
            (Self::PaymentCurrency(pc1), Self::PaymentCurrency(pc2)) => pc1 == pc2,
            (Self::BusinessCountry(c1), Self::BusinessCountry(c2)) => c1 == c2,
            (Self::BillingCountry(c1), Self::BillingCountry(c2)) => c1 == c2,
            (Self::PaymentType(pt1), Self::PaymentType(pt2)) => pt1 == pt2,
            (Self::MandateType(mt1), Self::MandateType(mt2)) => mt1 == mt2,
            (Self::MandateAcceptanceType(mat1), Self::MandateAcceptanceType(mat2)) => mat1 == mat2,
            (Self::RewardType(rt1), Self::RewardType(rt2)) => rt1 == rt2,
            (Self::Connector(c1), Self::Connector(c2)) => c1 == c2,
            (Self::BusinessLabel(bl1), Self::BusinessLabel(bl2)) => bl1 == bl2,
            (Self::SetupFutureUsage(sfu1), Self::SetupFutureUsage(sfu2)) => sfu1 == sfu2,
            (Self::UpiType(ut1), Self::UpiType(ut2)) => ut1 == ut2,
            (Self::VoucherType(vt1), Self::VoucherType(vt2)) => vt1 == vt2,
            (Self::CardRedirectType(crt1), Self::CardRedirectType(crt2)) => crt1 == crt2,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DirComparisonLogic {
    NegativeConjunction,
    PositiveDisjunction,
}

#[derive(Debug, Clone)]
pub struct DirComparison {
    pub values: Vec<DirValue>,
    pub logic: DirComparisonLogic,
    pub metadata: types::Metadata,
}

pub type DirIfCondition = Vec<DirComparison>;

#[derive(Debug, Clone)]
pub struct DirIfStatement {
    pub condition: DirIfCondition,
    pub nested: Option<Vec<DirIfStatement>>,
}

#[derive(Debug, Clone)]
pub struct DirRule<O> {
    pub name: String,
    pub connector_selection: O,
    pub statements: Vec<DirIfStatement>,
}

#[derive(Debug, Clone)]
pub struct DirProgram<O> {
    pub default_selection: O,
    pub rules: Vec<DirRule<O>>,
    pub metadata: types::Metadata,
}

#[cfg(test)]
mod test {
    #![allow(clippy::expect_used)]
    use rustc_hash::FxHashMap;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn test_consistent_dir_key_naming() {
        let mut key_names: FxHashMap<DirKeyKind, String> = FxHashMap::default();

        for key in DirKeyKind::iter() {
            let json_str = if let DirKeyKind::MetaData = key {
                r#""metadata""#.to_string()
            } else {
                serde_json::to_string(&key).expect("JSON Serialization")
            };
            let display_str = key.to_string();

            assert_eq!(&json_str[1..json_str.len() - 1], display_str);
            key_names.insert(key, display_str);
        }

        let values = vec![
            dirval!(PaymentMethod = Card),
            dirval!(CardBin s= "123456"),
            dirval!(CardType = Credit),
            dirval!(CardNetwork = Visa),
            dirval!(PayLaterType = Klarna),
            dirval!(WalletType = Paypal),
            dirval!(BankRedirectType = Sofort),
            dirval!(BankDebitType = Bacs),
            dirval!(CryptoType = CryptoCurrency),
            dirval!("" = "metadata"),
            dirval!(PaymentAmount = 100),
            dirval!(PaymentCurrency = USD),
            dirval!(CardRedirectType = Benefit),
            dirval!(AuthenticationType = ThreeDs),
            dirval!(CaptureMethod = Manual),
            dirval!(BillingCountry = UnitedStatesOfAmerica),
            dirval!(BusinessCountry = France),
        ];

        for val in values {
            let json_val = serde_json::to_value(&val).expect("JSON Value Serialization");

            let json_key = json_val
                .as_object()
                .expect("Serialized Object")
                .get("key")
                .expect("Object Key");

            let value_str = json_key.as_str().expect("Value string");
            let dir_key = val.get_key();

            let key_name = key_names.get(&dir_key.kind).expect("Key name");

            assert_eq!(key_name, value_str);
        }
    }

    #[cfg(feature = "ast_parser")]
    #[test]
    fn test_allowed_dir_keys() {
        use crate::types::DummyOutput;

        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           payment_method = card
        }
        "#;
        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");

        let out = ast::lowering::lower_program::<DummyOutput>(program);
        assert!(out.is_ok())
    }
    #[cfg(feature = "ast_parser")]
    #[test]
    fn test_not_allowed_dir_keys() {
        use crate::types::DummyOutput;

        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           bank_debit = ach
        }
        "#;
        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");

        let out = ast::lowering::lower_program::<DummyOutput>(program);
        assert!(out.is_err())
    }
}
