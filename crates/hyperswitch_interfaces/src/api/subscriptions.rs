//! Customers Interface for V1
use hyperswitch_domain_models::{
    router_flow_types::subscriptions::CreateCustomer as CreateCustomerFlow,
    router_request_types::subscriptions::CreateCustomerRequest,
    router_response_types::subscriptions::CreateCustomerResponse,
};

use super::{ConnectorCommon, ConnectorIntegration};

#[cfg(feature = "v1")]
/// trait CustomerCreate for V1
pub trait CreateCustomer:
    ConnectorIntegration<CreateCustomerFlow, CreateCustomerRequest, CreateCustomerResponse>
{
}

#[cfg(feature = "v1")]
pub trait Subscriptions: ConnectorCommon + CreateCustomer {}

#[cfg(not(feature = "v1"))]
/// trait CreateCustomer (disabled when not V1)
pub trait CreateCustomer {}

#[cfg(not(feature = "v1"))]
pub trait Subscriptions {}
