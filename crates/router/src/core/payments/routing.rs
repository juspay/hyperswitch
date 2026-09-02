use hyperswitch_domain_models::mandates;
mod transformers;
pub mod utils;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use std::collections::hash_map;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use std::hash::{Hash, Hasher};
use std::{collections::HashMap, future::Future, pin::Pin, str::FromStr, sync::Arc};

#[cfg(feature = "v1")]
use api_models::open_router::{self as or_types, DecidedGateway, OpenRouterDecideGatewayRequest};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::routing as api_routing;
use api_models::{
    admin as admin_api,
    enums::{self as api_enums, CountryAlpha2},
    routing::ConnectorSelection,
};
use common_types::payments as common_payments_types;
use common_utils::ext_traits::AsyncExt;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use euclid::{
    backend::{self, inputs as dsl_inputs, EuclidBackend},
    dssa::graph::{self as euclid_graph, CgraphExt},
    enums as euclid_enums,
    frontend::{ast, dir as euclid_dir},
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    elimination_based_client::EliminationBasedRouting,
    success_rate_client::SuccessBasedDynamicRouting, DynamicRoutingError,
};
use hyperswitch_domain_models::{
    address::Address,
    routing::{PreRoutingConnectorChoice, RoutingData},
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_interfaces::events::routing_api_logs::{ApiMethod, RoutingEngine};
use hyperswitch_masking::{PeekInterface, Secret};
use kgraph_utils::{
    mca as mca_graph,
    transformers::{IntoContext, IntoDirValue},
    types::CountryCurrencyFilter,
};
use rand::distributions::{self, Distribution};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use rand::SeedableRng;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;
use storage_impl::redis::cache::{CacheKey, CGRAPH_CACHE, ROUTING_CACHE};

#[cfg(feature = "v2")]
use crate::core::admin;
#[cfg(feature = "payouts")]
use crate::core::payouts;
#[cfg(feature = "v1")]
use crate::core::routing::transformers::OpenRouterDecideGatewayRequestExt;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use crate::routes::app::SessionStateInfo;
use crate::{
    core::{
        configs::dimension_state,
        errors, errors as oss_errors,
        payments::{
            routing::utils::DecisionEngineApiHandler, OperationSessionGetters,
            OperationSessionSetters,
        },
        routing,
    },
    logger, services,
    types::{
        api::{self, routing as routing_types},
        domain, storage as oss_storage,
        transformers::{ForeignFrom, ForeignInto, ForeignTryFrom},
    },
    utils::{OptionExt, ValueExt},
    SessionState,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum CachedAlgorithm {
    Single(Box<routing_types::RoutableConnectorChoice>),
    Priority(Vec<routing_types::RoutableConnectorChoice>),
    VolumeSplit(Vec<routing_types::ConnectorVolumeSplit>),
    Advanced(backend::VirInterpreterBackend<ConnectorSelection>),
}

#[cfg(feature = "v1")]
pub struct SessionFlowRoutingInput<'a> {
    pub state: &'a SessionState,
    pub country: Option<CountryAlpha2>,
    pub key_store: &'a domain::MerchantKeyStore,
    pub merchant_account: &'a domain::MerchantAccount,
    pub payment_attempt: &'a oss_storage::PaymentAttempt,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: api::SessionConnectorDatas,
}

#[cfg(feature = "v2")]
pub struct SessionFlowRoutingInput<'a> {
    pub country: Option<CountryAlpha2>,
    pub payment_intent: &'a oss_storage::PaymentIntent,
    pub chosen: api::SessionConnectorDatas,
}

#[allow(dead_code)]
#[cfg(feature = "v1")]
pub struct SessionRoutingPmTypeInput<'a> {
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<SessionRoutingConnectorKey, api::GetToken>,
    profile_id: &'a common_utils::id_type::ProfileId,
    /// Resolves whether this profile is cut over to the Decision Engine.
    dimensions: &'a dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    payment_id: String,
}

#[cfg(feature = "v2")]
pub struct SessionRoutingPmTypeInput<'a> {
    routing_algorithm: &'a MerchantAccountRoutingAlgorithm,
    backend_input: dsl_inputs::BackendInput,
    allowed_connectors: FxHashMap<SessionRoutingConnectorKey, api::GetToken>,
    profile_id: &'a common_utils::id_type::ProfileId,
}

type RoutingResult<O> = oss_errors::CustomResult<O, errors::RoutingError>;

type SessionRoutingConnectorKey = Option<common_utils::id_type::MerchantConnectorAccountId>;

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MerchantAccountRoutingAlgorithm {
    V1(routing_types::RoutingAlgorithmRef),
}

#[cfg(feature = "v1")]
impl Default for MerchantAccountRoutingAlgorithm {
    fn default() -> Self {
        Self::V1(routing_types::RoutingAlgorithmRef::default())
    }
}

#[cfg(feature = "v2")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum MerchantAccountRoutingAlgorithm {
    V1(Option<common_utils::id_type::RoutingId>),
}

#[cfg(feature = "payouts")]
pub fn make_dsl_input_for_payouts(
    payout_data: &payouts::PayoutData,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate = dsl_inputs::MandateData {
        mandate_acceptance_type: None,
        mandate_type: None,
        payment_type: None,
    };
    let metadata = payout_data
        .payouts
        .metadata
        .clone()
        .map(|val| val.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payouts")
        .unwrap_or(None);
    let payment = dsl_inputs::PaymentInput {
        amount: payout_data.payouts.amount,
        card_bin: None,
        transaction_initiator: None,
        extended_card_bin: None,
        currency: payout_data.payouts.destination_currency,
        authentication_type: None,
        capture_method: None,
        business_country: payout_data
            .payout_attempt
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: payout_data
            .billing_address
            .as_ref()
            .and_then(|ba| ba.address.as_ref())
            .and_then(|addr| addr.country)
            .map(api_enums::Country::from_alpha2),
        business_label: payout_data.payout_attempt.business_label.clone(),
        setup_future_usage: None,
        surcharge_amount: None,
    };
    let payment_method = dsl_inputs::PaymentMethodInput {
        payment_method: payout_data
            .payouts
            .payout_type
            .map(api_enums::PaymentMethod::foreign_from),
        payment_method_type: payout_data
            .payout_method_data
            .as_ref()
            .map(api_enums::PaymentMethodType::foreign_from)
            .or_else(|| {
                payout_data.payment_method.as_ref().and_then(|pm| {
                    #[cfg(feature = "v1")]
                    {
                        pm.payment_method_type
                    }
                    #[cfg(feature = "v2")]
                    {
                        pm.payment_method_subtype
                    }
                })
            }),
        card_network: None,
        card_discovery: None,
    };
    Ok(dsl_inputs::BackendInput {
        mandate,
        metadata,
        payment,
        payment_method,
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    })
}

#[cfg(feature = "v2")]
pub fn make_dsl_input(
    payments_dsl_input: &routing::PaymentsDslInput<'_>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: payments_dsl_input.setup_mandate.as_ref().and_then(
            |mandate_data| {
                mandate_data
                    .customer_acceptance
                    .as_ref()
                    .map(|customer_accept| match customer_accept.acceptance_type {
                        common_payments_types::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        common_payments_types::AcceptanceType::Offline => {
                            euclid_enums::MandateAcceptanceType::Offline
                        }
                    })
            },
        ),
        mandate_type: payments_dsl_input
            .setup_mandate
            .as_ref()
            .and_then(|mandate_data| {
                mandate_data
                    .mandate_type
                    .clone()
                    .map(|mandate_type| match mandate_type {
                        mandates::MandateDataType::SingleUse(_) => {
                            euclid_enums::MandateType::SingleUse
                        }
                        mandates::MandateDataType::MultiUse(_) => {
                            euclid_enums::MandateType::MultiUse
                        }
                    })
            }),
        payment_type: Some(
            if payments_dsl_input
                .recurring_details
                .as_ref()
                .is_some_and(|data| {
                    matches!(
                        data,
                        api_models::mandates::RecurringDetails::ProcessorPaymentToken(_)
                    )
                })
            {
                euclid_enums::PaymentType::PptMandate
            } else {
                payments_dsl_input.setup_mandate.map_or_else(
                    || euclid_enums::PaymentType::NonMandate,
                    |_| euclid_enums::PaymentType::SetupMandate,
                )
            },
        ),
    };
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: Some(payments_dsl_input.payment_attempt.payment_method_type),
        payment_method_type: payments_dsl_input.payment_attempt.payment_method_subtype,
        card_network: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => card.card_network.clone(),

                _ => None,
            }),
        card_discovery: None,
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: payments_dsl_input
            .payment_attempt
            .amount_details
            .get_net_amount(),
        card_bin: payments_dsl_input.payment_method_data.as_ref().and_then(
            |pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => Some(card.card_number.get_card_isin()),
                _ => None,
            },
        ),
        transaction_initiator: None,
        extended_card_bin: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    Some(card.card_number.peek().chars().take(8).collect::<String>())
                }
                _ => None,
            }),
        currency: payments_dsl_input.currency,
        authentication_type: Some(payments_dsl_input.payment_attempt.authentication_type),
        capture_method: Some(payments_dsl_input.payment_intent.capture_method),
        business_country: None,
        billing_country: payments_dsl_input
            .address
            .get_payment_method_billing()
            .and_then(|billing_address| billing_address.address.as_ref())
            .and_then(|address_details| address_details.country)
            .map(api_enums::Country::from_alpha2),
        business_label: None,
        setup_future_usage: Some(payments_dsl_input.payment_intent.setup_future_usage),
        surcharge_amount: None,
    };

    let metadata = payments_dsl_input
        .payment_intent
        .metadata
        .clone()
        .map(|value| value.parse_value("routing_parameters"))
        .transpose()
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    Ok(dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    })
}

