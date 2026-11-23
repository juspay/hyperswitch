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
use crate::core::errors::RoutingError;

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
                dir::DirValue::PayLaterType(value) => Some(("pay_later", value.to_string())),
                dir::DirValue::WalletType(value) => Some(("wallet", value.to_string())),
                dir::DirValue::UpiType(value) => Some(("upi", value.to_string())),
                dir::DirValue::VoucherType(value) => Some(("voucher", value.to_string())),
                dir::DirValue::BankRedirectType(value) => {
                    Some(("bank_redirect", value.to_string()))
                }
                dir::DirValue::BankDebitType(value) => Some(("bank_debit", value.to_string())),
                dir::DirValue::BankTransferType(value) => {
                    Some(("bank_transfer", value.to_string()))
                }
                dir::DirValue::RealTimePaymentType(value) => {
                    Some(("real_time_payment", value.to_string()))
                }
                dir::DirValue::GiftCardType(value) => Some(("gift_card", value.to_string())),
                dir::DirValue::CardRedirectType(value) => {
                    Some(("card_redirect", value.to_string()))
                }
                dir::DirValue::OpenBankingType(value) => Some(("open_banking", value.to_string())),
                dir::DirValue::MobilePaymentType(value) => {
                    Some(("mobile_payment", value.to_string()))
                }
                dir::DirValue::CryptoType(value) => Some(("crypto", value.to_string())),
                dir::DirValue::RewardType(value) => Some(("reward", value.to_string())),
                dir::DirValue::NetworkTokenType(value) => {
                    Some(("network_token", value.to_string()))
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

#[cfg(test)]
mod tests {
    use api_models::routing::RoutableChoiceKind;
    use euclid::{
        backend::inputs::{
            CustomerDeviceDataInput, IssuerDataInput, MandateData, PaymentInput, PaymentMethodInput,
        },
        enums,
        frontend::dir::enums::{CustomerDevicePlatform, CustomerDeviceType, TransactionInitiator},
    };
    use serde_json::json;

    use super::*;

    fn input() -> BackendInput {
        BackendInput {
            metadata: Some(
                [("segment".to_string(), "premium".to_string())]
                    .into_iter()
                    .collect(),
            ),
            payment: PaymentInput {
                amount: MinorUnit::new(1234),
                currency: enums::Currency::USD,
                authentication_type: Some(enums::AuthenticationType::ThreeDs),
                card_bin: Some("001234".to_string()),
                extended_card_bin: Some("00123456".to_string()),
                capture_method: Some(enums::CaptureMethod::Manual),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(
                    enums::Country::UnitedKingdomOfGreatBritainAndNorthernIreland,
                ),
                business_label: Some("online".to_string()),
                setup_future_usage: Some(enums::SetupFutureUsage::OffSession),
                transaction_initiator: Some(TransactionInitiator::Merchant),
                surcharge_amount: Some(MinorUnit::new(50)),
            },
            payment_method: PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::Card),
                payment_method_type: Some(enums::PaymentMethodType::Credit),
                card_network: Some(enums::CardNetwork::Visa),
                card_discovery: None,
            },
            acquirer_data: None,
            customer_device_data: Some(CustomerDeviceDataInput {
                platform: Some(CustomerDevicePlatform::Web),
                device_type: Some(CustomerDeviceType::Desktop),
                display_size: None,
            }),
            issuer_data: Some(IssuerDataInput {
                name: Some("Issuer".to_string()),
                country: Some(enums::Country::India),
            }),
            mandate: MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: Some(enums::PaymentType::PptMandate),
            },
        }
    }

    fn connector(account: &str) -> RoutableConnectorChoice {
        RoutableConnectorChoice {
            choice_kind: RoutableChoiceKind::FullStruct,
            connector: RoutableConnectors::Stripe,
            merchant_connector_id: Some(
                MerchantConnectorAccountId::wrap(account.to_string()).expect("valid account"),
            ),
        }
    }

    #[test]
    fn builds_current_decide_contract_without_losing_account_identity() {
        let request = build_decide_request(
            "pay_test".to_string(),
            &input(),
            &[connector("mca_one"), connector("mca_two")],
            Some(vec!["stripe:mca_two".to_string()]),
            Some(RankingAlgorithm::SrBasedRouting),
        )
        .expect("valid decide request");
        let value = serde_json::to_value(request).expect("request JSON");
        for (path, expected) in [
            ("/amount", json!(1234)),
            ("/payment_method_type", json!("card")),
            ("/payment_method_sub_type", json!("credit")),
            ("/payment_type", json!("ppt_mandate")),
            ("/business_country", json!("US")),
            ("/billing_country", json!("GB")),
            ("/card/card_issuing_country_code", json!("IN")),
            ("/card/card_isin", json!("001234")),
            ("/card/card_extended_bin", json!("00123456")),
            ("/card/card_type", json!("credit")),
            ("/card/card_issuer", json!("Issuer")),
            ("/customer_device_platform", json!("web")),
            ("/customer_device_type", json!("desktop")),
            ("/transaction_initiator", json!("merchant")),
            ("/metadata/segment", json!("premium")),
            (
                "/routing_config/eligible_connectors",
                json!([
                    {"connector": "stripe", "merchant_connector_id": "mca_one"},
                    {"connector": "stripe", "merchant_connector_id": "mca_two"}
                ]),
            ),
            (
                "/routing_config/preferred_connectors",
                json!([
                    {"connector": "stripe", "merchant_connector_id": "mca_two"}
                ]),
            ),
            (
                "/routing_config/ranking_algorithm",
                json!("SR_BASED_ROUTING"),
            ),
        ] {
            assert_eq!(value.pointer(path), Some(&expected), "{path}");
        }
        for removed in [
            "merchant_id",
            "created_by",
            "order_amount",
            "payment_method",
            "card_bin",
            "issuer_country",
        ] {
            assert!(
                value.get(removed).is_none(),
                "unexpected legacy field {removed}"
            );
        }
    }

    #[test]
    fn preserves_wallet_subset_and_omits_missing_card_and_optional_fields() {
        let mut input = input();
        input.payment_method.payment_method = Some(enums::PaymentMethod::Wallet);
        input.payment_method.payment_method_type = Some(enums::PaymentMethodType::ApplePay);
        input.payment_method.card_network = None;
        input.payment.card_bin = None;
        input.payment.extended_card_bin = None;
        input.issuer_data = None;
        input.metadata = None;
        input.customer_device_data = None;
        let value = serde_json::to_value(
            build_decide_request("pay_wallet".to_string(), &input, &[], None, None)
                .expect("wallet request"),
        )
        .expect("request JSON");
        assert_eq!(
            value.pointer("/wallet").expect("serialized field"),
            "apple_pay"
        );
        assert_eq!(
            value
                .pointer("/payment_method_type")
                .expect("serialized field"),
            "wallet"
        );
        assert_eq!(
            value
                .pointer("/payment_method_sub_type")
                .expect("serialized field"),
            "apple_pay"
        );
        for absent in ["card", "metadata", "customer_device_platform"] {
            assert!(value.get(absent).is_none());
        }
        assert!(value
            .pointer("/routing_config")
            .expect("serialized field")
            .get("ranking_algorithm")
            .is_none());
        assert!(value
            .pointer("/routing_config")
            .expect("serialized field")
            .get("preferred_connectors")
            .is_none());
    }

    #[test]
    fn rejects_negative_amounts_and_surcharges_and_keeps_zero() {
        let mut input = input();
        input.payment.amount = MinorUnit::new(-1);
        assert!(build_decide_request("pay_test".to_string(), &input, &[], None, None).is_err());
        input.payment.amount = MinorUnit::new(0);
        assert_eq!(
            build_decide_request("pay_test".to_string(), &input, &[], None, None)
                .expect("zero allowed")
                .amount,
            0
        );
        input.payment.surcharge_amount = Some(MinorUnit::new(-1));
        assert!(build_decide_request("pay_test".to_string(), &input, &[], None, None).is_err());
    }

    #[test]
    fn response_uses_only_final_order_and_accepts_new_descriptive_values() {
        let response: DecideResponse = serde_json::from_value(json!({
            "connectors": [
                {"connector": "stripe", "merchant_connector_id": "mca_two"},
                {"connector": "stripe", "merchant_connector_id": "mca_one"}
            ],
            "outcome": {"type": "routed", "strategy": "future_strategy", "reason": "future_reason"},
            "success_rate": {"status": "applied", "approach": "sr_selection_v3_routing", "diagnostics": {"unstable": true}},
            "rule": {"selected": [{"connector": "adyen", "merchant_connector_id": "mca_other"}]},
            "future_section": {"value": true},
            "id": "dec_test",
            "warnings": [{"code": "future_warning"}]
        })).expect("extensible response");
        let recorded = serde_json::to_value(&response).expect("recorded response");
        for (path, expected) in [
            ("/id", json!("dec_test")),
            ("/warnings", json!([{"code": "future_warning"}])),
            ("/outcome/reason", json!("future_reason")),
            ("/success_rate/diagnostics", json!({"unstable": true})),
            ("/future_section", json!({"value": true})),
        ] {
            assert_eq!(recorded.pointer(path), Some(&expected), "{path}");
        }
        let outcome = normalize_decide_routing_response(&response).expect("normalized response");
        assert_eq!(
            outcome
                .connectors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["stripe:mca_two", "stripe:mca_one"]
        );
        assert!(matches!(
            outcome.routing_approach,
            RoutingApproach::Exploitation
        ));
    }

    #[test]
    fn normalizes_lowercase_hedging_and_rule_routing() {
        for (strategy, success_rate, expected) in [
            (
                "success_rate",
                json!({"status": "applied", "approach": "sr_v3_hedging"}),
                "Exploration",
            ),
            ("rule", json!(null), "StaticRouting"),
            (
                "candidate_order",
                json!({"status": "failed", "approach": "sr_selection_v3_routing"}),
                "Default",
            ),
            (
                "future_strategy",
                json!({"status": "applied", "approach": "future_approach"}),
                "Default",
            ),
        ] {
            let response: DecideResponse = serde_json::from_value(json!({
                "connectors": [{"connector": "stripe", "merchant_connector_id": "mca_one"}],
                "outcome": {"type": "routed", "strategy": strategy},
                "success_rate": success_rate
            }))
            .expect("valid response");
            assert_eq!(
                normalize_decide_routing_response(&response)
                    .expect("normalized response")
                    .routing_approach
                    .to_string(),
                expected
            );
        }
    }

    #[test]
    fn empty_results_preserve_hs_fallback_and_malformed_results_are_rejected() {
        let response: DecideResponse = serde_json::from_value(json!({
            "connectors": [], "outcome": {"type": "no_connector", "strategy": "none"},
            "rule": {"selected": [{"connector": "stripe", "merchant_connector_id": "mca_one"}]}
        }))
        .expect("empty response");
        assert!(normalize_decide_routing_response(&response)
            .expect("empty outcome")
            .connectors
            .is_empty());
        for value in [
            json!({"outcome": {"type": "routed", "strategy": "rule"}}),
            json!({"connectors": [], "outcome": null}),
            json!({"connectors": [{"connector": "unknown_provider", "merchant_connector_id": "mca_one"}], "outcome": {"type": "routed", "strategy": "rule"}}),
            json!({"connectors": [{"connector": "stripe", "merchant_connector_id": 42}], "outcome": {"type": "routed", "strategy": "rule"}}),
        ] {
            assert!(serde_json::from_value::<DecideResponse>(value).is_err());
        }
    }
}
