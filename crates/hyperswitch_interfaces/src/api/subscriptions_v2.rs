use hyperswitch_domain_models::{
    router_data_v2::flow_common_types::CreateCustomerData,
    router_flow_types::payments::CreateConnectorCustomer,
    router_request_types::ConnectorCustomerData,
    router_response_types::PaymentsResponseData,
};

use crate::connector_integration_v2::ConnectorIntegrationV2;

/// trait SubscriptionsV2
pub trait SubscriptionsV2: CustomerCreateV2 {}

/// trait CustomersCreateV2
pub trait CustomerCreateV2:
    ConnectorIntegrationV2<
    CreateConnectorCustomer,
    CreateCustomerData,
    ConnectorCustomerData,
    PaymentsResponseData,
>
{
}