#[cfg(feature = "v1")]
pub fn make_dsl_input(
    payments_dsl_input: &routing::PaymentsDslInput<'_>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: payments_dsl_input.setup_mandate.as_ref().and_then(
            |mandate_data| {
                mandate_data
                    .customer_acceptance
                    .as_ref()
                    .map(|cat| match cat.acceptance_type {
                        common_payments_types::AcceptanceType::Online => {
                            euclid_enums::MandateAcceptanceType::Online
                        }
                        common_payments_types::AcceptanceType::Offline => {
                            euclid_enums::MandateAcceptanceType::Offline
                        }
                    })
            },
        ),
        mandate_type: payments_dsl_input
            .setup_mandate
            .as_ref()
            .and_then(|mandate_data| {
                mandate_data.mandate_type.clone().map(|mt| match mt {
                    mandates::MandateDataType::SingleUse(_) => euclid_enums::MandateType::SingleUse,
                    mandates::MandateDataType::MultiUse(_) => euclid_enums::MandateType::MultiUse,
                })
            }),
        payment_type: Some(
            if payments_dsl_input
                .recurring_details
                .as_ref()
                .is_some_and(|data| {
                    matches!(
                        data,
                        api_models::mandates::RecurringDetails::ProcessorPaymentToken(_)
                    )
                })
            {
                euclid_enums::PaymentType::PptMandate
            } else {
                payments_dsl_input.setup_mandate.map_or_else(
                    || euclid_enums::PaymentType::NonMandate,
                    |_| euclid_enums::PaymentType::SetupMandate,
                )
            },
        ),
    };
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: payments_dsl_input.payment_attempt.payment_method,
        payment_method_type: payments_dsl_input.payment_attempt.payment_method_type,
        card_network: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => card.card_network.clone(),
                domain::PaymentMethodData::CardWithOptionalCVC(card) => card.card_network.clone(),
                domain::PaymentMethodData::CardWithNetworkTokenDetails(
                    card_with_network_token_details,
                ) => card_with_network_token_details
                    .card_details
                    .card_network
                    .clone(),
                domain::PaymentMethodData::CardDetailsForNetworkTransactionId(
                    card_details_for_ntid,
                ) => card_details_for_ntid.card_network.clone(),
                domain::PaymentMethodData::CardWithLimitedDetails(card_with_limited_details) => {
                    card_with_limited_details.card_network.clone()
                }
                domain::PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(
                    network_token_details_for_ntid,
                ) => network_token_details_for_ntid.card_network.clone(),
                domain::PaymentMethodData::NetworkToken(network_token_details) => {
                    network_token_details.card_network.clone()
                }
                domain::PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(
                    _,
                )
                | domain::PaymentMethodData::CardRedirect(_)
                | domain::PaymentMethodData::Wallet(_)
                | domain::PaymentMethodData::PayLater(_)
                | domain::PaymentMethodData::BankRedirect(_)
                | domain::PaymentMethodData::BankDebit(_)
                | domain::PaymentMethodData::BankTransfer(_)
                | domain::PaymentMethodData::Crypto(_)
                | domain::PaymentMethodData::MandatePayment
                | domain::PaymentMethodData::Reward
                | domain::PaymentMethodData::RealTimePayment(_)
                | domain::PaymentMethodData::Upi(_)
                | domain::PaymentMethodData::Voucher(_)
                | domain::PaymentMethodData::GiftCard(_)
                | domain::PaymentMethodData::CardToken(_)
                | domain::PaymentMethodData::OpenBanking(_)
                | domain::PaymentMethodData::MobilePayment(_) => None,
            }),
        card_discovery: payments_dsl_input.payment_attempt.card_discovery,
    };

    let issuer_data_input = dsl_inputs::IssuerDataInput {
        name: payments_dsl_input
            .payment_method_data
            .as_ref()
            .and_then(|pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => card.card_issuer.clone(),
                domain::PaymentMethodData::CardWithOptionalCVC(card) => card.card_issuer.clone(),
                domain::PaymentMethodData::CardWithNetworkTokenDetails(
                    card_with_network_token_details,
                ) => card_with_network_token_details
                    .card_details
                    .card_issuer
                    .clone(),
                domain::PaymentMethodData::CardDetailsForNetworkTransactionId(
                    card_details_for_ntid,
                ) => card_details_for_ntid.card_issuer.clone(),
                domain::PaymentMethodData::CardWithLimitedDetails(card_with_limited_details) => {
                    card_with_limited_details.card_issuer.clone()
                }
                domain::PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(
                    network_token_details_for_ntid,
                ) => network_token_details_for_ntid.card_issuer.clone(),
                domain::PaymentMethodData::NetworkToken(network_token_details) => {
                    network_token_details.card_issuer.clone()
                }
                domain::PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(
                    _,
                )
                | domain::PaymentMethodData::CardRedirect(_)
                | domain::PaymentMethodData::Wallet(_)
                | domain::PaymentMethodData::PayLater(_)
                | domain::PaymentMethodData::BankRedirect(_)
                | domain::PaymentMethodData::BankDebit(_)
                | domain::PaymentMethodData::BankTransfer(_)
                | domain::PaymentMethodData::Crypto(_)
                | domain::PaymentMethodData::MandatePayment
                | domain::PaymentMethodData::Reward
                | domain::PaymentMethodData::RealTimePayment(_)
                | domain::PaymentMethodData::Upi(_)
                | domain::PaymentMethodData::Voucher(_)
                | domain::PaymentMethodData::GiftCard(_)
                | domain::PaymentMethodData::CardToken(_)
                | domain::PaymentMethodData::OpenBanking(_)
                | domain::PaymentMethodData::MobilePayment(_) => None,
            }),
        country: payments_dsl_input.payment_method_data.as_ref().and_then(
            |pm_data| match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    card.card_issuing_country_code.clone().and_then(|code| {
                        CountryAlpha2::from_str(&code)
                            .ok()
                            .map(common_enums::Country::from_alpha2)
                    })
                }
                domain::PaymentMethodData::CardDetailsForNetworkTransactionId(
                    card_details_for_ntid,
                ) => card_details_for_ntid
                    .card_issuing_country_code
                    .clone()
                    .and_then(|code| {
                        CountryAlpha2::from_str(&code)
                            .ok()
                            .map(common_enums::Country::from_alpha2)
                    }),
                domain::PaymentMethodData::CardWithLimitedDetails(card_with_limited_details) => {
                    card_with_limited_details
                        .card_issuing_country_code
                        .clone()
                        .and_then(|code| {
                            CountryAlpha2::from_str(&code)
                                .ok()
                                .map(common_enums::Country::from_alpha2)
                        })
                }
                _ => None,
            },
        ),
    };

    let issuer_data = match (&issuer_data_input.name, &issuer_data_input.country) {
        (None, None) => None,
        _ => Some(issuer_data_input),
    };

    let payment_input =
        dsl_inputs::PaymentInput {
            amount: payments_dsl_input.payment_attempt.get_total_amount(),
            card_bin: {
                let card_bin = payments_dsl_input.payment_method_data.as_ref().and_then(
                    |pm_data| match pm_data {
                        domain::PaymentMethodData::Card(card) => {
                            let bin = card.card_number.peek().chars().take(6).collect::<String>();

                            (!bin.is_empty()).then_some(bin)
                        }
                        domain::PaymentMethodData::CardDetailsForNetworkTransactionId(
                            card_details_for_ntid,
                        ) => {
                            let bin = card_details_for_ntid
                                .card_number
                                .peek()
                                .chars()
                                .take(6)
                                .collect::<String>();

                            (!bin.is_empty()).then_some(bin)
                        }
                        domain::PaymentMethodData::CardWithLimitedDetails(
                            card_with_limited_details,
                        ) => {
                            let bin = card_with_limited_details
                                .card_number
                                .peek()
                                .chars()
                                .take(6)
                                .collect::<String>();

                            (!bin.is_empty()).then_some(bin)
                        }
                        _ => None,
                    },
                );

                card_bin.or_else(|| {
                    payments_dsl_input
                        .payment_attempt
                        .payment_method_data
                        .as_ref()
                        .and_then(|pm_data| pm_data.get("card"))
                        .and_then(|card| card.get("card_isin"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
            },
            transaction_initiator: match payments_dsl_input.payment_intent.off_session {
                Some(true) => Some(euclid_dir::enums::TransactionInitiator::Merchant),
                _ => Some(euclid_dir::enums::TransactionInitiator::Customer),
            },
            extended_card_bin: {
                let extended_bin = payments_dsl_input.payment_method_data.as_ref().and_then(
                    |pm_data| match pm_data {
                        domain::PaymentMethodData::Card(card) => {
                            let bin = card.card_number.peek().chars().take(8).collect::<String>();

                            (!bin.is_empty()).then_some(bin)
                        }
                        domain::PaymentMethodData::CardDetailsForNetworkTransactionId(
                            card_details_for_ntid,
                        ) => {
                            let bin = card_details_for_ntid
                                .card_number
                                .peek()
                                .chars()
                                .take(8)
                                .collect::<String>();

                            (!bin.is_empty()).then_some(bin)
                        }
                        domain::PaymentMethodData::CardWithLimitedDetails(
                            card_with_limited_details,
                        ) => {
                            let bin = card_with_limited_details
                                .card_number
                                .peek()
                                .chars()
                                .take(8)
                                .collect::<String>();

                            (!bin.is_empty()).then_some(bin)
                        }
                        _ => None,
                    },
                );

                extended_bin.or_else(|| {
                    payments_dsl_input
                        .payment_attempt
                        .payment_method_data
                        .as_ref()
                        .and_then(|pm_data| pm_data.get("card"))
                        .and_then(|card| card.get("card_extended_bin"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
            },
            currency: payments_dsl_input.currency,
            authentication_type: payments_dsl_input.payment_attempt.authentication_type,
            capture_method: payments_dsl_input
                .payment_attempt
                .capture_method
                .and_then(|cm| cm.foreign_into()),
            business_country: payments_dsl_input
                .payment_intent
                .business_country
                .map(api_enums::Country::from_alpha2),
            billing_country: payments_dsl_input
                .address
                .get_payment_method_billing()
                .and_then(|bic| bic.address.as_ref())
                .and_then(|add| add.country)
                .map(api_enums::Country::from_alpha2),
            business_label: payments_dsl_input.payment_intent.business_label.clone(),
            setup_future_usage: payments_dsl_input.payment_intent.setup_future_usage,
            surcharge_amount: payments_dsl_input
                .payment_attempt
                .external_surcharge_details
                .as_ref()
                .map(|details| details.external_surcharge_amount),
        };

    let metadata = payments_dsl_input
        .payment_intent
        .parse_and_get_metadata("routing_parameters")
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    Ok(dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
        acquirer_data: None,
        customer_device_data: None,
        issuer_data,
    })
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub trait RoutingStage: Send + Sync {
    type Input<'a>
    where
        Self: 'a;

    type Output;
    type Fut<'a>: Future<Output = RoutingResult<Self::Output>> + Send
    where
        Self: 'a;

    fn route<'a>(&'a self, input: Self::Input<'a>) -> Self::Fut<'a>;

    fn routing_approach(&self) -> common_enums::RoutingApproach;
}

#[cfg(feature = "v1")]
#[derive(Clone)]
pub struct SessionRoutingContext {
    pub routing_algorithm: Arc<MerchantAccountRoutingAlgorithm>,
}

#[derive(Clone)]
pub struct RoutingContext {
    pub routing_algorithm: Arc<CachedAlgorithm>,
}

pub struct RoutingConnectorOutcome {
    pub connectors: Vec<routing_types::RoutableConnectorChoice>,
    /// Selection involved a volume split (randomized), so a DE-vs-HS diff is expected.
    pub is_volume_split: bool,
}

impl RoutingConnectorOutcome {
    pub fn resolve_or_fallback_with_approach(
        self,
        stage: &'static str,
        fallback: &[routing_types::RoutableConnectorChoice],
        success_approach: common_enums::RoutingApproach,
        fallback_approach: common_enums::RoutingApproach,
    ) -> (
        Vec<routing_types::RoutableConnectorChoice>,
        common_enums::RoutingApproach,
    ) {
        if self.connectors.is_empty() {
            logger::warn!("euclid: {} returned empty connectors, falling back", stage);
            routing::log_connectors(stage, fallback);
            logger::debug!(
                stage = %stage,
                routing_approach = ?fallback_approach,
                "euclid: routing approach after stage"
            );
            (fallback.to_vec(), fallback_approach)
        } else {
            routing::log_connectors(stage, &self.connectors);
            logger::debug!(
                stage = %stage,
                routing_approach = ?success_approach,
                "euclid: routing approach after stage"
            );
            (self.connectors, success_approach)
        }
    }

    pub fn empty() -> Self {
        Self {
            connectors: Vec::new(),
            is_volume_split: false,
        }
    }
}

impl From<Vec<routing_types::RoutableConnectorChoice>> for RoutingConnectorOutcome {
    fn from(connectors: Vec<routing_types::RoutableConnectorChoice>) -> Self {
        Self {
            connectors,
            is_volume_split: false,
        }
    }
}

pub struct StraightThroughRoutingStage {
    pub algorithm: Arc<api_models::routing::StraightThroughAlgorithm>,
}

pub struct StraightThroughRoutingInput<'a> {
    pub creds_identifier: Option<&'a str>,
}

pub struct ConnectorOutcomeWithEligibilityRequirement {
    pub connectors: RoutingConnectorOutcome,
    pub check_eligibility: bool,
}

impl RoutingStage for StraightThroughRoutingStage {
    type Input<'a> = StraightThroughRoutingInput<'a>;
    type Output = ConnectorOutcomeWithEligibilityRequirement;
    type Fut<'a> = BoxFuture<'a, RoutingResult<Self::Output>>;

    fn route<'a>(&'a self, input: Self::Input<'a>) -> Self::Fut<'a> {
        Box::pin(async move {
            let (connectors, check_eligibility) =
                perform_straight_through_routing(&self.algorithm.clone(), input.creds_identifier)
                    .change_context(errors::RoutingError::DslExecutionError)
                    .attach_printable("euclid: unable to perform straight through routing")?;

            Ok(ConnectorOutcomeWithEligibilityRequirement {
                connectors: connectors.into(),
                check_eligibility,
            })
        })
    }

    fn routing_approach(&self) -> common_enums::RoutingApproach {
        common_enums::RoutingApproach::StraightThroughRouting
    }
}

#[derive(Clone)]
pub struct StaticRoutingInput<'a> {
    pub backend_input: &'a backend::BackendInput,
}

#[derive(Clone)]
pub struct StaticRoutingStage {
    pub ctx: RoutingContext,
}

impl RoutingStage for StaticRoutingStage {
    type Input<'a> = StaticRoutingInput<'a>;
    type Output = RoutingConnectorOutcome;
    type Fut<'a> = BoxFuture<'a, RoutingResult<Self::Output>>;

    fn route<'a>(&'a self, input: Self::Input<'a>) -> Self::Fut<'a> {
        Box::pin(async move {
            static_routing_v1(&self.ctx.routing_algorithm, input.backend_input.clone())
                .await
                .change_context(errors::RoutingError::DslExecutionError)
                .attach_printable("euclid: unable to perform static routing locally")
        })
    }

    fn routing_approach(&self) -> common_enums::RoutingApproach {
        common_enums::RoutingApproach::RuleBasedRouting
    }
}

#[cfg(feature = "v1")]
pub async fn perform_static_routing_locally(
    state: &SessionState,
    business_profile: &domain::Profile,
    payment_dsl_input: &routing::PaymentsDslInput<'_>,
    backend_input: &backend::BackendInput,
    fallback_config: &[api_models::routing::RoutableConnectorChoice],
) -> errors::RouterResult<(
    Vec<routing_types::RoutableConnectorChoice>,
    common_enums::RoutingApproach,
    bool,
)> {
    let txn_type = routing::transaction_type_from_payments_dsl(payment_dsl_input);

    let routing_algorithm_id = business_profile
        .routing_algorithm
        .clone()
        .map(|ra| ra.parse_value::<api::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)?
        .unwrap_or_default()
        .algorithm_id;

    let cached_algorithm = routing_algorithm_id
        .async_and_then(|routing_algorithm_id| async move {
            try_ensure_algorithm_cached_v1(
                state,
                &business_profile.merchant_id,
                &routing_algorithm_id,
                business_profile.get_id(),
                &txn_type,
            )
            .await
        })
        .await;

    let static_input = StaticRoutingInput { backend_input };

    let static_stage = cached_algorithm.map(|algo| StaticRoutingStage {
        ctx: RoutingContext {
            routing_algorithm: algo,
        },
    });

    let outcome = static_stage
        .clone()
        .async_and_then(|static_stage| async move {
            static_stage
                .route(static_input)
                .await
                .inspect_err(|err| {
                    logger::error!(
                        error=?err,
                        "euclid: local static routing failed"
                    );
                })
                .ok()
        })
        .await
        .unwrap_or_else(RoutingConnectorOutcome::empty);

    let is_volume_split = outcome.is_volume_split;

    let (static_connectors, static_approach) = outcome.resolve_or_fallback_with_approach(
        "static-routing",
        fallback_config,
        static_stage
            .as_ref()
            .map(|s| s.routing_approach())
            .unwrap_or(common_enums::RoutingApproach::DefaultFallback),
        common_enums::RoutingApproach::DefaultFallback,
    );

    Ok((static_connectors, static_approach, is_volume_split))
}

/// Spawns one batch shadow evaluation for a whole session/pre-routing request.
///
/// These flows evaluate per payment method type; the batch keeps that off the request
/// path and down to a single engine round trip. The result is only load-bearing for a
/// cut-over profile -- for everyone else it exists purely for the diff, so the request
/// never waits on it.
#[cfg(feature = "v1")]
fn spawn_session_shadow_batch_evaluation(
    state: &SessionState,
    business_profile: &domain::Profile,
    payment_id: String,
    entries: Vec<utils::ShadowBatchEntry>,
    fallback_config: Vec<routing_types::RoutableConnectorChoice>,
    transaction_type: api_enums::TransactionType,
    routing_flow: utils::RoutingFlow,
) {
    use router_env::tracing::Instrument;

    let shadow_state = state.clone();
    let shadow_profile = business_profile.clone();
    let shadow_span = router_env::tracing::info_span!(
        "shadow_decision_engine_routing",
        de_shadow = true,
        routing_flow = routing_flow.as_str(),
        entry_count = entries.len(),
        profile_id = %business_profile.get_id().get_string_repr(),
        merchant_id = %business_profile.merchant_id.get_string_repr(),
        payment_id = %payment_id,
    );

    tokio::spawn(
        async move {
            utils::shadow_decision_engine_routing_batch(
                shadow_state,
                shadow_profile,
                payment_id,
                entries,
                fallback_config,
                transaction_type,
                routing_flow,
            )
            .await;
        }
        .instrument(shadow_span),
    );
}

pub struct SessionRoutingInput<'a> {
    pub state: &'a SessionState,
    pub business_profile: &'a domain::Profile,
    pub key_store: &'a domain::MerchantKeyStore,
    pub merchant_account: &'a domain::MerchantAccount,
    pub transaction_type: &'a api_enums::TransactionType,
    pub chosen: &'a api::SessionConnectorDatas,
    pub active_mca_ids:
        &'a std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
    pub default_config: &'a Vec<routing_types::RoutableConnectorChoice>,
    pub backend_input: &'a mut backend::BackendInput,
    /// Resolves whether this profile is cut over to the Decision Engine.
    pub dimensions: &'a dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    pub payment_id: String,
}

#[cfg(feature = "v1")]
#[derive(Clone)]
pub struct SessionRoutingStage {
    pub ctx: SessionRoutingContext,
}

#[cfg(feature = "v1")]
impl RoutingStage for SessionRoutingStage {
    type Input<'a> = SessionRoutingInput<'a>;
    type Output = RoutingConnectorOutcomeForSessionRouting;
    type Fut<'a> = BoxFuture<'a, RoutingResult<Self::Output>>;

    fn route<'a>(&'a self, input: Self::Input<'a>) -> Self::Fut<'a> {
        Box::pin(async move {
            let mut pm_type_map: FxHashMap<
                api_enums::PaymentMethodType,
                FxHashMap<SessionRoutingConnectorKey, api::GetToken>,
            > = FxHashMap::default();

            let profile_id = input.business_profile.get_id();

            for connector_data in input.chosen.iter() {
                pm_type_map
                    .entry(connector_data.payment_method_sub_type)
                    .or_default()
                    .insert(
                        connector_data.connector.merchant_connector_id.clone(),
                        connector_data.connector.get_token.clone(),
                    );
            }

            let mut final_routing_approach = common_enums::RoutingApproach::DefaultFallback;

            let mut result: FxHashMap<
                api_enums::PaymentMethodType,
                Vec<routing_types::SessionRoutingChoice>,
            > = FxHashMap::default();

            // Both are independent of payment method type, so they are resolved once rather
            // than per iteration.
            let de_routing_effective =
                utils::is_decision_engine_routing_effective(input.state, input.dimensions).await;
            let shadow_evaluation_enabled = input.state.conf.open_router.static_routing_enabled
                && input.state.conf.open_router.shadow_routing_enabled
                && profile_has_active_routing_algorithm(input.business_profile);

            // Built up front so the Decision Engine calls can be issued together rather
            // than one wallet type at a time. A rule may branch on payment method type, so
            // the calls cannot be collapsed into one -- but they need not be serialised.
            let pm_entries = pm_type_map
                .into_iter()
                .map(|(pm_type, allowed_connectors)| {
                    let euclid_pmt: euclid_enums::PaymentMethodType = pm_type;
                    let euclid_pm: euclid_enums::PaymentMethod = euclid_pmt.into();
                    let mut backend_input = input.backend_input.clone();
                    backend_input.payment_method.payment_method = Some(euclid_pm);
                    backend_input.payment_method.payment_method_type = Some(euclid_pmt);
                    (pm_type, allowed_connectors, backend_input)
                })
                .collect::<Vec<_>>();

            // One batch call for a cut-over profile: the engine fetches the rule once and
            // evaluates every wallet type's parameters in a single round trip. Against an
            // engine without the batch endpoint this degrades to concurrent single calls.
            // Not cut over, the result is shadow-only and is spawned after the loop.
            let de_results: Vec<Vec<routing_types::RoutableConnectorChoice>> =
                if de_routing_effective {
                    utils::decision_engine_routing_batch_with_fallback(
                        input.state,
                        pm_entries
                            .iter()
                            .map(|(_, _, backend_input)| backend_input.clone())
                            .collect(),
                        input.business_profile,
                        input.payment_id.clone(),
                        input.default_config.clone(),
                        *input.transaction_type,
                        utils::RoutingFlow::SessionToken,
                    )
                    .await
                } else {
                    vec![Vec::new(); pm_entries.len()]
                };

            let mut shadow_entries: Vec<utils::ShadowBatchEntry> = Vec::new();

            for ((pm_type, allowed_connectors, backend_input), de_connectors) in
                pm_entries.into_iter().zip(de_results)
            {
                let algorithm_id = match &*self.ctx.routing_algorithm {
                    MerchantAccountRoutingAlgorithm::V1(algorithm_ref) => {
                        &algorithm_ref.algorithm_id
                    }
                };

                // Evaluated even under cutover, so an empty DE result falls back to the
                // merchant's own rule rather than the flat fallback list.
                let cached_algorithm = algorithm_id
                    .clone()
                    .async_and_then(|algorithm_id| async move {
                        try_ensure_algorithm_cached_v1(
                            input.state,
                            &input.business_profile.merchant_id,
                            &algorithm_id,
                            input.business_profile.get_id(),
                            input.transaction_type,
                        )
                        .await
                    })
                    .await;

                let static_input = StaticRoutingInput {
                    backend_input: &backend_input,
                };

                let static_stage = cached_algorithm.map(|cached_algorithm| StaticRoutingStage {
                    ctx: RoutingContext {
                        routing_algorithm: cached_algorithm,
                    },
                });

                let (chosen_connectors, static_approach) = static_stage
                    .clone()
                    .async_and_then(|static_stage| async move {
                        static_stage
                            .route(static_input)
                            .await
                            .inspect_err(|err| {
                                logger::error!(
                                    error=?err,
                                    "euclid: session routing failed"
                                );
                            })
                            .ok()
                    })
                    .await
                    .unwrap_or_else(RoutingConnectorOutcome::empty)
                    .resolve_or_fallback_with_approach(
                        "static-routing",
                        input.default_config,
                        static_stage
                            .as_ref()
                            .map(|s| s.routing_approach())
                            .unwrap_or(common_enums::RoutingApproach::DefaultFallback),
                        common_enums::RoutingApproach::DefaultFallback,
                    );

                let is_volume_split = matches!(
                    static_approach,
                    common_enums::RoutingApproach::VolumeBasedRouting
                );

                // The DE result is only load-bearing for a cut-over profile. Everyone else
                // gets it shadow-evaluated off the request path, so the diff stays visible
                // without adding a round trip to session token generation.
                let chosen_connectors = if de_routing_effective {
                    // Diff logging only. The kill switch is deliberately not fed from here:
                    // it gates routing for the whole profile, and tripping it on a
                    // session-flow discrepancy would disable DE routing for payments too.
                    utils::compare_and_log_result(
                        de_connectors.clone(),
                        chosen_connectors.clone(),
                        utils::RoutingFlow::SessionToken.as_str().to_string(),
                        is_volume_split,
                    );

                    // Only the connector list is swapped; `RoutingApproach` is left as the
                    // Hyperswitch side derived it, matching `perform_static_routing_v1` --
                    // it is persisted on the attempt and read by analytics, so relabelling
                    // it here would shift those numbers for cut-over merchants.
                    utils::select_routing_result(
                        input.state,
                        input.dimensions,
                        input.business_profile,
                        chosen_connectors,
                        de_connectors,
                    )
                    .await
                } else {
                    if shadow_evaluation_enabled {
                        shadow_entries.push(utils::ShadowBatchEntry {
                            backend_input: backend_input.clone(),
                            hs_connectors: chosen_connectors.clone(),
                            is_volume: is_volume_split,
                        });
                    }
                    chosen_connectors
                };

                let primary = perform_cgraph_filtering(
                    input.state,
                    input.key_store,
                    chosen_connectors,
                    backend_input.clone(),
                    None,
                    profile_id,
                    input.transaction_type,
                    input.active_mca_ids,
                )
                .await?;

                let final_selection = if primary.is_empty() {
                    perform_cgraph_filtering(
                        input.state,
                        input.key_store,
                        input.default_config.clone(),
                        backend_input.clone(),
                        None,
                        profile_id,
                        input.transaction_type,
                        input.active_mca_ids,
                    )
                    .await?
                } else {
                    primary
                };

                let routable_connector_choice_option = if final_selection.is_empty() {
                    (None, static_approach.clone())
                } else {
                    (Some(final_selection), static_approach)
                };

                final_routing_approach = routable_connector_choice_option.1;

                if let Some(routable_connector_choice) = routable_connector_choice_option.0 {
                    let mut session_routing_choice: Vec<routing_types::SessionRoutingChoice> =
                        Vec::new();

                    for selection in routable_connector_choice {
                        let connector_name = selection.connector.to_string();
                        if let Some(get_token) =
                            allowed_connectors.get(&selection.merchant_connector_id)
                        {
                            let connector_data = api::ConnectorData::get_connector_by_name(
                                &input.state.clone().conf.connectors,
                                &connector_name,
                                get_token.clone(),
                                selection.merchant_connector_id,
                            )
                            .change_context(
                                errors::RoutingError::InvalidConnectorName(connector_name),
                            )?;

                            session_routing_choice.push(routing_types::SessionRoutingChoice {
                                connector: connector_data,
                                payment_method_type: pm_type,
                            });
                        }
                    }
                    if !session_routing_choice.is_empty() {
                        result.insert(pm_type, session_routing_choice);
                    }
                }
            }

            // One spawned batch evaluation for the whole request, replacing a spawned
            // call per wallet type. Off the request path; diff logging only.
            if !shadow_entries.is_empty() {
                spawn_session_shadow_batch_evaluation(
                    input.state,
                    input.business_profile,
                    input.payment_id.clone(),
                    shadow_entries,
                    input.default_config.clone(),
                    *input.transaction_type,
                    utils::RoutingFlow::SessionToken,
                );
            }

            Ok(RoutingConnectorOutcomeForSessionRouting {
                session_output: result,
                routing_approach: final_routing_approach,
            })
        })
    }

    fn routing_approach(&self) -> common_enums::RoutingApproach {
        common_enums::RoutingApproach::Other("SessionFlowRouting".to_string())
    }
}

pub struct RoutingConnectorOutcomeWithApproachAndEligibility {
    pub connectors: Vec<routing_types::RoutableConnectorChoice>,
    pub routing_approach: common_enums::RoutingApproach,
    pub requires_eligibility: bool,
}

pub struct PreRoutingInput<'a> {
    pub pre_routing_results:
        &'a Option<HashMap<api_enums::PaymentMethodType, PreRoutingConnectorChoice>>,
    pub payment_method_type: &'a storage_enums::PaymentMethodType,
    pub connectors: &'a hyperswitch_interfaces::configs::Connectors,
    pub processor: &'a domain::Processor,
    pub business_profile: &'a domain::Profile,
    pub creds_identifier: Option<&'a str>,
}

pub async fn resolve_pre_routed_connectors(
    input: PreRoutingInput<'_>,
) -> RoutingResult<Vec<api::ConnectorRoutingData>> {
    let routable_connector_choice = input
        .pre_routing_results
        .as_ref()
        .ok_or(errors::RoutingError::DslExecutionError)?
        .get(input.payment_method_type)
        .ok_or(errors::RoutingError::DslExecutionError)?;

    let routable_connectors = match routable_connector_choice {
        PreRoutingConnectorChoice::Single(c) => vec![c.clone()],
        PreRoutingConnectorChoice::Multiple(cs) => cs.clone(),
    };

    let mut connector_routing_data = Vec::with_capacity(routable_connectors.len());

    for connector_choice in routable_connectors {
        let connector_data = api::ConnectorData::get_connector_by_name(
            input.connectors,
            &connector_choice.connector.to_string(),
            api::GetToken::Connector,
            connector_choice.merchant_connector_id.clone(),
        )
        .change_context(errors::RoutingError::DslExecutionError)
        .attach_printable("Invalid connector name received")?
        .into();

        connector_routing_data.push(connector_data);
    }
    logger::debug!("euclid_routing: pre-routing connectors resolved");
    Ok(connector_routing_data)
}

pub fn try_get_attempt_connector<F, D>(
    connectors: &hyperswitch_interfaces::configs::Connectors,
    payment_data: &D,
    routing_data: &mut RoutingData,
) -> errors::RouterResult<Option<api::ConnectorCallType>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F>,
{
    Ok(payment_data
        .get_payment_attempt()
        .connector
        .as_ref()
        .and_then(|connector_name| {
            api::ConnectorData::get_connector_by_name(
                connectors,
                connector_name,
                api::GetToken::Connector,
                payment_data
                    .get_payment_attempt()
                    .merchant_connector_id
                    .clone(),
            )
            .inspect_err(|err| {
                logger::warn!(
                    error=?err,
                    "euclid: invalid predetermined connector, ignoring"
                );
            })
            .ok()
            .map(|connector_data| {
                logger::debug!("euclid_routing: predetermined connector present in attempt");
                routing_data.routed_through = Some(connector_name.clone());
                api::ConnectorCallType::PreDetermined(connector_data.into())
            })
        }))
}

pub fn try_get_mandate_connector<F, D>(
    connectors: &hyperswitch_interfaces::configs::Connectors,
    payment_data: &D,
    routing_data: &mut RoutingData,
) -> errors::RouterResult<Option<api::ConnectorCallType>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F>,
{
    Ok(payment_data
        .get_mandate_connector()
        .and_then(|mandate_connector_details| {
            api::ConnectorData::get_connector_by_name(
                connectors,
                &mandate_connector_details.connector,
                api::GetToken::Connector,
                mandate_connector_details.merchant_connector_id.clone(),
            )
            .inspect_err(|err| {
                logger::warn!(
                    error=?err,
                    "euclid: invalid mandate connector, ignoring"
                );
            })
            .ok()
            .map(|connector_data| {
                logger::debug!("euclid_routing: predetermined mandate connector");
                routing_data.routed_through = Some(mandate_connector_details.connector.clone());
                routing_data
                    .merchant_connector_id
                    .clone_from(&mandate_connector_details.merchant_connector_id);
                api::ConnectorCallType::PreDetermined(connector_data.into())
            })
        }))
}

pub fn try_get_pre_determined_connector<F, D>(
    connectors: &hyperswitch_interfaces::configs::Connectors,
    payment_data: &D,
    routing_data: &mut RoutingData,
) -> errors::RouterResult<Option<api::ConnectorCallType>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F>,
{
    match try_get_attempt_connector::<F, D>(connectors, payment_data, routing_data)? {
        Some(connector) => Ok(Some(connector)),
        None => try_get_mandate_connector::<F, D>(connectors, payment_data, routing_data),
    }
}

#[cfg(feature = "v1")]
pub async fn try_pre_routing_connectors<F, D>(
    state: &SessionState,
    processor: &domain::Processor,
    business_profile: &domain::Profile,
    payment_data: &mut D,
    routing_data: &mut RoutingData,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
) -> errors::RouterResult<Option<api::ConnectorCallType>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let mut connector_call_type = None;

    if let (None, Some(payment_method_type)) = (
        payment_data.get_token_data(),
        payment_data
            .get_payment_attempt()
            .payment_method_type
            .as_ref(),
    ) {
        logger::debug!("euclid: checking for pre-routing result");
        let pre_routing_input = PreRoutingInput {
            pre_routing_results: &routing_data.routing_info.pre_routing_results,
            payment_method_type,
            connectors: &state.conf.connectors,
            processor,
            business_profile,
            creds_identifier: payment_data.get_creds_identifier(),
        };

        if let Ok(connectors) = resolve_pre_routed_connectors(pre_routing_input).await {
            let first_connector = connectors
                .first()
                .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)?;

            routing_data.routed_through =
                Some(first_connector.connector_data.connector_name.to_string());
            routing_data.merchant_connector_id =
                first_connector.connector_data.merchant_connector_id.clone();

            #[cfg(feature = "retry")]
            {
                let should_do_retry = crate::core::payments::retry::config_should_call_gsm(
                    state,
                    dimensions,
                    business_profile,
                    payment_data.get_payment_intent().customer_id.as_ref(),
                )
                .await;

                if payment_data.get_payment_attempt().payment_method_type
                    == Some(storage_enums::PaymentMethodType::ApplePay)
                    && should_do_retry
                {
                    let retryable_connector_data =
                        crate::core::payments::helpers::get_apple_pay_retryable_connectors(
                            state,
                            processor,
                            payment_data.get_creds_identifier(),
                            &connectors.clone(),
                            first_connector
                                .connector_data
                                .merchant_connector_id
                                .clone()
                                .as_ref(),
                            business_profile.clone(),
                        )
                        .await?;

                    if let Some(connector_data_list) = retryable_connector_data {
                        if connector_data_list.len() > 1 {
                            logger::info!("Constructed apple pay retryable connector list");
                            connector_call_type =
                                Some(api::ConnectorCallType::Retryable(connector_data_list));
                        }
                    }
                }
            }

            if connector_call_type.is_none() {
                crate::core::payments::helpers::override_setup_future_usage_to_on_session(
                    &*state.store,
                    payment_data,
                )
                .await?;

                connector_call_type = Some(api::ConnectorCallType::PreDetermined(
                    first_connector.clone(),
                ));
            }
        }
    }

    Ok(connector_call_type)
}

pub struct RoutingConnectorOutcomeForSessionRouting {
    pub session_output:
        FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>,
    pub routing_approach: common_enums::RoutingApproach,
}
pub struct RoutingConnectorOutcomeWithApproach {
    pub connectors: Vec<routing_types::RoutableConnectorChoice>,
    pub routing_approach: common_enums::RoutingApproach,
}

impl RoutingConnectorOutcomeWithApproach {
    pub fn resolve_or_fallback(
        self,
        stage: &'static str,
        fallback_connectors: &[routing_types::RoutableConnectorChoice],
        fallback_approach: common_enums::RoutingApproach,
    ) -> (
        Vec<routing_types::RoutableConnectorChoice>,
        common_enums::RoutingApproach,
    ) {
        if self.connectors.is_empty() {
            logger::warn!("euclid: {} returned empty connectors, falling back", stage);
            routing::log_connectors(stage, fallback_connectors);
            logger::debug!(
                stage = %stage,
                routing_approach = ?fallback_approach,
                "euclid: routing approach after stage"
            );
            (fallback_connectors.to_vec(), fallback_approach)
        } else {
            let routing_approach = self.routing_approach;
            routing::log_connectors(stage, &self.connectors);
            logger::debug!(
                stage = %stage,
                routing_approach = ?routing_approach,
                "euclid: routing approach after stage"
            );
            (self.connectors, routing_approach)
        }
    }

    pub fn empty() -> Self {
        Self {
            connectors: Vec::new(),
            routing_approach: common_enums::RoutingApproach::DefaultFallback,
        }
    }
}

#[cfg(feature = "v1")]
pub struct HybridRoutingInput<'a> {
    pub state: &'a SessionState,
    pub business_profile: &'a domain::Profile,
    pub payment_dsl_input: &'a routing::PaymentsDslInput<'a>,
    pub backend_input: &'a backend::BackendInput,
    pub fallback_config: &'a [routing_types::RoutableConnectorChoice],
    pub static_connectors: &'a [routing_types::RoutableConnectorChoice],
    pub static_approach: common_enums::RoutingApproach,
    /// Whether the HS static selection involved a volume split.
    pub static_is_volume_split: bool,
}

