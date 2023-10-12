pub mod address;
pub mod api_keys;
pub mod business_profile;
pub mod capture;
pub mod cards_info;
pub mod configs;
pub mod connector_response;
pub mod customers;
pub mod dispute;
pub mod enums;
pub mod ephemeral_key;
pub mod events;
pub mod file;
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
pub use diesel_models::{ProcessTracker, ProcessTrackerNew, ProcessTrackerUpdate};
pub use scheduler::db::process_tracker;
pub mod reverse_lookup;

pub mod payout_attempt;
pub mod payouts;
mod query;
pub mod refund;

pub use data_models::payments::{
    payment_attempt::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
    payment_intent::{PaymentIntent, PaymentIntentNew, PaymentIntentUpdate},
};

pub use self::{
    address::*, api_keys::*, capture::*, cards_info::*, configs::*, connector_response::*,
    customers::*, dispute::*, ephemeral_key::*, events::*, file::*, locker_mock_up::*, mandate::*,
    merchant_account::*, merchant_connector_account::*, merchant_key_store::*, payment_link::*,
    payment_method::*, payout_attempt::*, payouts::*, process_tracker::*, refund::*,
    reverse_lookup::*,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingData {
    pub routed_through: Option<String>,
    pub algorithm: Option<api_models::admin::StraightThroughAlgorithm>,
}
