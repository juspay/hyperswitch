pub mod address;
pub mod api_keys;
pub mod authentication;
pub mod authorization;
pub mod blocklist;
pub mod blocklist_fingerprint;
pub mod blocklist_lookup;
pub mod business_profile;
pub mod capture;
pub mod cards_info;
pub mod configs;
pub mod customers;
pub mod dashboard_metadata;
pub mod dispute;
pub mod enums;
pub mod ephemeral_key;
pub mod events;
pub mod file;
pub mod fraud_check;
pub mod generic_link;
pub mod gsm;
#[cfg(feature = "kv_store")]
pub mod kv;
pub mod locker_mock_up;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod payment_attempt;
pub mod payment_link;
pub mod payment_method;
pub mod payout_attempt;
pub mod payouts;
pub mod refund;
pub mod reverse_lookup;
pub mod role;
pub mod routing_algorithm;
pub mod user;
pub mod user_role;

use std::collections::HashMap;

pub use data_models::payments::{
    payment_attempt::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
    payment_intent::{PaymentIntentNew, PaymentIntentUpdate},
    PaymentIntent,
};
#[cfg(feature = "payouts")]
pub use data_models::payouts::{
    payout_attempt::{PayoutAttempt, PayoutAttemptNew, PayoutAttemptUpdate},
    payouts::{Payouts, PayoutsNew, PayoutsUpdate},
};
pub use diesel_models::{
    ProcessTracker, ProcessTrackerNew, ProcessTrackerRunner, ProcessTrackerUpdate,
};
pub use scheduler::db::process_tracker;

pub use self::{
    address::*, api_keys::*, authentication::*, authorization::*, blocklist::*,
    blocklist_fingerprint::*, blocklist_lookup::*, business_profile::*, capture::*, cards_info::*,
    configs::*, customers::*, dashboard_metadata::*, dispute::*, ephemeral_key::*, events::*,
    file::*, fraud_check::*, generic_link::*, gsm::*, locker_mock_up::*, mandate::*,
    merchant_account::*, merchant_connector_account::*, merchant_key_store::*, payment_link::*,
    payment_method::*, process_tracker::*, refund::*, reverse_lookup::*, role::*,
    routing_algorithm::*, user::*, user_role::*,
};
use crate::types::api::routing;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingData {
    pub routed_through: Option<String>,
    #[cfg(feature = "connector_choice_mca_id")]
    pub merchant_connector_id: Option<String>,
    #[cfg(not(feature = "connector_choice_mca_id"))]
    pub business_sub_label: Option<String>,
    pub routing_info: PaymentRoutingInfo,
    pub algorithm: Option<api_models::routing::StraightThroughAlgorithm>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "PaymentRoutingInfoSerde", into = "PaymentRoutingInfoSerde")]
pub struct PaymentRoutingInfo {
    pub algorithm: Option<routing::StraightThroughAlgorithm>,
    pub pre_routing_results:
        Option<HashMap<api_models::enums::PaymentMethodType, routing::RoutableConnectorChoice>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentRoutingInfoInner {
    pub algorithm: Option<routing::StraightThroughAlgorithm>,
    pub pre_routing_results:
        Option<HashMap<api_models::enums::PaymentMethodType, routing::RoutableConnectorChoice>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PaymentRoutingInfoSerde {
    OnlyAlgorithm(Box<routing::StraightThroughAlgorithm>),
    WithDetails(Box<PaymentRoutingInfoInner>),
}

impl From<PaymentRoutingInfoSerde> for PaymentRoutingInfo {
    fn from(value: PaymentRoutingInfoSerde) -> Self {
        match value {
            PaymentRoutingInfoSerde::OnlyAlgorithm(algo) => Self {
                algorithm: Some(*algo),
                pre_routing_results: None,
            },
            PaymentRoutingInfoSerde::WithDetails(details) => Self {
                algorithm: details.algorithm,
                pre_routing_results: details.pre_routing_results,
            },
        }
    }
}

impl From<PaymentRoutingInfo> for PaymentRoutingInfoSerde {
    fn from(value: PaymentRoutingInfo) -> Self {
        Self::WithDetails(Box::new(PaymentRoutingInfoInner {
            algorithm: value.algorithm,
            pre_routing_results: value.pre_routing_results,
        }))
    }
}