#[cfg(feature = "v1")]
pub struct HybridRoutingStage;

#[cfg(feature = "v1")]
impl HybridRoutingStage {
    #[cfg(all(feature = "v1", feature = "dynamic_routing"))]
    fn build_dynamic_routing_request(
        &self,
        input: &HybridRoutingInput<'_>,
    ) -> (Option<OpenRouterDecideGatewayRequest>, Option<u8>) {
        if !input.state.conf.open_router.dynamic_routing_enabled {
            (None, None)
        } else if let Some(dynamic_routing_algo) =
            input.business_profile.dynamic_routing_algorithm.clone()
        {
            match dynamic_routing_algo.parse_value::<api_routing::DynamicRoutingAlgorithmRef>(
                "DynamicRoutingAlgorithmRef",
            ) {
                Ok(dynamic_routing_config) => {
                    let dynamic_routing_volume_split = dynamic_routing_config
                        .dynamic_routing_volume_split
                        .unwrap_or_default();
                    let is_dynamic_feature_enabled = dynamic_routing_config
                        .is_success_rate_routing_enabled()
                        || dynamic_routing_config.is_elimination_enabled();

                    if !is_dynamic_feature_enabled {
                        logger::debug!(
                            "euclid: dynamic routing config present but dynamic features are disabled"
                        );
                        (None, Some(dynamic_routing_volume_split))
                    } else {
                        match perform_dynamic_routing_volume_split(
                            vec![
                                api_models::routing::RoutingVolumeSplit {
                                    routing_type: api_models::routing::RoutingType::Dynamic,
                                    split: dynamic_routing_volume_split,
                                },
                                api_models::routing::RoutingVolumeSplit {
                                    routing_type: api_models::routing::RoutingType::Static,
                                    split: crate::consts::DYNAMIC_ROUTING_MAX_VOLUME
                                        - dynamic_routing_volume_split,
                                },
                            ],
                            None,
                        ) {
                            Ok(routing_choice)
                                if routing_choice.routing_type.is_dynamic_routing() =>
                            {
                                (
                                    Some(OpenRouterDecideGatewayRequest::construct_sr_request(
                                        input.payment_dsl_input.payment_attempt,
                                        input.static_connectors.to_vec(),
                                        Some(or_types::RankingAlgorithm::SrBasedRouting),
                                        dynamic_routing_config.is_elimination_enabled(),
                                    )),
                                    Some(dynamic_routing_volume_split),
                                )
                            }
                            Ok(_) => (None, Some(dynamic_routing_volume_split)),
                            Err(error) => {
                                logger::error!(
                                    error=?error,
                                    "euclid: failed to perform dynamic routing volume split for hybrid routing"
                                );
                                (None, Some(dynamic_routing_volume_split))
                            }
                        }
                    }
                }
                Err(error) => {
                    logger::error!(
                        error=?error,
                        "euclid: failed to parse dynamic routing config for hybrid routing"
                    );
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    }

    #[cfg(not(all(feature = "v1", feature = "dynamic_routing")))]
    fn build_dynamic_routing_request(
        &self,
        _input: &HybridRoutingInput<'_>,
    ) -> (Option<OpenRouterDecideGatewayRequest>, Option<u8>) {
        (None, None)
    }
}

#[cfg(feature = "v1")]
impl RoutingStage for HybridRoutingStage {
    type Input<'a> = HybridRoutingInput<'a>;
    type Output = RoutingConnectorOutcomeWithApproach;
    type Fut<'a> = BoxFuture<'a, RoutingResult<Self::Output>>;

    fn route<'a>(&'a self, input: Self::Input<'a>) -> Self::Fut<'a> {
        Box::pin(async move {
            let (dynamic_routing_request, _dynamic_routing_volume_split) =
                self.build_dynamic_routing_request(&input);

            // Under DE cutover, always evaluate the profile's rule on DE; the caller falls back to HS static/default on empty or error.
            let should_include_static_request = input.state.conf.open_router.static_routing_enabled;

            let payment_id = input
                .payment_dsl_input
                .payment_attempt
                .payment_id
                .get_string_repr()
                .to_string();

            let static_routing_request = if should_include_static_request {
                Some(utils::build_static_routing_request_for_hybrid(
                    input
                        .business_profile
                        .get_id()
                        .get_string_repr()
                        .to_string(),
                    payment_id.clone(),
                    input.backend_input.clone(),
                    input.fallback_config.to_vec(),
                )?)
            } else {
                None
            };

            let outcome = if static_routing_request.is_none() && dynamic_routing_request.is_none() {
                logger::debug!(
                    "euclid: hybrid routing skipped since both static and dynamic DE flags are disabled"
                );
                RoutingConnectorOutcomeWithApproach::empty()
            } else {
                // An unreachable/erroring DE is treated as an empty (unresponsive) outcome so the
                // diff is still logged and counted below; the caller falls back to the HS static
                // result. This keeps unresponsive-DE handling identical to the static path.
                let hybrid_outcome = utils::decision_engine_hybrid_routing(
                    input.state,
                    input.business_profile,
                    payment_id,
                    utils::HybridRoutingRequest {
                        static_routing_request,
                        dynamic_routing_request,
                    },
                    input.static_connectors.to_vec(),
                )
                .await
                .unwrap_or_else(|error| {
                    logger::error!(error=?error, "euclid: hybrid DE evaluation failed, treating as unresponsive");
                    utils::HybridRoutingOutcome::empty()
                });

                // Diff logging only — no kill-switch counting: this stage runs solely for
                // cut-over profiles, whose DE-only writes make the HS baseline stale by design.
                utils::compare_and_log_result(
                    hybrid_outcome.connectors.clone(),
                    input.static_connectors.to_vec(),
                    "evaluate_routing".to_string(),
                    input.static_is_volume_split,
                );

                RoutingConnectorOutcomeWithApproach {
                    connectors: hybrid_outcome.connectors,
                    routing_approach: hybrid_outcome.routing_approach.into(),
                }
            };

            Ok(outcome)
        })
    }

    fn routing_approach(&self) -> common_enums::RoutingApproach {
        common_enums::RoutingApproach::Other("HybridRouting".to_string())
    }
}

/// Whether the profile has an active routing algorithm for the Decision Engine to evaluate.
#[cfg(feature = "v1")]
fn profile_has_active_routing_algorithm(business_profile: &domain::Profile) -> bool {
    business_profile
        .routing_algorithm
        .clone()
        .and_then(|ra| {
            ra.parse_value::<api::routing::RoutingAlgorithmRef>("RoutingAlgorithmRef")
                .ok()
        })
        .and_then(|algorithm_ref| algorithm_ref.algorithm_id)
        .is_some()
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v1")]
pub async fn perform_hybrid_routing_if_enabled(
    state: &SessionState,
    business_profile: &domain::Profile,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    payment_dsl_input: &routing::PaymentsDslInput<'_>,
    backend_input: &backend::BackendInput,
    fallback_config: &[routing_types::RoutableConnectorChoice],
    static_connectors: &[routing_types::RoutableConnectorChoice],
    static_approach: common_enums::RoutingApproach,
    static_is_volume_split: bool,
) -> (
    Vec<routing_types::RoutableConnectorChoice>,
    common_enums::RoutingApproach,
) {
    let stage = HybridRoutingStage;
    let input = HybridRoutingInput {
        state,
        business_profile,
        payment_dsl_input,
        backend_input,
        fallback_config,
        static_connectors,
        static_approach: static_approach.clone(),
        static_is_volume_split,
    };

    // Flag-aware like every other consumer: with static_routing_enabled off the profile is
    // Hyperswitch-routed, so this stage must not run (the kill-switch counting below is
    // skipped on the premise that it only ever runs for cut-over profiles).
    let is_decision_engine_cutover_enabled =
        utils::is_decision_engine_routing_effective(state, dimensions).await;
    let has_active_routing_algorithm = profile_has_active_routing_algorithm(business_profile);

    // A cut-over profile's rules live on the Decision Engine, so a missing Hyperswitch
    // algorithm is the normal state and must not skip evaluation; for every other profile
    // there is nothing to evaluate against, so the DE call is skipped.
    if is_decision_engine_cutover_enabled {
        let hybrid_stage_outcome = stage
            .route(input)
            .await
            .inspect_err(|error| {
                logger::error!(
                    error=?error,
                    "euclid: hybrid routing failed"
                );
            })
            .unwrap_or_else(|_| RoutingConnectorOutcomeWithApproach::empty());

        let selected_source = if hybrid_stage_outcome.connectors.is_empty() {
            "hyperswitch_static"
        } else {
            "decision_engine"
        };

        logger::info!(
            business_profile_id=?business_profile.get_id(),
            routing_source = %selected_source,
            "decision_engine_euclid: selected routing source after hybrid stage"
        );

        hybrid_stage_outcome.resolve_or_fallback(
            "hybrid-routing",
            static_connectors,
            static_approach,
        )
    } else if !has_active_routing_algorithm {
        logger::debug!(
            business_profile_id=?business_profile.get_id(),
            "decision_engine_euclid: no active routing algorithm, skipping DE evaluation"
        );
        logger::info!(
            business_profile_id=?business_profile.get_id(),
            routing_source = "hyperswitch_static",
            "decision_engine_euclid: selected routing source after hybrid stage"
        );

        (static_connectors.to_vec(), static_approach)
    } else {
        logger::debug!(
            business_profile_id=?business_profile.get_id(),
            "decision_engine_euclid: cutover not enabled, using static routing result"
        );
        logger::info!(
            business_profile_id=?business_profile.get_id(),
            routing_source = "hyperswitch_static",
            "decision_engine_euclid: selected routing source after hybrid stage"
        );

        // Shadow mode: diff-check DE against the HS result for non-cutover profiles without
        // touching the payment path — detached task, HS result serves the payment either way.
        if state.conf.open_router.static_routing_enabled
            && state.conf.open_router.shadow_routing_enabled
        {
            use router_env::tracing::Instrument;

            let payment_id = payment_dsl_input
                .payment_attempt
                .payment_id
                .get_string_repr()
                .to_string();
            let shadow_state = state.clone();
            let shadow_profile = business_profile.clone();
            let shadow_backend_input = backend_input.clone();
            let shadow_fallback = fallback_config.to_vec();
            let shadow_static_connectors = static_connectors.to_vec();
            let shadow_span = router_env::tracing::info_span!(
                "shadow_decision_engine_routing",
                de_shadow = true,
                profile_id = %business_profile.get_id().get_string_repr(),
                merchant_id = %business_profile.merchant_id.get_string_repr(),
                payment_id = %payment_id,
            );
            tokio::spawn(
                async move {
                    utils::shadow_decision_engine_routing(
                        shadow_state,
                        shadow_profile,
                        payment_id,
                        shadow_backend_input,
                        shadow_fallback,
                        shadow_static_connectors,
                        static_is_volume_split,
                        api_enums::TransactionType::Payment,
                        utils::RoutingFlow::Payment,
                    )
                    .await;
                }
                .instrument(shadow_span),
            );
        }

        (static_connectors.to_vec(), static_approach)
    }
}

pub async fn static_routing_v1(
    routing_algorithm: &CachedAlgorithm,
    backend_input: backend::BackendInput,
) -> RoutingResult<RoutingConnectorOutcome> {
    logger::debug!("euclid_routing: performing routing for connector selection");
    let outcome = match routing_algorithm {
        CachedAlgorithm::Single(conn) => RoutingConnectorOutcome {
            connectors: vec![(**conn).clone()],
            is_volume_split: false,
        },
        CachedAlgorithm::Priority(plist) => RoutingConnectorOutcome {
            connectors: plist.clone(),
            is_volume_split: false,
        },
        CachedAlgorithm::VolumeSplit(splits) => RoutingConnectorOutcome {
            connectors: perform_volume_split(splits.to_vec())
                .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
            is_volume_split: true,
        },
        CachedAlgorithm::Advanced(interpreter) => {
            let dsl_output = execute_dsl_v1(backend_input, interpreter)?;
            let is_volume_split = matches!(
                dsl_output,
                routing_types::StaticRoutingAlgorithm::VolumeSplit(_)
            );
            RoutingConnectorOutcome {
                connectors: dsl_output_to_connectors(dsl_output)?,
                is_volume_split,
            }
        }
    };
    Ok(outcome)
}

pub async fn perform_static_routing_v1(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    algorithm_id: Option<&common_utils::id_type::RoutingId>,
    business_profile: &domain::Profile,
    transaction_data: &routing::TransactionData<'_>,
) -> RoutingResult<(
    Vec<routing_types::RoutableConnectorChoice>,
    Option<common_enums::RoutingApproach>,
)> {
    logger::debug!("euclid_routing: performing routing for connector selection");
    let get_merchant_fallback_config = || async {
        #[cfg(feature = "v1")]
        return routing::helpers::get_merchant_default_config(
            &*state.clone().store,
            business_profile.get_id().get_string_repr(),
            &api_enums::TransactionType::from(transaction_data),
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed);
        #[cfg(feature = "v2")]
        return admin::ProfileWrapper::new(business_profile.clone())
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed);
    };

    let fallback_config = get_merchant_fallback_config().await?;

    // A cut-over profile may have rules only on the DE, so a missing HS algorithm must not skip DE evaluation.
    let de_routing_effective = utils::is_decision_engine_routing_effective(state, dimensions).await;

    let algorithm_id = match algorithm_id {
        // Evaluated even under cutover. The DE result is still preferred, but when it comes
        // back empty -- the engine is unreachable, or the profile's rules have not been
        // migrated yet -- the merchant's own rule is a far better answer than the flat
        // fallback list. The ref is never cleared, so cutover stays a reversible config flip.
        Some(id) => Some(id),
        None if de_routing_effective => {
            logger::debug!(
                "decision_engine_euclid: no active HS algorithm, profile is cut over; evaluating on DE"
            );
            None
        }
        None => {
            logger::debug!("euclid_routing: active algorithm isn't present, default falling back");
            return Ok((fallback_config, None));
        }
    };

    let cached_algorithm = match algorithm_id {
        Some(algorithm_id) => match ensure_algorithm_cached_v1(
            state,
            merchant_id,
            algorithm_id,
            business_profile.get_id(),
            &api_enums::TransactionType::from(transaction_data),
        )
        .await
        {
            Ok(algo) => Some(algo),
            Err(err) => {
                logger::error!(
                    error=?err,
                    "euclid_routing: ensure_algorithm_cached failed, falling back to merchant default connectors"
                );

                if de_routing_effective {
                    None
                } else {
                    return Ok((fallback_config, None));
                }
            }
        },
        None => None,
    };

    let payment_id = match transaction_data {
        routing::TransactionData::Payment(payment_data) => payment_data
            .payment_attempt
            .payment_id
            .clone()
            .get_string_repr()
            .to_string(),
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => payout_data
            .payout_attempt
            .payout_id
            .get_string_repr()
            .to_string(),
    };

    // A routing failure must never fail the payment: any error building the routing input, or
    // evaluating the active algorithm, falls back to the merchant default connectors.
    let backend_input = match transaction_data {
        routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data),
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => make_dsl_input_for_payouts(payout_data),
    };

    let (
        routable_connectors,
        routing_approach,
        is_volume_split,
        de_evaluated_connector,
        hs_eval_succeeded,
    ) = match backend_input {
        Err(err) => {
            logger::error!(error=?err, "euclid_routing: failed to build routing input, falling back to merchant default connectors");
            (fallback_config.clone(), None, false, Vec::default(), false)
        }
        Ok(backend_input) => {
            // Decision engine evaluation is diagnostic only; errors degrade to an empty result.
            let de_evaluated_connector = if !state.conf.open_router.static_routing_enabled {
                logger::debug!("decision_engine_euclid: decision_engine routing not enabled");
                Vec::default()
            } else {
                utils::decision_engine_routing(
                        state,
                        backend_input.clone(),
                        business_profile,
                        payment_id,
                        fallback_config.clone(),
                        api_enums::TransactionType::from(transaction_data),
                        utils::RoutingFlow::Payment,
                    )
                    .await
                    .map_err(|e| logger::error!(decision_engine_euclid_evaluate_error=?e, "decision_engine_euclid: error in evaluation of rule"))
                    .unwrap_or_default()
            };

            let evaluated = (|| -> RoutingResult<(
                    Vec<routing_types::RoutableConnectorChoice>,
                    Option<common_enums::RoutingApproach>,
                    bool,
                )> {
                    Ok(match cached_algorithm.as_deref() {
                        // No HS algorithm (cut-over profile): HS side is the fallback list.
                        None => (fallback_config.clone(), None, false),
                        Some(CachedAlgorithm::Single(conn)) => (
                            vec![(**conn).clone()],
                            Some(common_enums::RoutingApproach::StraightThroughRouting),
                            false,
                        ),
                        Some(CachedAlgorithm::Priority(plist)) => (plist.clone(), None, false),
                        Some(CachedAlgorithm::VolumeSplit(splits)) => (
                            perform_volume_split(splits.to_vec())
                                .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
                            Some(common_enums::RoutingApproach::VolumeBasedRouting),
                            true,
                        ),
                        Some(CachedAlgorithm::Advanced(interpreter)) => {
                            let dsl_output = execute_dsl_v1(backend_input, interpreter)?;
                            let is_volume_split = matches!(
                                dsl_output,
                                routing_types::StaticRoutingAlgorithm::VolumeSplit(_)
                            );
                            (
                                dsl_output_to_connectors(dsl_output)?,
                                Some(common_enums::RoutingApproach::RuleBasedRouting),
                                is_volume_split,
                            )
                        }
                    })
                })();

            let hs_eval_succeeded = evaluated.is_ok();
            let (routable_connectors, routing_approach, is_volume_split) = evaluated
                    .unwrap_or_else(|err| {
                        logger::error!(error=?err, "euclid_routing: algorithm evaluation failed, falling back to merchant default connectors");
                        (fallback_config.clone(), None, false)
                    });

            (
                routable_connectors,
                routing_approach,
                is_volume_split,
                de_evaluated_connector,
                hs_eval_succeeded,
            )
        }
    };

    // Always diff-log (dashboards consume this for cut-over profiles too), but feed the
    // kill switch only from a successfully evaluated HS algorithm on a non-cut-over
    // profile — under DE-only writes the HS baseline is stale by design.
    let comparison = utils::compare_and_log_result(
        de_evaluated_connector.clone(),
        routable_connectors.clone(),
        utils::RoutingFlow::Payment.as_str().to_string(),
        is_volume_split,
    );

    if cached_algorithm.is_some() && hs_eval_succeeded && !de_routing_effective {
        utils::record_de_diff_and_maybe_trip_kill_switch(
            state,
            business_profile.get_id(),
            comparison,
        )
        .await;
    }

    Ok((
        utils::select_routing_result(
            state,
            dimensions,
            business_profile,
            routable_connectors,
            de_evaluated_connector,
        )
        .await,
        routing_approach,
    ))
}

pub async fn ensure_algorithm_cached_v1(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    algorithm_id: &common_utils::id_type::RoutingId,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<CachedAlgorithm>> {
    let key = {
        match transaction_type {
            common_enums::TransactionType::Payment => {
                format!(
                    "routing_config_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr(),
                )
            }
            #[cfg(feature = "payouts")]
            common_enums::TransactionType::Payout => {
                format!(
                    "routing_config_po_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr()
                )
            }
            common_enums::TransactionType::ThreeDsAuthentication => {
                Err(errors::RoutingError::InvalidTransactionType)?
            }
        }
    };

    let cached_algorithm = ROUTING_CACHE
        .get_val::<Arc<CachedAlgorithm>>(CacheKey {
            key: key.clone(),
            prefix: state.tenant.redis_key_prefix.clone(),
        })
        .await;

    let algorithm = if let Some(algo) = cached_algorithm {
        algo
    } else {
        refresh_routing_cache_v1(state, key.clone(), algorithm_id, profile_id).await?
    };

    Ok(algorithm)
}

pub async fn try_ensure_algorithm_cached_v1(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
    algorithm_id: &common_utils::id_type::RoutingId,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> Option<Arc<CachedAlgorithm>> {
    ensure_algorithm_cached_v1(
        state,
        merchant_id,
        algorithm_id,
        profile_id,
        transaction_type,
    )
    .await
    .inspect_err(|err| {
        logger::error!(
            error=?err,
            "euclid_routing: ensure_algorithm_cached failed, falling back"
        );
    })
    .ok()
}

pub fn perform_straight_through_routing(
    algorithm: &routing_types::StraightThroughAlgorithm,
    creds_identifier: Option<&str>,
) -> RoutingResult<(Vec<routing_types::RoutableConnectorChoice>, bool)> {
    Ok(match algorithm {
        routing_types::StraightThroughAlgorithm::Single(conn) => {
            (vec![(**conn).clone()], creds_identifier.is_none())
        }

        routing_types::StraightThroughAlgorithm::Priority(conns) => (conns.clone(), true),

        routing_types::StraightThroughAlgorithm::VolumeSplit(splits) => (
            perform_volume_split(splits.to_vec())
                .change_context(errors::RoutingError::ConnectorSelectionFailed)
                .attach_printable(
                    "Volume Split connector selection error in straight through routing",
                )?,
            true,
        ),
    })
}

pub fn perform_routing_for_single_straight_through_algorithm(
    algorithm: &routing_types::StraightThroughAlgorithm,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    Ok(match algorithm {
        routing_types::StraightThroughAlgorithm::Single(connector) => vec![(**connector).clone()],

        routing_types::StraightThroughAlgorithm::Priority(_)
        | routing_types::StraightThroughAlgorithm::VolumeSplit(_) => {
            Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
                .attach_printable("Unsupported algorithm received as a result of static routing")?
        }
    })
}

fn execute_dsl_v1(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<ConnectorSelection>,
) -> RoutingResult<routing_types::StaticRoutingAlgorithm> {
    interpreter
        .execute(backend_input)
        .map(|out| out.connector_selection.foreign_into())
        .change_context(errors::RoutingError::DslExecutionError)
}

fn dsl_output_to_connectors(
    routing_output: routing_types::StaticRoutingAlgorithm,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    Ok(match routing_output {
        routing_types::StaticRoutingAlgorithm::Priority(plist) => plist,

        routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => perform_volume_split(splits)
            .change_context(errors::RoutingError::DslFinalConnectorSelectionFailed)?,

        _ => Err(errors::RoutingError::DslIncorrectSelectionAlgorithm)
            .attach_printable("Unsupported algorithm received as a result of static routing")?,
    })
}

fn execute_dsl_and_get_connector_v1(
    backend_input: dsl_inputs::BackendInput,
    interpreter: &backend::VirInterpreterBackend<ConnectorSelection>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    dsl_output_to_connectors(execute_dsl_v1(backend_input, interpreter)?)
}

pub async fn refresh_routing_cache_v1(
    state: &SessionState,
    key: String,
    algorithm_id: &common_utils::id_type::RoutingId,
    profile_id: &common_utils::id_type::ProfileId,
) -> RoutingResult<Arc<CachedAlgorithm>> {
    let algorithm = {
        let algorithm = state
            .store
            .find_routing_algorithm_by_profile_id_algorithm_id(profile_id, algorithm_id)
            .await
            .change_context(errors::RoutingError::DslMissingInDb)?;
        let algorithm: routing_types::StaticRoutingAlgorithm = algorithm
            .algorithm_data
            .parse_value("RoutingAlgorithm")
            .change_context(errors::RoutingError::DslParsingError)?;
        algorithm
    };

    let cached_algorithm = match algorithm {
        routing_types::StaticRoutingAlgorithm::Single(conn) => CachedAlgorithm::Single(conn),
        routing_types::StaticRoutingAlgorithm::Priority(plist) => CachedAlgorithm::Priority(plist),
        routing_types::StaticRoutingAlgorithm::VolumeSplit(splits) => {
            CachedAlgorithm::VolumeSplit(splits)
        }
        routing_types::StaticRoutingAlgorithm::Advanced(program) => {
            let interpreter = backend::VirInterpreterBackend::with_program(program)
                .change_context(errors::RoutingError::DslBackendInitError)
                .attach_printable("Error initializing DSL interpreter backend")?;

            CachedAlgorithm::Advanced(interpreter)
        }
        api_models::routing::StaticRoutingAlgorithm::ThreeDsDecisionRule(_program) => {
            Err(errors::RoutingError::InvalidRoutingAlgorithmStructure)
                .attach_printable("Unsupported algorithm received")?
        }
    };

    let arc_cached_algorithm = Arc::new(cached_algorithm);

    ROUTING_CACHE
        .push(
            CacheKey {
                key,
                prefix: state.tenant.redis_key_prefix.clone(),
            },
            arc_cached_algorithm.clone(),
        )
        .await;

    Ok(arc_cached_algorithm)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub fn perform_dynamic_routing_volume_split(
    splits: Vec<api_models::routing::RoutingVolumeSplit>,
    rng_seed: Option<&str>,
) -> RoutingResult<api_models::routing::RoutingVolumeSplit> {
    let weights: Vec<u8> = splits.iter().map(|sp| sp.split).collect();
    let weighted_index = distributions::WeightedIndex::new(weights)
        .change_context(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Error creating weighted distribution for volume split")?;

    let idx = if let Some(seed) = rng_seed {
        let mut hasher = hash_map::DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(hash);
        weighted_index.sample(&mut rng)
    } else {
        let mut rng = rand::thread_rng();
        weighted_index.sample(&mut rng)
    };

    let routing_choice = *splits
        .get(idx)
        .ok_or(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Volume split index lookup failed")?;

    Ok(routing_choice)
}

pub fn perform_volume_split(
    mut splits: Vec<routing_types::ConnectorVolumeSplit>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let weights: Vec<u8> = splits.iter().map(|sp| sp.split).collect();
    let weighted_index = distributions::WeightedIndex::new(weights)
        .change_context(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Error creating weighted distribution for volume split")?;

    let mut rng = rand::thread_rng();
    let idx = weighted_index.sample(&mut rng);

    splits
        .get(idx)
        .ok_or(errors::RoutingError::VolumeSplitFailed)
        .attach_printable("Volume split index lookup failed")?;

    // Panic Safety: We have performed a `get(idx)` operation just above which will
    // ensure that the index is always present, else throw an error.
    let removed = splits.remove(idx);
    splits.insert(0, removed);

    Ok(splits.into_iter().map(|sp| sp.connector).collect())
}

// #[cfg(feature = "v1")]
pub async fn get_merchant_cgraph(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>> {
    let merchant_id = &key_store.merchant_id;

    let key = {
        match transaction_type {
            api_enums::TransactionType::Payment => {
                format!(
                    "cgraph_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr()
                )
            }
            #[cfg(feature = "payouts")]
            api_enums::TransactionType::Payout => {
                format!(
                    "cgraph_po_{}_{}",
                    merchant_id.get_string_repr(),
                    profile_id.get_string_repr()
                )
            }
            api_enums::TransactionType::ThreeDsAuthentication => {
                Err(errors::RoutingError::InvalidTransactionType)?
            }
        }
    };

    let cached_cgraph = CGRAPH_CACHE
        .get_val::<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>>(
            CacheKey {
                key: key.clone(),
                prefix: state.tenant.redis_key_prefix.clone(),
            },
        )
        .await;

    let cgraph = if let Some(graph) = cached_cgraph {
        graph
    } else {
        refresh_cgraph_cache(state, key_store, key.clone(), profile_id, transaction_type).await?
    };

    Ok(cgraph)
}

// #[cfg(feature = "v1")]
pub async fn refresh_cgraph_cache(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    key: String,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<Arc<hyperswitch_constraint_graph::ConstraintGraph<euclid_dir::DirValue>>> {
    // Fetch the MCA list from the DB here (only reached on a cache miss) rather than
    // reusing a caller-supplied snapshot, so the cached graph is always built from
    // live data and cannot be poisoned by a stale pre-eviction snapshot.
    let mut merchant_connector_accounts = state
        .store
        .list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
            &key_store.merchant_id,
            profile_id,
        )
        .await
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;

    match transaction_type {
        api_enums::TransactionType::Payment => {
            merchant_connector_accounts.retain(|mca| {
                mca.connector_type != storage_enums::ConnectorType::PaymentVas
                    && mca.connector_type != storage_enums::ConnectorType::PaymentMethodAuth
                    && mca.connector_type != storage_enums::ConnectorType::PayoutProcessor
                    && mca.connector_type != storage_enums::ConnectorType::AuthenticationProcessor
            });
        }
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => {
            merchant_connector_accounts
                .retain(|mca| mca.connector_type == storage_enums::ConnectorType::PayoutProcessor);
        }
        api_enums::TransactionType::ThreeDsAuthentication => {
            Err(errors::RoutingError::InvalidTransactionType)?
        }
    };

    let connector_type = match transaction_type {
        api_enums::TransactionType::Payment => common_enums::ConnectorType::PaymentProcessor,
        #[cfg(feature = "payouts")]
        api_enums::TransactionType::Payout => common_enums::ConnectorType::PayoutProcessor,
        api_enums::TransactionType::ThreeDsAuthentication => {
            Err(errors::RoutingError::InvalidTransactionType)?
        }
    };

    let merchant_connector_accounts =
        merchant_connector_accounts.filter_by_connector_type(connector_type);

    let api_mcas = merchant_connector_accounts
        .into_iter()
        .map(admin_api::MCACGraphData::foreign_try_from)
        .collect::<Result<Vec<_>, _>>()
        .change_context(errors::RoutingError::KgraphCacheRefreshFailed)?;
    let connector_configs = state
        .conf
        .pm_filters
        .0
        .clone()
        .into_iter()
        .filter(|(key, _)| key != "default")
        .map(|(key, value)| {
            let key = api_enums::RoutableConnectors::from_str(&key).map_err(|error| {
                logger::error!(
                    error=?error,
                    connector_name = %key,
                    "euclid: invalid connector name in pm_filters config"
                );
                errors::RoutingError::InvalidConnectorName(key)
            })?;

            Ok((key, value.foreign_into()))
        })
        .collect::<Result<HashMap<_, _>, errors::RoutingError>>()?;
    let default_configs = state
        .conf
        .pm_filters
        .0
        .get("default")
        .cloned()
        .map(ForeignFrom::foreign_from);
    let config_pm_filters = CountryCurrencyFilter {
        connector_configs,
        default_configs,
    };
    let cgraph = Arc::new(
        mca_graph::make_mca_graph(api_mcas, &config_pm_filters)
            .change_context(errors::RoutingError::KgraphCacheRefreshFailed)
            .attach_printable("when construction cgraph")?,
    );

    CGRAPH_CACHE
        .push(
            CacheKey {
                key,
                prefix: state.tenant.redis_key_prefix.clone(),
            },
            Arc::clone(&cgraph),
        )
        .await;

    Ok(cgraph)
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_cgraph_filtering(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    backend_input: dsl_inputs::BackendInput,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
    transaction_type: &api_enums::TransactionType,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let context = euclid_graph::AnalysisContext::from_dir_values(
        backend_input
            .into_context()
            .change_context(errors::RoutingError::KgraphAnalysisError)?,
    );

    let cached_cgraph = get_merchant_cgraph(state, key_store, profile_id, transaction_type).await?;

    let mut final_selection = Vec::new();

    for choice in chosen {
        let routable_connector = choice.connector;

        let euclid_choice: ast::ConnectorChoice = choice.clone().foreign_into();
        let dir_val = euclid_choice
            .into_dir_value()
            .change_context(errors::RoutingError::KgraphAnalysisError)?;

        let cgraph_eligible = cached_cgraph
            .check_value_validity(
                dir_val,
                &context,
                &mut hyperswitch_constraint_graph::Memoization::new(),
                &mut hyperswitch_constraint_graph::CycleCheck::new(),
                None,
            )
            .change_context(errors::RoutingError::KgraphAnalysisError)?;

        let filter_eligible =
            eligible_connectors.is_none_or(|list| list.contains(&routable_connector));

        let mca_active = choice
            .merchant_connector_id
            .as_ref()
            .map(|id| active_mca_ids.contains(id))
            .unwrap_or(false);

        if cgraph_eligible && filter_eligible && mca_active {
            final_selection.push(choice);
        }
    }

    Ok(final_selection)
}

#[cfg(feature = "v1")]
fn is_installment_payment(payment_data: &routing::PaymentsDslInput<'_>) -> bool {
    payment_data.payment_attempt.installment_data.is_some()
}

#[cfg(feature = "v1")]
fn get_payment_method_and_type_for_installment(
    payment_data: &routing::PaymentsDslInput<'_>,
) -> (
    Option<api_enums::PaymentMethod>,
    Option<api_enums::PaymentMethodType>,
) {
    (
        payment_data.payment_attempt.payment_method,
        payment_data.payment_attempt.payment_method_type,
    )
}

#[cfg(feature = "v1")]
fn get_payment_data_for_installments<'a>(
    transaction_data: &'a routing::TransactionData<'a>,
) -> Option<&'a routing::PaymentsDslInput<'a>> {
    match transaction_data {
        routing::TransactionData::Payment(payment_data) => Some(payment_data),
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(_) => None,
    }
}

#[cfg(feature = "v1")]
fn get_installment_supported_connectors(
    state: &SessionState,
    transaction_data: &routing::TransactionData<'_>,
) -> Option<Vec<api_enums::RoutableConnectors>> {
    get_payment_data_for_installments(transaction_data)
        .filter(|payment_data| is_installment_payment(payment_data))
        .map(|payment_data| {
            let (payment_method, payment_method_type) =
                get_payment_method_and_type_for_installment(payment_data);

            payment_method
                .zip(payment_method_type)
                .and_then(|(payment_method, payment_method_type)| {
                    state
                        .conf
                        .installments
                        .supported_payment_methods
                        .0
                        .get(&payment_method)
                        .and_then(|supported_payment_method_types| {
                            supported_payment_method_types.0.get(&payment_method_type)
                        })
                        .map(|supported_connectors| {
                            supported_connectors
                                .0
                                .iter()
                                .filter_map(|connector| {
                                    api_enums::RoutableConnectors::from_str(
                                        connector.to_string().as_str(),
                                    )
                                    .ok()
                                })
                                .collect::<Vec<_>>()
                        })
                })
                .unwrap_or_default()
        })
}

#[cfg(feature = "v1")]
fn update_eligible_connectors_for_installments(
    state: &SessionState,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<Vec<api_enums::RoutableConnectors>>,
) -> Option<Vec<api_enums::RoutableConnectors>> {
    let installment_supported_connectors =
        get_installment_supported_connectors(state, transaction_data);

    eligible_connectors
        .map(|existing_eligible_connectors| {
            installment_supported_connectors.as_ref().map_or(
                existing_eligible_connectors.clone(),
                |installment_supported_connectors| {
                    installment_supported_connectors
                        .iter()
                        .filter(|connector| existing_eligible_connectors.contains(connector))
                        .copied()
                        .collect()
                },
            )
        })
        .or(installment_supported_connectors)
}

pub async fn perform_eligibility_analysis(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    profile_id: &common_utils::id_type::ProfileId,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let backend_input = match transaction_data {
        routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data)?,
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => make_dsl_input_for_payouts(payout_data)?,
    };

    perform_cgraph_filtering(
        state,
        key_store,
        chosen,
        backend_input,
        eligible_connectors,
        profile_id,
        &api_enums::TransactionType::from(transaction_data),
        active_mca_ids,
    )
    .await
}

/// Fetches the merchant's default (fallback) list of routable connectors.
///
/// This is the ultimate degrade target for routing: it never applies cgraph/MCA
/// filtering, so it can be returned as-is when the active MCA list is unavailable.
#[cfg_attr(feature = "v2", allow(clippy::unused_async))]
async fn get_fallback_config(
    state: &SessionState,
    transaction_data: &routing::TransactionData<'_>,
    #[cfg(feature = "v1")] _business_profile: &domain::Profile,
    #[cfg(feature = "v2")] business_profile: &domain::Profile,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    #[cfg(feature = "v1")]
    {
        routing::helpers::get_merchant_default_config(
            &*state.store,
            match transaction_data {
                routing::TransactionData::Payment(payment_data) => payment_data
                    .payment_intent
                    .profile_id
                    .as_ref()
                    .get_required_value("profile_id")
                    .change_context(errors::RoutingError::ProfileIdMissing)?
                    .get_string_repr(),
                #[cfg(feature = "payouts")]
                routing::TransactionData::Payout(payout_data) => {
                    payout_data.payout_attempt.profile_id.get_string_repr()
                }
            },
            &api_enums::TransactionType::from(transaction_data),
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)
    }
    #[cfg(feature = "v2")]
    {
        let _ = (state, transaction_data);
        admin::ProfileWrapper::new(business_profile.clone())
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)
    }
}

pub async fn perform_fallback_routing(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<&Vec<api_enums::RoutableConnectors>>,
    business_profile: &domain::Profile,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    let fallback_config = get_fallback_config(state, transaction_data, business_profile).await?;
    let backend_input = match transaction_data {
        routing::TransactionData::Payment(payment_data) => make_dsl_input(payment_data)?,
        #[cfg(feature = "payouts")]
        routing::TransactionData::Payout(payout_data) => make_dsl_input_for_payouts(payout_data)?,
    };
    perform_cgraph_filtering(
        state,
        key_store,
        fallback_config,
        backend_input,
        eligible_connectors,
        business_profile.get_id(),
        &api_enums::TransactionType::from(transaction_data),
        active_mca_ids,
    )
    .await
}

pub async fn perform_eligibility_analysis_with_fallback(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    chosen: Vec<routing_types::RoutableConnectorChoice>,
    transaction_data: &routing::TransactionData<'_>,
    eligible_connectors: Option<Vec<api_enums::RoutableConnectors>>,
    business_profile: &domain::Profile,
) -> RoutingResult<Vec<routing_types::RoutableConnectorChoice>> {
    logger::debug!("euclid_routing: performing eligibility");

    #[cfg(feature = "v1")]
    let eligible_connectors =
        update_eligible_connectors_for_installments(state, transaction_data, eligible_connectors);

    // If the active-MCA fetch fails (e.g. a transient DB error), degrade to the
    // merchant's fallback config instead of aborting the payment — log and return the
    // fallback config, still restricted to the caller-specified eligible connectors
    // (already intersected for installments) so a requested connector is never silently
    // swapped for an arbitrary default. Hard failure can still occur downstream (e.g. a
    // cold-cache refresh errors after this fetch succeeded); only the MCA-fetch failure
    // is degraded here. Never populate the cgraph cache from a failed fetch.
    let active_mca_ids =
        match get_active_merchant_connector_accounts(state, key_store, business_profile.get_id())
            .await
        {
            Ok(merchant_connector_accounts) => merchant_connector_accounts.get_ids(),
            Err(err) => {
                logger::error!(
                    error = ?err,
                    "euclid_routing: failed to fetch active merchant connector accounts; \
                     degrading to eligibility-filtered fallback config"
                );
                return get_fallback_config(state, transaction_data, business_profile)
                    .await
                    .map(|mut fallback_config| {
                        if let Some(eligible) = eligible_connectors {
                            fallback_config.retain(|choice| eligible.contains(&choice.connector));
                        }
                        fallback_config
                    });
            }
        };

    let mut final_selection = perform_eligibility_analysis(
        state,
        key_store,
        chosen,
        transaction_data,
        eligible_connectors.as_ref(),
        business_profile.get_id(),
        &active_mca_ids,
    )
    .await?;

    let fallback_selection = perform_fallback_routing(
        state,
        key_store,
        transaction_data,
        eligible_connectors.as_ref(),
        business_profile,
        &active_mca_ids,
    )
    .await;

    final_selection.append(
        &mut fallback_selection
            .unwrap_or_default()
            .iter()
            .filter(|&routable_connector_choice| {
                !final_selection.contains(routable_connector_choice)
            })
            .cloned()
            .collect::<Vec<_>>(),
    );

    let final_selected_connectors = final_selection
        .iter()
        .map(|item| item.connector)
        .collect::<Vec<_>>();
    logger::debug!(final_selected_connectors_for_routing=?final_selected_connectors, "euclid_routing: List of final selected connectors for routing");

    Ok(final_selection)
}

#[cfg(feature = "v2")]
pub async fn perform_session_flow_routing<'a>(
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    session_input: SessionFlowRoutingInput<'_>,
    business_profile: &domain::Profile,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>>
{
    let mut pm_type_map: FxHashMap<
        api_enums::PaymentMethodType,
        FxHashMap<SessionRoutingConnectorKey, api::GetToken>,
    > = FxHashMap::default();

    let profile_id = business_profile.get_id().clone();

    let routing_algorithm =
        MerchantAccountRoutingAlgorithm::V1(business_profile.routing_algorithm_id.clone());

    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: None,
        card_discovery: None,
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: session_input
            .payment_intent
            .amount_details
            .calculate_net_amount(),
        transaction_initiator: None,
        currency: session_input.payment_intent.amount_details.currency,
        authentication_type: session_input.payment_intent.authentication_type,
        card_bin: None,
        extended_card_bin: None,
        capture_method: Option::<euclid_enums::CaptureMethod>::foreign_from(
            session_input.payment_intent.capture_method,
        ),
        // business_country not available in payment_intent anymore
        business_country: None,
        billing_country: session_input
            .country
            .map(storage_enums::Country::from_alpha2),
        // business_label not available in payment_intent anymore
        business_label: None,
        setup_future_usage: Some(session_input.payment_intent.setup_future_usage),
        surcharge_amount: None,
    };

    let metadata = session_input
        .payment_intent
        .parse_and_get_metadata("routing_parameters")
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    let mut backend_input = dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: dsl_inputs::MandateData {
            mandate_acceptance_type: None,
            mandate_type: None,
            payment_type: None,
        },
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    };

    for connector_data in session_input.chosen.iter() {
        pm_type_map
            .entry(connector_data.payment_method_sub_type)
            .or_default()
            .insert(
                connector_data.connector.merchant_connector_id.clone(),
                connector_data.connector.get_token.clone(),
            );
    }

    let mut result: FxHashMap<
        api_enums::PaymentMethodType,
        Vec<routing_types::SessionRoutingChoice>,
    > = FxHashMap::default();
    let active_mca_ids = get_active_mca_ids_for_session(state, key_store, &profile_id).await;

    for (pm_type, allowed_connectors) in pm_type_map {
        let euclid_pmt: euclid_enums::PaymentMethodType = pm_type;
        let euclid_pm: euclid_enums::PaymentMethod = euclid_pmt.into();

        backend_input.payment_method.payment_method = Some(euclid_pm);
        backend_input.payment_method.payment_method_type = Some(euclid_pmt);

        let session_pm_input = SessionRoutingPmTypeInput {
            routing_algorithm: &routing_algorithm,
            backend_input: backend_input.clone(),
            allowed_connectors,
            profile_id: &profile_id,
        };

        let routable_connector_choice_option = perform_session_routing_for_pm_type(
            state,
            key_store,
            &session_pm_input,
            transaction_type,
            business_profile,
            &active_mca_ids,
        )
        .await?;

        if let Some(routable_connector_choice) = routable_connector_choice_option {
            let mut session_routing_choice: Vec<routing_types::SessionRoutingChoice> = Vec::new();

            for selection in routable_connector_choice {
                let connector_name = selection.connector.to_string();
                if let Some(get_token) = session_pm_input
                    .allowed_connectors
                    .get(&selection.merchant_connector_id)
                {
                    let connector_data = api::ConnectorData::get_connector_by_name(
                        &state.clone().conf.connectors,
                        &connector_name,
                        get_token.clone(),
                        selection.merchant_connector_id,
                    )
                    .change_context(errors::RoutingError::InvalidConnectorName(connector_name))?;

                    session_routing_choice.push(routing_types::SessionRoutingChoice {
                        connector: connector_data,
                        payment_method_type: pm_type,
                    });
                }
            }
            if !session_routing_choice.is_empty() {
                result.insert(pm_type, session_routing_choice);
            }
        }
    }

    Ok(result)
}

#[cfg(feature = "v1")]
pub async fn perform_session_flow_routing(
    session_input: SessionFlowRoutingInput<'_>,
    business_profile: &domain::Profile,
    dimensions: &dimension_state::DimensionsWithProcessorAndProviderMerchantIdAndProfileId,
    transaction_type: &api_enums::TransactionType,
) -> RoutingResult<(
    FxHashMap<api_enums::PaymentMethodType, Vec<routing_types::SessionRoutingChoice>>,
    Option<common_enums::RoutingApproach>,
)> {
    let mut pm_type_map: FxHashMap<
        api_enums::PaymentMethodType,
        FxHashMap<SessionRoutingConnectorKey, api::GetToken>,
    > = FxHashMap::default();

    let profile_id = session_input
        .payment_intent
        .profile_id
        .clone()
        .get_required_value("profile_id")
        .change_context(errors::RoutingError::ProfileIdMissing)?;

    let routing_algorithm: MerchantAccountRoutingAlgorithm = {
        business_profile
            .routing_algorithm
            .clone()
            .map(|val| val.parse_value("MerchantAccountRoutingAlgorithm"))
            .transpose()
            .change_context(errors::RoutingError::InvalidRoutingAlgorithmStructure)?
            .unwrap_or_default()
    };

    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: None,
        card_discovery: None,
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: session_input.payment_attempt.get_total_amount(),
        transaction_initiator: match session_input.payment_intent.off_session {
            Some(true) => Some(euclid_dir::enums::TransactionInitiator::Merchant),
            _ => Some(euclid_dir::enums::TransactionInitiator::Customer),
        },
        currency: session_input
            .payment_intent
            .currency
            .get_required_value("Currency")
            .change_context(errors::RoutingError::DslMissingRequiredField {
                field_name: "currency".into(),
            })?,
        authentication_type: session_input.payment_attempt.authentication_type,
        card_bin: None,
        extended_card_bin: None,
        capture_method: session_input
            .payment_attempt
            .capture_method
            .and_then(Option::<euclid_enums::CaptureMethod>::foreign_from),
        business_country: session_input
            .payment_intent
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: session_input
            .country
            .map(storage_enums::Country::from_alpha2),
        business_label: session_input.payment_intent.business_label.clone(),
        setup_future_usage: session_input.payment_intent.setup_future_usage,
        surcharge_amount: None,
    };

    let metadata = session_input
        .payment_intent
        .parse_and_get_metadata("routing_parameters")
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);

    let backend_input = dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: dsl_inputs::MandateData {
            mandate_acceptance_type: None,
            mandate_type: None,
            payment_type: None,
        },
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    };

    for connector_data in session_input.chosen.iter() {
        pm_type_map
            .entry(connector_data.payment_method_sub_type)
            .or_default()
            .insert(
                connector_data.connector.merchant_connector_id.clone(),
                connector_data.connector.get_token.clone(),
            );
    }

    let mut result: FxHashMap<
        api_enums::PaymentMethodType,
        Vec<routing_types::SessionRoutingChoice>,
    > = FxHashMap::default();
    let mut final_routing_approach = None;
    let active_mca_ids =
        get_active_mca_ids_for_session(session_input.state, session_input.key_store, &profile_id)
            .await;

    // Independent of payment method type, so resolved once rather than per iteration.
    let de_routing_effective =
        utils::is_decision_engine_routing_effective(session_input.state, dimensions).await;

    let payment_id = session_input
        .payment_intent
        .payment_id
        .get_string_repr()
        .to_string();

    // Built up front so the Decision Engine calls can be issued together rather than one
    // wallet type at a time. A rule may branch on payment method type, so they cannot be
    // collapsed into one call -- but they need not be serialised.
    let pm_entries = pm_type_map
        .into_iter()
        .map(|(pm_type, allowed_connectors)| {
            let euclid_pmt: euclid_enums::PaymentMethodType = pm_type;
            let euclid_pm: euclid_enums::PaymentMethod = euclid_pmt.into();
            let mut backend_input = backend_input.clone();
            backend_input.payment_method.payment_method = Some(euclid_pm);
            backend_input.payment_method.payment_method_type = Some(euclid_pmt);
            (pm_type, allowed_connectors, backend_input)
        })
        .collect::<Vec<_>>();

    // Not cut over, the evaluation is shadow-only and off the request path.
    let collect_shadow_entries = !de_routing_effective
        && session_input.state.conf.open_router.static_routing_enabled
        && session_input.state.conf.open_router.shadow_routing_enabled
        && profile_has_active_routing_algorithm(business_profile);

    // Same list for every wallet type, so it is fetched once rather than per iteration.
    let de_fallback_config = if de_routing_effective || collect_shadow_entries {
        routing::helpers::get_merchant_default_config(
            &*session_input.state.clone().store,
            profile_id.get_string_repr(),
            transaction_type,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
    } else {
        Vec::new()
    };

    // One batch call for a cut-over profile: the engine fetches the rule once and
    // evaluates every wallet type's parameters in a single round trip. Against an engine
    // without the batch endpoint this degrades to concurrent single calls.
    let de_results: Vec<Vec<routing_types::RoutableConnectorChoice>> = if de_routing_effective {
        utils::decision_engine_routing_batch_with_fallback(
            session_input.state,
            pm_entries
                .iter()
                .map(|(_, _, backend_input)| backend_input.clone())
                .collect(),
            business_profile,
            payment_id.clone(),
            de_fallback_config.clone(),
            *transaction_type,
            utils::RoutingFlow::PaymentMethodList,
        )
        .await
    } else {
        vec![Vec::new(); pm_entries.len()]
    };

    let mut shadow_entries: Vec<utils::ShadowBatchEntry> = Vec::new();

    for ((pm_type, allowed_connectors, backend_input), de_connectors) in
        pm_entries.into_iter().zip(de_results)
    {
        let session_pm_input = SessionRoutingPmTypeInput {
            state: session_input.state,
            key_store: session_input.key_store,
            // attempt_id: session_input.payment_attempt.get_id(),
            routing_algorithm: &routing_algorithm,
            backend_input,
            allowed_connectors,
            profile_id: &profile_id,
            dimensions,
            payment_id: payment_id.clone(),
        };

        let (routable_connector_choice_option, routing_approach, shadow_entry) =
            perform_session_routing_for_pm_type(
                &session_pm_input,
                transaction_type,
                business_profile,
                &active_mca_ids,
                de_routing_effective,
                de_connectors,
                collect_shadow_entries,
            )
            .await?;

        if let Some(entry) = shadow_entry {
            shadow_entries.push(entry);
        }

        final_routing_approach = routing_approach;

        if let Some(routable_connector_choice) = routable_connector_choice_option {
            let mut session_routing_choice: Vec<routing_types::SessionRoutingChoice> = Vec::new();

            for selection in routable_connector_choice {
                let connector_name = selection.connector.to_string();
                if let Some(get_token) = session_pm_input
                    .allowed_connectors
                    .get(&selection.merchant_connector_id)
                {
                    let connector_data = api::ConnectorData::get_connector_by_name(
                        &session_pm_input.state.clone().conf.connectors,
                        &connector_name,
                        get_token.clone(),
                        selection.merchant_connector_id,
                    )
                    .change_context(errors::RoutingError::InvalidConnectorName(connector_name))?;

                    session_routing_choice.push(routing_types::SessionRoutingChoice {
                        connector: connector_data,
                        payment_method_type: pm_type,
                    });
                }
            }
            if !session_routing_choice.is_empty() {
                result.insert(pm_type, session_routing_choice);
            }
        }
    }

    // One spawned batch evaluation for the whole request, replacing a spawned call per
    // wallet type. Off the request path; diff logging only.
    if !shadow_entries.is_empty() {
        spawn_session_shadow_batch_evaluation(
            session_input.state,
            business_profile,
            payment_id,
            shadow_entries,
            de_fallback_config,
            *transaction_type,
            utils::RoutingFlow::PaymentMethodList,
        );
    }

    Ok((result, final_routing_approach))
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn perform_session_routing_for_pm_type(
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    business_profile: &domain::Profile,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
    de_routing_effective: bool,
    de_connectors: Vec<api_models::routing::RoutableConnectorChoice>,
    collect_shadow_entry: bool,
) -> RoutingResult<(
    Option<Vec<api_models::routing::RoutableConnectorChoice>>,
    Option<common_enums::RoutingApproach>,
    Option<utils::ShadowBatchEntry>,
)> {
    let merchant_id = &session_pm_input.key_store.merchant_id;

    let algorithm_id = match session_pm_input.routing_algorithm {
        MerchantAccountRoutingAlgorithm::V1(algorithm_ref) => &algorithm_ref.algorithm_id,
    };

    // The Decision Engine call moved to the caller (one batch per request), so the
    // fallback list is only needed here as the no-algorithm result.
    let fallback_config = if algorithm_id.is_none() {
        routing::helpers::get_merchant_default_config(
            &*session_pm_input.state.clone().store,
            session_pm_input.profile_id.get_string_repr(),
            transaction_type,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
    } else {
        Vec::new()
    };

    // Evaluated even under cutover, so an empty DE result falls back to the merchant's own
    // rule rather than the flat fallback list.
    let (chosen_connectors, routing_approach) = if let Some(ref algorithm_id) = algorithm_id {
        let cached_algorithm = ensure_algorithm_cached_v1(
            &session_pm_input.state.clone(),
            merchant_id,
            algorithm_id,
            session_pm_input.profile_id,
            transaction_type,
        )
        .await?;

        match cached_algorithm.as_ref() {
            CachedAlgorithm::Single(conn) => (
                vec![(**conn).clone()],
                Some(common_enums::RoutingApproach::StraightThroughRouting),
            ),
            CachedAlgorithm::Priority(plist) => (plist.clone(), None),
            CachedAlgorithm::VolumeSplit(splits) => (
                perform_volume_split(splits.to_vec())
                    .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
                Some(common_enums::RoutingApproach::VolumeBasedRouting),
            ),
            CachedAlgorithm::Advanced(interpreter) => (
                execute_dsl_and_get_connector_v1(
                    session_pm_input.backend_input.clone(),
                    interpreter,
                )?,
                Some(common_enums::RoutingApproach::RuleBasedRouting),
            ),
        }
    } else {
        (fallback_config.clone(), None)
    };

    let is_volume_split = matches!(
        routing_approach,
        Some(common_enums::RoutingApproach::VolumeBasedRouting)
    );

    // Load-bearing only for a cut-over profile; everyone else gets it shadow-evaluated off
    // the request path, so the diff stays visible without adding a round trip per wallet
    // type to the payment method list.
    let chosen_connectors = if de_routing_effective {
        // Diff logging only; see the note in `SessionRoutingStage` on why the kill switch
        // is not fed from these flows.
        utils::compare_and_log_result(
            de_connectors.clone(),
            chosen_connectors.clone(),
            utils::RoutingFlow::PaymentMethodList.as_str().to_string(),
            is_volume_split,
        );

        // Connector list only; see the note in `SessionRoutingStage` on why
        // `routing_approach` is left untouched.
        utils::select_routing_result(
            session_pm_input.state,
            session_pm_input.dimensions,
            business_profile,
            chosen_connectors,
            de_connectors,
        )
        .await
    } else {
        chosen_connectors
    };

    // Handed back to the caller, which shadow-evaluates the whole request in one
    // spawned batch instead of one task per wallet type.
    let shadow_entry = collect_shadow_entry.then(|| utils::ShadowBatchEntry {
        backend_input: session_pm_input.backend_input.clone(),
        hs_connectors: chosen_connectors.clone(),
        is_volume: is_volume_split,
    });

    let mut final_selection = perform_cgraph_filtering(
        &session_pm_input.state.clone(),
        session_pm_input.key_store,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        session_pm_input.profile_id,
        transaction_type,
        active_mca_ids,
    )
    .await?;

    if final_selection.is_empty() {
        let fallback = routing::helpers::get_merchant_default_config(
            &*session_pm_input.state.clone().store,
            session_pm_input.profile_id.get_string_repr(),
            transaction_type,
        )
        .await
        .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        final_selection = perform_cgraph_filtering(
            &session_pm_input.state.clone(),
            session_pm_input.key_store,
            fallback,
            session_pm_input.backend_input.clone(),
            None,
            session_pm_input.profile_id,
            transaction_type,
            active_mca_ids,
        )
        .await?;
    }

    if final_selection.is_empty() {
        Ok((None, routing_approach, shadow_entry))
    } else {
        Ok((Some(final_selection), routing_approach, shadow_entry))
    }
}

#[cfg(feature = "v2")]
async fn get_chosen_connectors<'a>(
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    profile_wrapper: &admin::ProfileWrapper,
) -> RoutingResult<Vec<api_models::routing::RoutableConnectorChoice>> {
    let merchant_id = &key_store.merchant_id;

    let MerchantAccountRoutingAlgorithm::V1(algorithm_id) = session_pm_input.routing_algorithm;

    let chosen_connectors = if let Some(ref algorithm_id) = algorithm_id {
        let cached_algorithm = ensure_algorithm_cached_v1(
            state,
            merchant_id,
            algorithm_id,
            session_pm_input.profile_id,
            transaction_type,
        )
        .await?;

        match cached_algorithm.as_ref() {
            CachedAlgorithm::Single(conn) => vec![(**conn).clone()],
            CachedAlgorithm::Priority(plist) => plist.clone(),
            CachedAlgorithm::VolumeSplit(splits) => perform_volume_split(splits.to_vec())
                .change_context(errors::RoutingError::ConnectorSelectionFailed)?,
            CachedAlgorithm::Advanced(interpreter) => execute_dsl_and_get_connector_v1(
                session_pm_input.backend_input.clone(),
                interpreter,
            )?,
        }
    } else {
        profile_wrapper
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?
    };
    Ok(chosen_connectors)
}

#[cfg(feature = "v2")]
async fn perform_session_routing_for_pm_type<'a>(
    state: &'a SessionState,
    key_store: &'a domain::MerchantKeyStore,
    session_pm_input: &SessionRoutingPmTypeInput<'_>,
    transaction_type: &api_enums::TransactionType,
    business_profile: &domain::Profile,
    active_mca_ids: &std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId>,
) -> RoutingResult<Option<Vec<api_models::routing::RoutableConnectorChoice>>> {
    let profile_wrapper = admin::ProfileWrapper::new(business_profile.clone());
    let chosen_connectors = get_chosen_connectors(
        state,
        key_store,
        session_pm_input,
        transaction_type,
        &profile_wrapper,
    )
    .await?;

    let mut final_selection = perform_cgraph_filtering(
        state,
        key_store,
        chosen_connectors,
        session_pm_input.backend_input.clone(),
        None,
        session_pm_input.profile_id,
        transaction_type,
        active_mca_ids,
    )
    .await?;

    if final_selection.is_empty() {
        let fallback = profile_wrapper
            .get_default_fallback_list_of_connector_under_profile()
            .change_context(errors::RoutingError::FallbackConfigFetchFailed)?;

        final_selection = perform_cgraph_filtering(
            state,
            key_store,
            fallback,
            session_pm_input.backend_input.clone(),
            None,
            session_pm_input.profile_id,
            transaction_type,
            active_mca_ids,
        )
        .await?;
    }

    if final_selection.is_empty() {
        Ok(None)
    } else {
        Ok(Some(final_selection))
    }
}
#[cfg(feature = "v2")]
pub fn make_dsl_input_for_surcharge(
    _payment_attempt: &oss_storage::PaymentAttempt,
    _payment_intent: &oss_storage::PaymentIntent,
    _billing_address: Option<Address>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    todo!()
}

#[cfg(feature = "v1")]
pub fn make_dsl_input_for_surcharge(
    payment_attempt: &oss_storage::PaymentAttempt,
    payment_intent: &oss_storage::PaymentIntent,
    billing_address: Option<Address>,
) -> RoutingResult<dsl_inputs::BackendInput> {
    let mandate_data = dsl_inputs::MandateData {
        mandate_acceptance_type: None,
        mandate_type: None,
        payment_type: None,
    };

    let payment_input = dsl_inputs::PaymentInput {
        amount: payment_attempt.get_total_amount(),
        transaction_initiator: match payment_intent.off_session {
            Some(true) => Some(euclid_dir::enums::TransactionInitiator::Merchant),
            _ => Some(euclid_dir::enums::TransactionInitiator::Customer),
        },
        // currency is always populated in payment_attempt during payment create
        currency: payment_attempt
            .currency
            .get_required_value("currency")
            .change_context(errors::RoutingError::DslMissingRequiredField {
                field_name: "currency".into(),
            })?,
        authentication_type: payment_attempt.authentication_type,
        card_bin: None,
        extended_card_bin: None,
        capture_method: payment_attempt.capture_method,
        business_country: payment_intent
            .business_country
            .map(api_enums::Country::from_alpha2),
        billing_country: billing_address
            .and_then(|bic| bic.address)
            .and_then(|add| add.country)
            .map(api_enums::Country::from_alpha2),
        business_label: payment_intent.business_label.clone(),
        setup_future_usage: payment_intent.setup_future_usage,
        surcharge_amount: None,
    };

    let metadata = payment_intent
        .parse_and_get_metadata("routing_parameters")
        .change_context(errors::RoutingError::MetadataParsingError)
        .attach_printable("Unable to parse routing_parameters from metadata of payment_intent")
        .unwrap_or(None);
    let payment_method_input = dsl_inputs::PaymentMethodInput {
        payment_method: None,
        payment_method_type: None,
        card_network: None,
        card_discovery: None,
    };
    let backend_input = dsl_inputs::BackendInput {
        metadata,
        payment: payment_input,
        payment_method: payment_method_input,
        mandate: mandate_data,
        acquirer_data: None,
        customer_device_data: None,
        issuer_data: None,
    };
    Ok(backend_input)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_dynamic_routing_with_open_router(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile: &domain::Profile,
    payment_data: oss_storage::PaymentAttempt,
) -> RoutingResult<Option<RoutingConnectorOutcomeWithApproach>> {
    let dynamic_routing_algo_ref: api_routing::DynamicRoutingAlgorithmRef = profile
        .dynamic_routing_algorithm
        .clone()
        .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::RoutingError::DeserializationError {
            from: "JSON".to_string(),
            to: "DynamicRoutingAlgorithmRef".to_string(),
        })
        .attach_printable("unable to deserialize DynamicRoutingAlgorithmRef from JSON")?
        .ok_or(errors::RoutingError::GenericNotFoundError {
            field: "dynamic_routing_algorithm".to_string(),
        })?;

    logger::debug!(
        "performing dynamic_routing with open_router for profile {}",
        profile.get_id().get_string_repr()
    );

    let is_success_rate_routing_enabled =
        dynamic_routing_algo_ref.is_success_rate_routing_enabled();
    let is_elimination_enabled = dynamic_routing_algo_ref.is_elimination_enabled();

    // Since success_based and elimination routing is being done in 1 api call, we call decide_gateway when either of it enabled
    let connectors = if is_success_rate_routing_enabled || is_elimination_enabled {
        let connectors = perform_decide_gateway_call_with_open_router(
            state,
            routable_connectors.clone(),
            profile.get_id(),
            &payment_data,
            is_elimination_enabled,
        )
        .await?;

        if is_elimination_enabled {
            // This will initiate the elimination process for the connector.
            // Penalize the elimination score of the connector before making a payment.
            // Once the payment is made, we will update the score based on the payment status
            if let Some(connector) = connectors.connectors.first() {
                logger::debug!(
                "penalizing the elimination score of the gateway with id {} in open_router for profile {}",
                connector, profile.get_id().get_string_repr()
            );
                update_gateway_score_with_open_router(
                    state,
                    connector.clone(),
                    profile.get_id(),
                    &payment_data.merchant_id,
                    &payment_data.payment_id,
                    common_enums::AttemptStatus::AuthenticationPending,
                )
                .await?
            }
        }
        Some(connectors)
    } else {
        None
    };
    Ok(connectors)
}

#[cfg(feature = "v1")]
pub async fn perform_open_routing_for_debit_routing<F, D>(
    state: &SessionState,
    co_badged_card_request: or_types::CoBadgedCardRequest,
    card_isin: Option<Secret<String>>,
    old_payment_data: &mut D,
) -> RoutingResult<or_types::DebitRoutingOutput>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let payment_attempt = old_payment_data.get_payment_attempt().clone();

    logger::debug!(
        "performing debit routing with open_router for profile {}",
        payment_attempt.profile_id.get_string_repr()
    );

    let metadata = Some(
        serde_json::to_string(&co_badged_card_request)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode Vaulting data to string")
            .change_context(errors::RoutingError::MetadataParsingError)?,
    );

    let open_router_req_body = OpenRouterDecideGatewayRequest::construct_debit_request(
        &payment_attempt,
        metadata,
        card_isin,
        Some(or_types::RankingAlgorithm::NtwBasedRouting),
    );

    let routing_events_wrapper = utils::RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_attempt.payment_id.get_string_repr().to_string(),
        payment_attempt.profile_id.to_owned(),
        payment_attempt.merchant_id.to_owned(),
        "DecisionEngine: Debit Routing".to_string(),
        Some(open_router_req_body.clone()),
        true,
        true,
    );

    let response: RoutingResult<utils::RoutingEventsResponse<DecidedGateway>> =
        utils::EuclidApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            "decide-gateway",
            Some(open_router_req_body),
            None,
            Some(routing_events_wrapper),
        )
        .await;

