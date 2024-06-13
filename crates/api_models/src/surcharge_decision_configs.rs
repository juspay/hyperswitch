use common_utils::{
    consts::SURCHARGE_PERCENTAGE_PRECISION_LENGTH,
    events,
    types::{MinorUnit, Percentage},
};
use euclid::frontend::{
    ast::Program,
    dir::{DirKey, EuclidDirFilter},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SurchargeDetailsOutput {
    pub surcharge: SurchargeOutput,
    pub tax_on_surcharge: Option<Percentage<SURCHARGE_PERCENTAGE_PRECISION_LENGTH>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum SurchargeOutput {
    Fixed { amount: MinorUnit },
    Rate(Percentage<SURCHARGE_PERCENTAGE_PRECISION_LENGTH>),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SurchargeDecisionConfigs {
    pub surcharge_details: Option<SurchargeDetailsOutput>,
}
impl EuclidDirFilter for SurchargeDecisionConfigs {
    const ALLOWED: &'static [DirKey] = &[
        DirKey::PaymentMethod,
        DirKey::MetaData,
        DirKey::PaymentAmount,
        DirKey::PaymentCurrency,
        DirKey::BillingCountry,
        DirKey::CardNetwork,
        DirKey::PayLaterType,
        DirKey::WalletType,
        DirKey::BankTransferType,
        DirKey::BankRedirectType,
        DirKey::BankDebitType,
        DirKey::CryptoType,
    ];
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SurchargeDecisionManagerRecord {
    pub name: String,
    pub merchant_surcharge_configs: MerchantSurchargeConfigs,
    pub algorithm: Program<SurchargeDecisionConfigs>,
    pub created_at: i64,
    pub modified_at: i64,
}

impl events::ApiEventMetric for SurchargeDecisionManagerRecord {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurchargeDecisionConfigReq {
    pub name: Option<String>,
    pub merchant_surcharge_configs: MerchantSurchargeConfigs,
    pub algorithm: Option<Program<SurchargeDecisionConfigs>>,
}

impl events::ApiEventMetric for SurchargeDecisionConfigReq {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct MerchantSurchargeConfigs {
    pub show_surcharge_breakup_screen: Option<bool>,
}

pub type SurchargeDecisionManagerResponse = SurchargeDecisionManagerRecord;
