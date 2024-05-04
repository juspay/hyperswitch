pub mod transformers;

use euclid_macros::EnumNums;
use serde::Serialize;
use strum::VariantNames;

use crate::{
    dssa::types::EuclidAnalysable,
    enums,
    frontend::{
        ast,
        dir::{DirKeyKind, DirValue, EuclidDirFilter},
    },
};

pub type Metadata = std::collections::HashMap<String, serde_json::Value>;

#[derive(
    Debug,
    Clone,
    EnumNums,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumVariantNames,
    strum::EnumString,
)]
pub enum EuclidKey {
    #[strum(serialize = "payment_method")]
    PaymentMethod,
    #[strum(serialize = "card_bin")]
    CardBin,
    #[strum(serialize = "metadata")]
    Metadata,
    #[strum(serialize = "mandate_type")]
    MandateType,
    #[strum(serialize = "mandate_acceptance_type")]
    MandateAcceptanceType,
    #[strum(serialize = "payment_type")]
    PaymentType,
    #[strum(serialize = "payment_method_type")]
    PaymentMethodType,
    #[strum(serialize = "card_network")]
    CardNetwork,
    #[strum(serialize = "authentication_type")]
    AuthenticationType,
    #[strum(serialize = "capture_method")]
    CaptureMethod,
    #[strum(serialize = "amount")]
    PaymentAmount,
    #[strum(serialize = "currency")]
    PaymentCurrency,
    #[strum(serialize = "country", to_string = "business_country")]
    BusinessCountry,
    #[strum(serialize = "billing_country")]
    BillingCountry,
    #[strum(serialize = "business_label")]
    BusinessLabel,
    #[strum(serialize = "setup_future_usage")]
    SetupFutureUsage,
}
impl EuclidDirFilter for DummyOutput {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::AuthenticationType,
        DirKeyKind::PaymentMethod,
        DirKeyKind::CardType,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::CaptureMethod,
        DirKeyKind::AuthenticationType,
        DirKeyKind::CardBin,
        DirKeyKind::PayLaterType,
        DirKeyKind::PaymentAmount,
        DirKeyKind::MetaData,
        DirKeyKind::MandateAcceptanceType,
        DirKeyKind::MandateType,
        DirKeyKind::PaymentType,
        DirKeyKind::SetupFutureUsage,
    ];
}
impl EuclidAnalysable for DummyOutput {
    fn get_dir_value_for_analysis(&self, rule_name: String) -> Vec<(DirValue, Metadata)> {
        self.outputs
            .iter()
            .map(|dummyc| {
                let metadata_key = "MetadataKey".to_string();
                let metadata_value = dummyc;
                (
                    DirValue::MetaData(MetadataValue {
                        key: metadata_key.clone(),
                        value: metadata_value.clone(),
                    }),
                    std::collections::HashMap::from_iter([(
                        "DUMMY_OUTPUT".to_string(),
                        serde_json::json!({
                            "rule_name":rule_name,
                             "Metadata_Key" :metadata_key,
                             "Metadata_Value" : metadata_value,
                        }),
                    )]),
                )
            })
            .collect()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct DummyOutput {
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DataType {
    Number,
    EnumVariant,
    MetadataValue,
    StrValue,
}

impl EuclidKey {
    pub fn key_type(&self) -> DataType {
        match self {
            Self::PaymentMethod => DataType::EnumVariant,
            Self::CardBin => DataType::StrValue,
            Self::Metadata => DataType::MetadataValue,
            Self::PaymentMethodType => DataType::EnumVariant,
            Self::CardNetwork => DataType::EnumVariant,
            Self::AuthenticationType => DataType::EnumVariant,
            Self::CaptureMethod => DataType::EnumVariant,
            Self::PaymentAmount => DataType::Number,
            Self::PaymentCurrency => DataType::EnumVariant,
            Self::BusinessCountry => DataType::EnumVariant,
            Self::BillingCountry => DataType::EnumVariant,
            Self::MandateType => DataType::EnumVariant,
            Self::MandateAcceptanceType => DataType::EnumVariant,
            Self::PaymentType => DataType::EnumVariant,
            Self::BusinessLabel => DataType::StrValue,
            Self::SetupFutureUsage => DataType::EnumVariant,
        }
    }
}

enums::collect_variants!(EuclidKey);

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumValueRefinement {
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanEqual,
    LessThanEqual,
}

impl From<ast::ComparisonType> for Option<NumValueRefinement> {
    fn from(comp_type: ast::ComparisonType) -> Self {
        match comp_type {
            ast::ComparisonType::Equal => None,
            ast::ComparisonType::NotEqual => Some(NumValueRefinement::NotEqual),
            ast::ComparisonType::GreaterThan => Some(NumValueRefinement::GreaterThan),
            ast::ComparisonType::LessThan => Some(NumValueRefinement::LessThan),
            ast::ComparisonType::LessThanEqual => Some(NumValueRefinement::LessThanEqual),
            ast::ComparisonType::GreaterThanEqual => Some(NumValueRefinement::GreaterThanEqual),
        }
    }
}

impl From<NumValueRefinement> for ast::ComparisonType {
    fn from(value: NumValueRefinement) -> Self {
        match value {
            NumValueRefinement::NotEqual => Self::NotEqual,
            NumValueRefinement::LessThan => Self::LessThan,
            NumValueRefinement::GreaterThan => Self::GreaterThan,
            NumValueRefinement::GreaterThanEqual => Self::GreaterThanEqual,
            NumValueRefinement::LessThanEqual => Self::LessThanEqual,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct StrValue {
    pub value: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct MetadataValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub struct NumValue {
    pub number: i64,
    pub refinement: Option<NumValueRefinement>,
}

impl NumValue {
    pub fn fits(&self, other: &Self) -> bool {
        let this_num = self.number;
        let other_num = other.number;

        match (&self.refinement, &other.refinement) {
            (None, None) => this_num == other_num,

            (Some(NumValueRefinement::GreaterThan), None) => other_num > this_num,

            (Some(NumValueRefinement::LessThan), None) => other_num < this_num,

            (Some(NumValueRefinement::NotEqual), Some(NumValueRefinement::NotEqual)) => {
                other_num == this_num
            }

            (Some(NumValueRefinement::GreaterThan), Some(NumValueRefinement::GreaterThan)) => {
                other_num > this_num
            }
            (Some(NumValueRefinement::LessThan), Some(NumValueRefinement::LessThan)) => {
                other_num < this_num
            }

            (Some(NumValueRefinement::GreaterThanEqual), None) => other_num >= this_num,
            (Some(NumValueRefinement::LessThanEqual), None) => other_num <= this_num,
            (
                Some(NumValueRefinement::GreaterThanEqual),
                Some(NumValueRefinement::GreaterThanEqual),
            ) => other_num >= this_num,

            (Some(NumValueRefinement::LessThanEqual), Some(NumValueRefinement::LessThanEqual)) => {
                other_num <= this_num
            }

            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EuclidValue {
    PaymentMethod(enums::PaymentMethod),
    CardBin(StrValue),
    Metadata(MetadataValue),
    PaymentMethodType(enums::PaymentMethodType),
    CardNetwork(enums::CardNetwork),
    AuthenticationType(enums::AuthenticationType),
    CaptureMethod(enums::CaptureMethod),
    PaymentType(enums::PaymentType),
    MandateAcceptanceType(enums::MandateAcceptanceType),
    MandateType(enums::MandateType),
    PaymentAmount(NumValue),
    PaymentCurrency(enums::Currency),
    BusinessCountry(enums::Country),
    BillingCountry(enums::Country),
    BusinessLabel(StrValue),
    SetupFutureUsage(enums::SetupFutureUsage),
}

impl EuclidValue {
    pub fn get_num_value(&self) -> Option<NumValue> {
        match self {
            Self::PaymentAmount(val) => Some(val.clone()),
            _ => None,
        }
    }

    pub fn get_key(&self) -> EuclidKey {
        match self {
            Self::PaymentMethod(_) => EuclidKey::PaymentMethod,
            Self::CardBin(_) => EuclidKey::CardBin,
            Self::Metadata(_) => EuclidKey::Metadata,
            Self::PaymentMethodType(_) => EuclidKey::PaymentMethodType,
            Self::MandateType(_) => EuclidKey::MandateType,
            Self::PaymentType(_) => EuclidKey::PaymentType,
            Self::MandateAcceptanceType(_) => EuclidKey::MandateAcceptanceType,
            Self::CardNetwork(_) => EuclidKey::CardNetwork,
            Self::AuthenticationType(_) => EuclidKey::AuthenticationType,
            Self::CaptureMethod(_) => EuclidKey::CaptureMethod,
            Self::PaymentAmount(_) => EuclidKey::PaymentAmount,
            Self::PaymentCurrency(_) => EuclidKey::PaymentCurrency,
            Self::BusinessCountry(_) => EuclidKey::BusinessCountry,
            Self::BillingCountry(_) => EuclidKey::BillingCountry,
            Self::BusinessLabel(_) => EuclidKey::BusinessLabel,
            Self::SetupFutureUsage(_) => EuclidKey::SetupFutureUsage,
        }
    }
}

#[cfg(test)]
mod global_type_tests {
    use super::*;

    #[test]
    fn test_num_value_fits_greater_than() {
        let val1 = NumValue {
            number: 10,
            refinement: Some(NumValueRefinement::GreaterThan),
        };
        let val2 = NumValue {
            number: 30,
            refinement: Some(NumValueRefinement::GreaterThan),
        };

        assert!(val1.fits(&val2))
    }

    #[test]
    fn test_num_value_fits_less_than() {
        let val1 = NumValue {
            number: 30,
            refinement: Some(NumValueRefinement::LessThan),
        };
        let val2 = NumValue {
            number: 10,
            refinement: Some(NumValueRefinement::LessThan),
        };

        assert!(val1.fits(&val2));
    }
}