    let output = match response {
        Ok(events_response) => {
            let response =
                events_response
                    .response
                    .ok_or(errors::RoutingError::OpenRouterError(
                        "Response from decision engine API is empty".to_string(),
                    ))?;

            let debit_routing_output = response
                .debit_routing_output
                .get_required_value("debit_routing_output")
                .change_context(errors::RoutingError::OpenRouterError(
                    "Failed to parse the response from open_router".into(),
                ))
                .attach_printable("debit_routing_output is missing in the open routing response")?;

            old_payment_data.set_routing_approach_in_attempt(Some(
                common_enums::RoutingApproach::from_decision_engine_approach(
                    &response.routing_approach,
                ),
            ));

            Ok(debit_routing_output)
        }
        Err(error_response) => {
            logger::error!("open_router_error_response: {:?}", error_response);
            Err(errors::RoutingError::OpenRouterError(
                "Failed to perform debit routing in open router".into(),
            ))
        }
    }?;

    Ok(output)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_dynamic_routing_with_intelligent_router<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile: &domain::Profile,
    dynamic_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    let dynamic_routing_algo_ref: api_routing::DynamicRoutingAlgorithmRef = profile
        .dynamic_routing_algorithm
        .clone()
        .map(|val| val.parse_value("DynamicRoutingAlgorithmRef"))
        .transpose()
        .change_context(errors::RoutingError::DeserializationError {
            from: "JSON".to_string(),
            to: "DynamicRoutingAlgorithmRef".to_string(),
        })
        .attach_printable("unable to deserialize DynamicRoutingAlgorithmRef from JSON")?
        .ok_or(errors::RoutingError::GenericNotFoundError {
            field: "dynamic_routing_algorithm".to_string(),
        })?;

