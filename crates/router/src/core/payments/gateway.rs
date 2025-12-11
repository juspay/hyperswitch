pub mod access_token_gateway;
pub mod authenticate_gateway;
pub mod authorize_gateway;
pub mod context;
pub mod create_customer_gateway;
pub mod create_order_gateway;
pub mod payment_method_token_create_gateway;
pub mod post_authenticate_gateway;
pub mod pre_authenticate_gateway;
pub mod psync_gateway;
pub mod session_token_gateway;
pub mod setup_mandate;
use std::sync;

use hyperswitch_domain_models::router_flow_types::payments;

pub static GRANULAR_GATEWAY_SUPPORTED_FLOWS: sync::LazyLock<Vec<&'static str>> =
    sync::LazyLock::new(|| {
        vec![
            std::any::type_name::<payments::PSync>(),
            std::any::type_name::<payments::Authorize>(),
            std::any::type_name::<payments::SetupMandate>(),
        ]
    });
