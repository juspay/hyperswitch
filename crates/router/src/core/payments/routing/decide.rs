//! The `/decide` wire contract and its adapter to Hyperswitch's routing result.
//!
//! Keep this separate from the hybrid DTOs: the two endpoints share routing behavior,
//! but `/decide` takes merchant scope in a header and returns one authoritative order.

use std::{collections::BTreeMap, str::FromStr};

use api_models::{
    open_router::RankingAlgorithm,
    routing::{DeRoutableConnectorChoice, RoutableConnectorChoice},
};
use common_utils::{id_type::MerchantConnectorAccountId, types::MinorUnit};
use error_stack::ResultExt;
use euclid::{
    backend::BackendInput,
    enums::RoutableConnectors,
    frontend::dir::{self, transformers::IntoDirValue},
};
use serde::{Deserialize, Serialize};

use super::{
    utils::{HybridRoutingOutcome, RoutingApproach},
    RoutingResult,
};
use crate::{
    consts::decision_engine_payment_method as payment_method_keys, core::errors::RoutingError,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecideRequest {
    pub payment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_reference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub amount: u64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method_sub_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_future_usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_initiator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surcharge_amount: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<DecideCard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_device_platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_device_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_device_display_size: Option<String>,
    // Only the supported method subset keys below are inserted by the adapter.
    #[serde(flatten)]
    pub payment_method_details: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    pub routing_config: DecideRoutingConfig,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct DecideCard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_isin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_extended_bin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_issuing_country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_discovery: Option<String>,
}

impl DecideCard {
    fn is_empty(&self) -> bool {
        self.card_isin.is_none()
            && self.card_extended_bin.is_none()
            && self.card_network.is_none()
            && self.card_type.is_none()
            && self.card_issuer.is_none()
            && self.card_issuing_country_code.is_none()
            && self.card_discovery.is_none()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecideRoutingConfig {
    pub algorithm_for: String,
    pub eligible_connectors: Vec<DecideConnector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_connectors: Option<Vec<DecideConnector>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_algorithm: Option<RankingAlgorithm>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecideConnector {
    pub connector: RoutableConnectors,
    pub merchant_connector_id: Option<MerchantConnectorAccountId>,
}

impl From<&RoutableConnectorChoice> for DecideConnector {
    fn from(choice: &RoutableConnectorChoice) -> Self {
        Self {
            connector: choice.connector,
            merchant_connector_id: choice.merchant_connector_id.clone(),
        }
    }
}

impl From<DecideConnector> for RoutableConnectorChoice {
    fn from(choice: DecideConnector) -> Self {
        DeRoutableConnectorChoice {
            gateway_name: choice.connector,
            gateway_id: choice.merchant_connector_id,
        }
        .into()
    }
}

// Read only stable fields HS consumes. Additional sections and new descriptive
// outcome values must not break deserialization of a usable connector order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecideResponse {
    pub connectors: Vec<DecideConnector>,
    pub outcome: DecideOutcome,
    pub success_rate: Option<DecideSuccessRate>,
    #[serde(flatten)]
    pub additional_fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecideOutcome {
    #[serde(rename = "type")]
    pub kind: String,
    pub strategy: String,
    #[serde(flatten)]
    pub additional_fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DecideSuccessRate {
    pub status: String,
    pub approach: Option<String>,
    #[serde(flatten)]
    pub additional_fields: BTreeMap<String, serde_json::Value>,
}

fn unsigned_amount(amount: MinorUnit, field: &str) -> RoutingResult<u64> {
    u64::try_from(amount.get_amount_as_i64()).change_context(
        RoutingError::DecisionEngineValidationError(format!(
            "{field} must be nonnegative for /decide"
        )),
    )
}

fn preferred_connector(identifier: String) -> RoutingResult<DecideConnector> {
    let (name, account) = identifier
        .split_once(':')
        .map_or((identifier.as_str(), None), |(name, account)| {
            (name, Some(account))
        });
    let connector = RoutableConnectors::from_str(name)
        .change_context(RoutingError::InvalidConnectorName(name.to_string()))?;
    let merchant_connector_id = account
        .map(|account| {
            if account.is_empty() {
                return Err(RoutingError::DecisionEngineValidationError(
                    "preferred connector account must be nonempty".to_string(),
                )
                .into());
            }
            MerchantConnectorAccountId::wrap(account.to_string()).change_context(
                RoutingError::DecisionEngineValidationError(
                    "invalid preferred connector account".to_string(),
                ),
            )
        })
        .transpose()?;
    Ok(DecideConnector {
        connector,
        merchant_connector_id,
    })
}

pub(super) fn build_decide_request(
    payment_id: String,
    input: &BackendInput,
    eligible_connectors: &[RoutableConnectorChoice],
    preferred_connectors: Option<Vec<String>>,
    ranking_algorithm: Option<RankingAlgorithm>,
) -> RoutingResult<DecideRequest> {
    let payment = &input.payment;
    let method = &input.payment_method;
    let issuer = input.issuer_data.as_ref();
    let device = input.customer_device_data.as_ref();
    let mut card = DecideCard {
        card_isin: payment.card_bin.clone(),
        card_extended_bin: payment.extended_card_bin.clone(),
        card_network: method.card_network.as_ref().map(ToString::to_string),
        card_type: None,
        card_issuer: issuer.and_then(|issuer| issuer.name.clone()),
        card_issuing_country_code: issuer
            .and_then(|issuer| issuer.country.as_ref())
            .map(|country| country.to_alpha2().to_string()),
        card_discovery: method.card_discovery.as_ref().map(ToString::to_string),
    };
    let mut payment_method_details = BTreeMap::new();
    if let (Some(subtype), Some(category)) = (method.payment_method_type, method.payment_method) {
        if let Ok(value) = (subtype, category).into_dir_value() {
            let subset = match value {
                dir::DirValue::CardType(value) => {
                    card.card_type = Some(value.to_string());
                    None
                }
                dir::DirValue::PayLaterType(value) => {
                    Some((payment_method_keys::PAY_LATER, value.to_string()))
                }
                dir::DirValue::WalletType(value) => {
                    Some((payment_method_keys::WALLET, value.to_string()))
                }
                dir::DirValue::UpiType(value) => {
                    Some((payment_method_keys::UPI, value.to_string()))
                }
                dir::DirValue::VoucherType(value) => {
                    Some((payment_method_keys::VOUCHER, value.to_string()))
                }
                dir::DirValue::BankRedirectType(value) => {
                    Some((payment_method_keys::BANK_REDIRECT, value.to_string()))
                }
                dir::DirValue::BankDebitType(value) => {
                    Some((payment_method_keys::BANK_DEBIT, value.to_string()))
                }
                dir::DirValue::BankTransferType(value) => {
                    Some((payment_method_keys::BANK_TRANSFER, value.to_string()))
                }
                dir::DirValue::RealTimePaymentType(value) => {
                    Some((payment_method_keys::REAL_TIME_PAYMENT, value.to_string()))
                }
                dir::DirValue::GiftCardType(value) => {
                    Some((payment_method_keys::GIFT_CARD, value.to_string()))
                }
                dir::DirValue::CardRedirectType(value) => {
                    Some((payment_method_keys::CARD_REDIRECT, value.to_string()))
                }
                dir::DirValue::OpenBankingType(value) => {
                    Some((payment_method_keys::OPEN_BANKING, value.to_string()))
                }
                dir::DirValue::MobilePaymentType(value) => {
                    Some((payment_method_keys::MOBILE_PAYMENT, value.to_string()))
                }
                dir::DirValue::CryptoType(value) => {
                    Some((payment_method_keys::CRYPTO, value.to_string()))
                }
                dir::DirValue::RewardType(value) => {
                    Some((payment_method_keys::REWARD, value.to_string()))
                }
                dir::DirValue::NetworkTokenType(value) => {
                    Some((payment_method_keys::NETWORK_TOKEN, value.to_string()))
                }
                _ => None,
            };
            if let Some((key, value)) = subset {
                payment_method_details.insert(key.to_string(), value);
            }
        }
    }

    Ok(DecideRequest {
        payment_id,
        merchant_reference_id: None,
        customer_id: None,
        amount: unsigned_amount(payment.amount, "amount")?,
        currency: payment.currency.to_string(),
        payment_method_type: method.payment_method.as_ref().map(ToString::to_string),
        payment_method_sub_type: method.payment_method_type.as_ref().map(ToString::to_string),
        authentication_type: payment
            .authentication_type
            .as_ref()
            .map(ToString::to_string),
        capture_method: payment.capture_method.as_ref().map(ToString::to_string),
        payment_type: input.mandate.payment_type.as_ref().map(ToString::to_string),
        setup_future_usage: payment.setup_future_usage.as_ref().map(ToString::to_string),
        transaction_initiator: payment
            .transaction_initiator
            .as_ref()
            .map(ToString::to_string),
        surcharge_amount: payment
            .surcharge_amount
            .map(|amount| unsigned_amount(amount, "surcharge_amount"))
            .transpose()?,
        business_country: payment
            .business_country
            .as_ref()
            .map(|country| country.to_alpha2().to_string()),
        billing_country: payment
            .billing_country
            .as_ref()
            .map(|country| country.to_alpha2().to_string()),
        business_label: payment.business_label.clone(),
        card: (!card.is_empty()).then_some(card),
        customer_device_platform: device
            .and_then(|device| device.platform.as_ref())
            .map(ToString::to_string),
        customer_device_type: device
            .and_then(|device| device.device_type.as_ref())
            .map(ToString::to_string),
        customer_device_display_size: device
            .and_then(|device| device.display_size.as_ref())
            .map(ToString::to_string),
        payment_method_details,
        metadata: input.metadata.as_ref().map(|metadata| {
            metadata
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        }),
        routing_config: DecideRoutingConfig {
            algorithm_for: "payment".to_string(),
            eligible_connectors: eligible_connectors
                .iter()
                .map(DecideConnector::from)
                .collect(),
            preferred_connectors: preferred_connectors
                .map(|connectors| connectors.into_iter().map(preferred_connector).collect())
                .transpose()?,
            ranking_algorithm,
        },
    })
}

pub(super) fn normalize_decide_routing_response(
    response: &DecideResponse,
) -> RoutingResult<HybridRoutingOutcome> {
    if response.connectors.is_empty() || response.outcome.kind == "no_connector" {
        return Ok(HybridRoutingOutcome::empty());
    }
    let routing_approach = response
        .success_rate
        .as_ref()
        .filter(|success_rate| success_rate.status == "applied")
        .and_then(|success_rate| success_rate.approach.as_ref())
        .map(|approach| {
            RoutingApproach::from_decision_engine_approach(&approach.to_ascii_uppercase())
        })
        .unwrap_or_else(|| {
            if response.outcome.strategy == "rule" {
                RoutingApproach::StaticRouting
            } else {
                RoutingApproach::Default
            }
        });
    Ok(HybridRoutingOutcome {
        connectors: response
            .connectors
            .iter()
            .cloned()
            .map(RoutableConnectorChoice::from)
            .collect(),
        routing_approach,
    })
}