    logger::debug!(
        "performing dynamic_routing for profile {}",
        profile.get_id().get_string_repr()
    );

    let payment_attempt = payment_data.get_payment_attempt().clone();

    let mut connector_list = match dynamic_routing_algo_ref
        .success_based_algorithm
        .as_ref()
        .async_map(|algorithm| {
            perform_success_based_routing(
                state,
                routable_connectors.clone(),
                profile.get_id(),
                &payment_attempt.merchant_id,
                &payment_attempt.payment_id,
                dynamic_routing_config_params_interpolator.clone(),
                algorithm.clone(),
                payment_data,
            )
        })
        .await
        .transpose()
        .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
        .ok()
        .flatten()
    {
        Some(success_based_list) => success_based_list,
        None => {
            // Only run contract based if success based returns None
            dynamic_routing_algo_ref
                .contract_based_routing
                .as_ref()
                .async_map(|algorithm| {
                    perform_contract_based_routing(
                        state,
                        routable_connectors.clone(),
                        profile.get_id(),
                        &payment_attempt.merchant_id,
                        &payment_attempt.payment_id,
                        dynamic_routing_config_params_interpolator.clone(),
                        algorithm.clone(),
                        payment_data,
                    )
                })
                .await
                .transpose()
                .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
                .ok()
                .flatten()
                .unwrap_or(routable_connectors.clone())
        }
    };

