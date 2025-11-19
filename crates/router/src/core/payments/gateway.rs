pub mod context;
pub mod psync_gateway;
pub mod authorize_gateway;

use std::sync;

use hyperswitch_domain_models::router_flow_types::payments;

pub static GRANULAR_GATEWAY_SUPPORTED_FLOWS: sync::LazyLock<Vec<&'static str>> =
    sync::LazyLock::new(|| vec![std::any::type_name::<payments::PSync>(), std::any::type_name::<payments::Authorize>()]);
