use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::CreateCustomerData,
    router_flow_types::subscriptions::CreateCustomer,
    router_request_types::subscriptions::CreateCustomerRequest,
    router_response_types::subscriptions::CreateCustomerResponse,
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: CustomerCreateV2 {}

/// trait CustomersCreateV2
pub trait CustomerCreateV2:
    ConnectorIntegrationV2<
    CreateCustomer,
    CreateCustomerData,
    CreateCustomerRequest,
    CreateCustomerResponse,
>
{
}