    connector_list = dynamic_routing_algo_ref
        .elimination_routing_algorithm
        .as_ref()
        .async_map(|algorithm| {
            perform_elimination_routing(
                state,
                connector_list.clone(),
                profile.get_id(),
                &payment_attempt.merchant_id,
                &payment_attempt.payment_id,
                dynamic_routing_config_params_interpolator.clone(),
                algorithm.clone(),
            )
        })
        .await
        .transpose()
        .inspect_err(|e| logger::error!(dynamic_routing_error=?e))
        .ok()
        .flatten()
        .unwrap_or(connector_list);

    Ok(connector_list)
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn perform_decide_gateway_call_with_open_router(
    state: &SessionState,
    mut routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    payment_attempt: &oss_storage::PaymentAttempt,
    is_elimination_enabled: bool,
) -> RoutingResult<RoutingConnectorOutcomeWithApproach> {
    logger::debug!(
        "performing decide_gateway call with open_router for profile {}",
        profile_id.get_string_repr()
    );

    let open_router_req_body = OpenRouterDecideGatewayRequest::construct_sr_request(
        payment_attempt,
        routable_connectors.clone(),
        Some(or_types::RankingAlgorithm::SrBasedRouting),
        is_elimination_enabled,
    );

    let routing_events_wrapper = utils::RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_attempt.payment_id.get_string_repr().to_string(),
        payment_attempt.profile_id.to_owned(),
        payment_attempt.merchant_id.to_owned(),
        "DecisionEngine: SuccessRate decide_gateway".to_string(),
        Some(open_router_req_body.clone()),
        true,
        false,
    );

