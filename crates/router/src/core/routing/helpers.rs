//! Analysis for usage of all helper functions for use case of routing
//!
//! Functions that are used to perform the retrieval of merchant's
//! routing dict, configs, defaults
use std::fmt::Debug;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use std::str::FromStr;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use std::sync::Arc;

#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use api_models::open_router;
use api_models::routing as routing_types;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use common_utils::ext_traits::ValueExt;
use common_utils::{ext_traits::Encode, id_type, types::keymanager::KeyManagerState};
use diesel_models::configs;
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use diesel_models::dynamic_routing_stats::{DynamicRoutingStatsNew, DynamicRoutingStatsUpdate};
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use diesel_models::routing_algorithm;
use error_stack::ResultExt;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use external_services::grpc_client::dynamic_routing::{
    contract_routing_client::ContractBasedDynamicRouting,
    elimination_based_client::EliminationBasedRouting,
    success_rate_client::SuccessBasedDynamicRouting,
};
#[cfg(all(feature = "v1", feature = "dynamic_routing"))]
use hyperswitch_domain_models::api::ApplicationResponse;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use router_env::logger;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use router_env::{instrument, tracing};
use rustc_hash::FxHashSet;
use storage_impl::redis::cache;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use storage_impl::redis::cache::Cacheable;

#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::db::errors::StorageErrorExt;
#[cfg(feature = "v2")]
use crate::types::domain::MerchantConnectorAccount;
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::types::transformers::ForeignFrom;
use crate::{
    core::errors::{self, RouterResult},
    db::StorageInterface,
    routes::SessionState,
    types::{domain, storage},
    utils::StringExt,
};
#[cfg(all(feature = "dynamic_routing", feature = "v1"))]
use crate::{
    core::{metrics as core_metrics},
    types::transformers::ForeignInto,
};
use hyperswitch_routing::payment_routing;
pub use hyperswitch_routing::helpers::*;