    let response: RoutingResult<utils::RoutingEventsResponse<DecidedGateway>> =
        utils::SRApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            "decide-gateway",
            Some(open_router_req_body),
            None,
            Some(routing_events_wrapper),
        )
        .await;

    let sr_sorted_connectors = match response {
        Ok(resp) => {
            let decided_gateway: DecidedGateway =
                resp.response.ok_or(errors::RoutingError::OpenRouterError(
                    "Empty response received from open_router".into(),
                ))?;

            let mut routing_event = resp.event.ok_or(errors::RoutingError::RoutingEventsError {
                message: "Decision-Engine: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
                status_code: 500,
            })?;

            routing_event.set_response_body(&decided_gateway);
            routing_event.set_routing_approach(
                utils::RoutingApproach::from_decision_engine_approach(
                    &decided_gateway.routing_approach,
                )
                .to_string(),
            );

            let routing_approach = common_enums::RoutingApproach::from_decision_engine_approach(
                &decided_gateway.routing_approach,
            );

            if let Some(gateway_priority_map) = decided_gateway.gateway_priority_map {
                logger::debug!(gateway_priority_map=?gateway_priority_map, routing_approach=decided_gateway.routing_approach, "open_router decide_gateway call response");
                routable_connectors.sort_by(|connector_choice_a, connector_choice_b| {
                    let connector_choice_a_score = gateway_priority_map
                        .get(&connector_choice_a.to_string())
                        .copied()
                        .unwrap_or(0.0);
                    let connector_choice_b_score = gateway_priority_map
                        .get(&connector_choice_b.to_string())
                        .copied()
                        .unwrap_or(0.0);
                    connector_choice_b_score
                        .partial_cmp(&connector_choice_a_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            routing_event.set_routable_connectors(routable_connectors.clone());
            state.event_handler().log_event(&routing_event);

            Ok(RoutingConnectorOutcomeWithApproach {
                connectors: routable_connectors,
                routing_approach,
            })
        }
        Err(err) => {
            logger::error!("open_router_error_response: {:?}", err);

            Err(errors::RoutingError::OpenRouterError(
                "Failed to perform decide_gateway call in open_router".into(),
            ))
        }
    }?;

    Ok(RoutingConnectorOutcomeWithApproach {
        connectors: sr_sorted_connectors.connectors,
        routing_approach: sr_sorted_connectors.routing_approach,
    })
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
pub async fn update_gateway_score_with_open_router(
    state: &SessionState,
    payment_connector: api_routing::RoutableConnectorChoice,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    payment_status: common_enums::AttemptStatus,
) -> RoutingResult<()> {
    let open_router_req_body = or_types::UpdateScorePayload {
        merchant_id: profile_id.clone(),
        gateway: payment_connector.to_string(),
        status: payment_status.foreign_into(),
        payment_id: payment_id.clone(),
    };

    let routing_events_wrapper = utils::RoutingEventsWrapper::new(
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
        payment_id.get_string_repr().to_string(),
        profile_id.to_owned(),
        merchant_id.to_owned(),
        "DecisionEngine: SuccessRate update_gateway_score".to_string(),
        Some(open_router_req_body.clone()),
        true,
        false,
    );

    let response: RoutingResult<utils::RoutingEventsResponse<or_types::UpdateScoreResponse>> =
        utils::SRApiClient::send_decision_engine_request(
            state,
            services::Method::Post,
            "update-gateway-score",
            Some(open_router_req_body),
            None,
            Some(routing_events_wrapper),
        )
        .await;

    match response {
        Ok(resp) => {
            let update_score_resp = resp.response.ok_or(errors::RoutingError::OpenRouterError(
                "Failed to parse the response from open_router".into(),
            ))?;

            let mut routing_event = resp.event.ok_or(errors::RoutingError::RoutingEventsError {
                message: "Decision-Engine: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
                status_code: 500,
            })?;

            logger::debug!(
                "open_router update_gateway_score response for gateway with id {}: {:?}",
                payment_connector,
                update_score_resp.message
            );

            routing_event.set_response_body(&update_score_resp);
            routing_event.set_payment_connector(payment_connector.clone()); // check this in review
            state.event_handler().log_event(&routing_event);

            Ok(())
        }
        Err(err) => {
            logger::error!("open_router_update_gateway_score_error: {:?}", err);

            Err(errors::RoutingError::OpenRouterError(
                "Failed to update gateway score in open_router".into(),
            ))
        }
    }?;

    Ok(())
}

/// success based dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn perform_success_based_routing<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    success_based_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    success_based_algo_ref: api_routing::SuccessBasedAlgorithm,
    payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if success_based_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing success_based_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = &state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .ok_or(errors::RoutingError::SuccessRateClientInitializationError)
            .attach_printable("dynamic routing gRPC client not found")?
            .success_rate_client;

        let success_based_routing_configs = routing::helpers::fetch_dynamic_routing_configs::<
            api_routing::SuccessBasedRoutingConfig,
        >(
            state,
            profile_id,
            success_based_algo_ref
                .algorithm_id_with_timestamp
                .algorithm_id
                .ok_or(errors::RoutingError::GenericNotFoundError {
                    field: "success_based_routing_algorithm_id".to_string(),
                })
                .attach_printable("success_based_routing_algorithm_id not found in profile_id")?,
        )
        .await
        .change_context(errors::RoutingError::SuccessBasedRoutingConfigError)
        .attach_printable("unable to fetch success_rate based dynamic routing configs")?;

        let success_based_routing_config_params = success_based_routing_config_params_interpolator
            .get_string_val(
                success_based_routing_configs
                    .params
                    .as_ref()
                    .ok_or(errors::RoutingError::SuccessBasedRoutingParamsNotFoundError)?,
            );

        let event_request = utils::CalSuccessRateEventRequest {
            id: profile_id.get_string_repr().to_string(),
            params: success_based_routing_config_params.clone(),
            labels: routable_connectors
                .iter()
                .map(|conn_choice| conn_choice.to_string())
                .collect::<Vec<_>>(),
            config: success_based_routing_configs
                .config
                .as_ref()
                .map(utils::CalSuccessRateConfigEventRequest::from),
        };

        let routing_events_wrapper = utils::RoutingEventsWrapper::new(
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
            payment_id.get_string_repr().to_string(),
            profile_id.to_owned(),
            merchant_id.to_owned(),
            "IntelligentRouter: CalculateSuccessRate".to_string(),
            Some(event_request.clone()),
            true,
            false,
        );

        let closure = || async {
            let success_based_connectors_result = client
                .calculate_success_rate(
                    profile_id.get_string_repr().into(),
                    success_based_routing_configs,
                    success_based_routing_config_params,
                    routable_connectors,
                    state.get_grpc_headers(),
                )
                .await
                .change_context(errors::RoutingError::SuccessRateCalculationError)
                .attach_printable(
                    "unable to calculate/fetch success rate from dynamic routing service",
                );

            match success_based_connectors_result {
                Ok(success_response) => {
                    let updated_resp = utils::CalSuccessRateEventResponse::try_from(
                        &success_response,
                    )
                    .change_context(errors::RoutingError::RoutingEventsError { message: "unable to convert SuccessBasedConnectors to CalSuccessRateEventResponse".to_string(), status_code: 500 })
                    .attach_printable(
                        "unable to convert SuccessBasedConnectors to CalSuccessRateEventResponse",
                    )?;

                    Ok(Some(updated_resp))
                }
                Err(e) => {
                    logger::error!(
                        "unable to calculate/fetch success rate from dynamic routing service: {:?}",
                        e.current_context()
                    );

                    Err(error_stack::report!(
                        errors::RoutingError::SuccessRateCalculationError
                    ))
                }
            }
        };

        let events_response = routing_events_wrapper
            .construct_event_builder(
                "SuccessRateCalculator.FetchSuccessRate".to_string(),
                RoutingEngine::IntelligentRouter,
                ApiMethod::Grpc,
            )?
            .trigger_event(state, closure)
            .await?;

        let success_based_connectors: utils::CalSuccessRateEventResponse = events_response
            .response
            .ok_or(errors::RoutingError::SuccessRateCalculationError)?;

        // Need to log error case
        let mut routing_event =
            events_response
                .event
                .ok_or(errors::RoutingError::RoutingEventsError {
                    message:
                        "SR-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                            .to_string(),
                    status_code: 500,
                })?;

        routing_event.set_routing_approach(success_based_connectors.routing_approach.to_string());
        payment_data.set_routing_approach_in_attempt(Some(common_enums::RoutingApproach::from(
            success_based_connectors.routing_approach,
        )));

        let mut connectors = Vec::with_capacity(success_based_connectors.labels_with_score.len());
        for label_with_score in success_based_connectors.labels_with_score {
            let (connector, merchant_connector_id) = label_with_score.label
                .split_once(':')
                .ok_or(errors::RoutingError::InvalidSuccessBasedConnectorLabel(label_with_score.label.to_string()))
                .attach_printable(
                    "unable to split connector_name and mca_id from the label obtained by the dynamic routing service",
                )?;
            connectors.push(api_routing::RoutableConnectorChoice {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector: euclid::enums::RoutableConnectors::from_str(connector)
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "RoutableConnectors".to_string(),
                    })
                    .attach_printable("unable to convert String to RoutableConnectors")?,
                merchant_connector_id: Some(
                    common_utils::id_type::MerchantConnectorAccountId::wrap(
                        merchant_connector_id.to_string(),
                    )
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "MerchantConnectorAccountId".to_string(),
                    })
                    .attach_printable("unable to convert MerchantConnectorAccountId from string")?,
                ),
            });
        }
        logger::debug!(success_based_routing_connectors=?connectors);

        routing_event.set_status_code(200);
        routing_event.set_routable_connectors(connectors.clone());
        state.event_handler().log_event(&routing_event);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

/// elimination dynamic routing
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
pub async fn perform_elimination_routing(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    elimination_routing_configs_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    elimination_algo_ref: api_routing::EliminationRoutingAlgorithm,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>> {
    if elimination_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing elimination_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = &state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .ok_or(errors::RoutingError::EliminationClientInitializationError)
            .attach_printable("dynamic routing gRPC client not found")?
            .elimination_based_client;

        let elimination_routing_config = routing::helpers::fetch_dynamic_routing_configs::<
            api_routing::EliminationRoutingConfig,
        >(
            state,
            profile_id,
            elimination_algo_ref
                .algorithm_id_with_timestamp
                .algorithm_id
                .ok_or(errors::RoutingError::GenericNotFoundError {
                    field: "elimination_routing_algorithm_id".to_string(),
                })
                .attach_printable(
                    "elimination_routing_algorithm_id not found in business_profile",
                )?,
        )
        .await
        .change_context(errors::RoutingError::EliminationRoutingConfigError)
        .attach_printable("unable to fetch elimination dynamic routing configs")?;

        let elimination_routing_config_params = elimination_routing_configs_params_interpolator
            .get_string_val(
                elimination_routing_config
                    .params
                    .as_ref()
                    .ok_or(errors::RoutingError::EliminationBasedRoutingParamsNotFoundError)?,
            );

        let event_request = utils::EliminationRoutingEventRequest {
            id: profile_id.get_string_repr().to_string(),
            params: elimination_routing_config_params.clone(),
            labels: routable_connectors
                .iter()
                .map(|conn_choice| conn_choice.to_string())
                .collect::<Vec<_>>(),
            config: elimination_routing_config
                .elimination_analyser_config
                .as_ref()
                .map(utils::EliminationRoutingEventBucketConfig::from),
        };

        let routing_events_wrapper = utils::RoutingEventsWrapper::new(
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
            payment_id.get_string_repr().to_string(),
            profile_id.to_owned(),
            merchant_id.to_owned(),
            "IntelligentRouter: PerformEliminationRouting".to_string(),
            Some(event_request.clone()),
            true,
            false,
        );

        let closure = || async {
            let elimination_based_connectors_result = client
                .perform_elimination_routing(
                    profile_id.get_string_repr().to_string(),
                    elimination_routing_config_params,
                    routable_connectors.clone(),
                    elimination_routing_config.elimination_analyser_config,
                    state.get_grpc_headers(),
                )
                .await
                .change_context(errors::RoutingError::EliminationRoutingCalculationError)
                .attach_printable(
                    "unable to analyze/fetch elimination routing from dynamic routing service",
                );

            match elimination_based_connectors_result {
                Ok(elimination_response) => Ok(Some(utils::EliminationEventResponse::from(
                    &elimination_response,
                ))),
                Err(e) => {
                    logger::error!(
                        "unable to analyze/fetch elimination routing from dynamic routing service: {:?}",
                        e.current_context()
                    );

                    Err(error_stack::report!(
                        errors::RoutingError::EliminationRoutingCalculationError
                    ))
                }
            }
        };

        let events_response = routing_events_wrapper
            .construct_event_builder(
                "EliminationAnalyser.GetEliminationStatus".to_string(),
                RoutingEngine::IntelligentRouter,
                ApiMethod::Grpc,
            )?
            .trigger_event(state, closure)
            .await?;

        let elimination_based_connectors: utils::EliminationEventResponse = events_response
            .response
            .ok_or(errors::RoutingError::EliminationRoutingCalculationError)?;

        let mut routing_event = events_response
            .event
            .ok_or(errors::RoutingError::RoutingEventsError {
            message:
                "Elimination-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
            status_code: 500,
        })?;

        routing_event.set_routing_approach(utils::RoutingApproach::Elimination.to_string());

        let mut connectors =
            Vec::with_capacity(elimination_based_connectors.labels_with_status.len());
        let mut eliminated_connectors =
            Vec::with_capacity(elimination_based_connectors.labels_with_status.len());
        let mut non_eliminated_connectors =
            Vec::with_capacity(elimination_based_connectors.labels_with_status.len());
        for labels_with_status in elimination_based_connectors.labels_with_status {
            let (connector, merchant_connector_id) = labels_with_status.label
                .split_once(':')
                .ok_or(errors::RoutingError::InvalidEliminationBasedConnectorLabel(labels_with_status.label.to_string()))
                .attach_printable(
                    "unable to split connector_name and mca_id from the label obtained by the elimination based dynamic routing service",
                )?;

            let routable_connector = api_routing::RoutableConnectorChoice {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector: euclid::enums::RoutableConnectors::from_str(connector)
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "RoutableConnectors".to_string(),
                    })
                    .attach_printable("unable to convert String to RoutableConnectors")?,
                merchant_connector_id: Some(
                    common_utils::id_type::MerchantConnectorAccountId::wrap(
                        merchant_connector_id.to_string(),
                    )
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "MerchantConnectorAccountId".to_string(),
                    })
                    .attach_printable("unable to convert MerchantConnectorAccountId from string")?,
                ),
            };

            if labels_with_status
                .elimination_information
                .is_some_and(|elimination_info| {
                    elimination_info
                        .entity
                        .is_some_and(|entity_info| entity_info.is_eliminated)
                })
            {
                eliminated_connectors.push(routable_connector);
            } else {
                non_eliminated_connectors.push(routable_connector);
            }
            connectors.extend(non_eliminated_connectors.clone());
            connectors.extend(eliminated_connectors.clone());
        }
        logger::debug!(dynamic_eliminated_connectors=?eliminated_connectors);
        logger::debug!(dynamic_elimination_based_routing_connectors=?connectors);

        routing_event.set_status_code(200);
        routing_event.set_routable_connectors(connectors.clone());
        state.event_handler().log_event(&routing_event);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
#[allow(clippy::too_many_arguments)]
pub async fn perform_contract_based_routing<F, D>(
    state: &SessionState,
    routable_connectors: Vec<api_routing::RoutableConnectorChoice>,
    profile_id: &common_utils::id_type::ProfileId,
    merchant_id: &common_utils::id_type::MerchantId,
    payment_id: &common_utils::id_type::PaymentId,
    _dynamic_routing_config_params_interpolator: routing::helpers::DynamicRoutingConfigParamsInterpolator,
    contract_based_algo_ref: api_routing::ContractRoutingAlgorithm,
    payment_data: &mut D,
) -> RoutingResult<Vec<api_routing::RoutableConnectorChoice>>
where
    F: Send + Clone,
    D: OperationSessionGetters<F> + OperationSessionSetters<F> + Send + Sync + Clone,
{
    if contract_based_algo_ref.enabled_feature
        == api_routing::DynamicRoutingFeatures::DynamicConnectorSelection
    {
        logger::debug!(
            "performing contract_based_routing for profile {}",
            profile_id.get_string_repr()
        );
        let client = &state
            .grpc_client
            .dynamic_routing
            .as_ref()
            .ok_or(errors::RoutingError::ContractRoutingClientInitializationError)
            .attach_printable("dynamic routing gRPC client not found")?
            .contract_based_client;

        let contract_based_routing_configs = routing::helpers::fetch_dynamic_routing_configs::<
            api_routing::ContractBasedRoutingConfig,
        >(
            state,
            profile_id,
            contract_based_algo_ref
                .algorithm_id_with_timestamp
                .algorithm_id
                .ok_or(errors::RoutingError::GenericNotFoundError {
                    field: "contract_based_routing_algorithm_id".to_string(),
                })
                .attach_printable("contract_based_routing_algorithm_id not found in profile_id")?,
        )
        .await
        .change_context(errors::RoutingError::ContractBasedRoutingConfigError)
        .attach_printable("unable to fetch contract based dynamic routing configs")?;

        let label_info = contract_based_routing_configs
            .label_info
            .clone()
            .ok_or(errors::RoutingError::ContractBasedRoutingConfigError)
            .attach_printable("Label information not found in contract routing configs")?;

        let contract_based_connectors = routable_connectors
            .clone()
            .into_iter()
            .filter(|conn| {
                label_info
                    .iter()
                    .any(|info| Some(info.mca_id.clone()) == conn.merchant_connector_id.clone())
            })
            .collect::<Vec<_>>();

        let mut other_connectors = routable_connectors
            .into_iter()
            .filter(|conn| {
                label_info
                    .iter()
                    .all(|info| Some(info.mca_id.clone()) != conn.merchant_connector_id.clone())
            })
            .collect::<Vec<_>>();

        let event_request = utils::CalContractScoreEventRequest {
            id: profile_id.get_string_repr().to_string(),
            params: "".to_string(),
            labels: contract_based_connectors
                .iter()
                .map(|conn_choice| conn_choice.to_string())
                .collect::<Vec<_>>(),
            config: Some(contract_based_routing_configs.clone()),
        };

        let routing_events_wrapper = utils::RoutingEventsWrapper::new(
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
            payment_id.get_string_repr().to_string(),
            profile_id.to_owned(),
            merchant_id.to_owned(),
            "IntelligentRouter: PerformContractRouting".to_string(),
            Some(event_request.clone()),
            true,
            false,
        );

        let closure = || async {
            let contract_based_connectors_result = client
                .calculate_contract_score(
                    profile_id.get_string_repr().into(),
                    contract_based_routing_configs.clone(),
                    "".to_string(),
                    contract_based_connectors,
                    state.get_grpc_headers(),
                )
                .await
                .attach_printable(
                    "unable to calculate/fetch contract score from dynamic routing service",
                );

            let contract_based_connectors = match contract_based_connectors_result {
                Ok(resp) => Some(utils::CalContractScoreEventResponse::from(&resp)),
                Err(err) => match err.current_context() {
                    DynamicRoutingError::ContractNotFound => {
                        client
                            .update_contracts(
                                profile_id.get_string_repr().into(),
                                label_info,
                                "".to_string(),
                                vec![],
                                u64::default(),
                                state.get_grpc_headers(),
                            )
                            .await
                            .change_context(errors::RoutingError::ContractScoreUpdationError)
                            .attach_printable(
                                "unable to update contract based routing window in dynamic routing service",
                            )?;
                        return Err((errors::RoutingError::ContractScoreCalculationError {
                            err: err.to_string(),
                        })
                        .into());
                    }
                    _ => {
                        return Err((errors::RoutingError::ContractScoreCalculationError {
                            err: err.to_string(),
                        })
                        .into())
                    }
                },
            };

            Ok(contract_based_connectors)
        };

        let events_response = routing_events_wrapper
            .construct_event_builder(
                "ContractScoreCalculator.FetchContractScore".to_string(),
                RoutingEngine::IntelligentRouter,
                ApiMethod::Grpc,
            )?
            .trigger_event(state, closure)
            .await?;

        let contract_based_connectors: utils::CalContractScoreEventResponse = events_response
            .response
            .ok_or(errors::RoutingError::ContractScoreCalculationError {
                err: "CalContractScoreEventResponse not found".to_string(),
            })?;

        let mut routing_event = events_response
            .event
            .ok_or(errors::RoutingError::RoutingEventsError {
            message:
                "ContractRouting-Intelligent-Router: RoutingEvent not found in RoutingEventsResponse"
                    .to_string(),
            status_code: 500,
        })?;

        payment_data.set_routing_approach_in_attempt(Some(
            common_enums::RoutingApproach::ContractBasedRouting,
        ));

        let mut connectors = Vec::with_capacity(contract_based_connectors.labels_with_score.len());

        for label_with_score in contract_based_connectors.labels_with_score {
            let (connector, merchant_connector_id) = label_with_score.label
                .split_once(':')
                .ok_or(errors::RoutingError::InvalidContractBasedConnectorLabel(label_with_score.label.to_string()))
                .attach_printable(
                    "unable to split connector_name and mca_id from the label obtained by the dynamic routing service",
                )?;

            connectors.push(api_routing::RoutableConnectorChoice {
                choice_kind: api_routing::RoutableChoiceKind::FullStruct,
                connector: euclid::enums::RoutableConnectors::from_str(connector)
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "RoutableConnectors".to_string(),
                    })
                    .attach_printable("unable to convert String to RoutableConnectors")?,
                merchant_connector_id: Some(
                    common_utils::id_type::MerchantConnectorAccountId::wrap(
                        merchant_connector_id.to_string(),
                    )
                    .change_context(errors::RoutingError::GenericConversionError {
                        from: "String".to_string(),
                        to: "MerchantConnectorAccountId".to_string(),
                    })
                    .attach_printable("unable to convert MerchantConnectorAccountId from string")?,
                ),
            });
        }

        connectors.append(&mut other_connectors);

        logger::debug!(contract_based_routing_connectors=?connectors);

        routing_event.set_status_code(200);
        routing_event.set_routable_connectors(connectors.clone());
        routing_event.set_routing_approach(api_routing::RoutingApproach::ContractBased.to_string());
        state.event_handler().log_event(&routing_event);
        Ok(connectors)
    } else {
        Ok(routable_connectors)
    }
}

pub async fn get_active_merchant_connector_accounts(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    profile_id: &common_utils::id_type::ProfileId,
) -> RoutingResult<domain::MerchantConnectorAccountsWithoutEncrypted> {
    state
        .store
        .list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
            &key_store.merchant_id,
            profile_id,
        )
        .await
        .change_context(errors::RoutingError::MerchantConnectorAccountsFetchFailed)
}

/// Fetches the set of active MCA ids for session routing, degrading to an empty set on
/// failure, with an explicit log. Note the empty set does not yield fallback-config
/// tokens: with a warm cgraph cache every MCA-carrying choice is filtered out (no
/// session tokens), and with a cold cache the refresh's own DB fetch can still
/// hard-error.
pub async fn get_active_mca_ids_for_session(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    profile_id: &common_utils::id_type::ProfileId,
) -> std::collections::HashSet<common_utils::id_type::MerchantConnectorAccountId> {
    match get_active_merchant_connector_accounts(state, key_store, profile_id).await {
        Ok(merchant_connector_accounts) => merchant_connector_accounts.get_ids(),
        Err(err) => {
            logger::error!(
                error = ?err,
                "euclid_routing: failed to fetch active merchant connector accounts for \
                 session routing; continuing with empty active set"
            );
            std::collections::HashSet::new()
        }
    }
}
