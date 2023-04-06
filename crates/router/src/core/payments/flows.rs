pub mod authorize_flow;
pub mod cancel_flow;
pub mod capture_flow;
pub mod complete_authorize_flow;
pub mod psync_flow;
pub mod session_flow;
pub mod verfiy_flow;

use async_trait::async_trait;

use crate::{
    connector,
    core::{
        errors::{ConnectorError, CustomResult, RouterResult},
        payments,
    },
    routes::AppState,
    services,
    types::{self, api, storage},
};

#[async_trait]
pub trait ConstructFlowSpecificData<F, Req, Res> {
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;
}

#[async_trait]
pub trait Feature<F, T> {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<storage::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &storage::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;
}

macro_rules! default_imp_for_complete_authorize{
    ($($path:ident::$connector:ident),*)=> {
        $(
            impl api::PaymentsCompleteAuthorize for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::CompleteAuthorize,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_complete_authorize!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Applepay,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bluesnap,
    connector::Braintree,
	connector::Cashtocode,
    connector::Checkout,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Globalpay,
    connector::Klarna,
    connector::Multisafepay,
    connector::Payu,
    connector::Rapyd,
    connector::Shift4,
    connector::Stripe,
    connector::Trustpay,
    connector::Worldline,
    connector::Worldpay
);

macro_rules! default_imp_for_connector_redirect_response{
    ($($path:ident::$connector:ident),*)=> {
        $(
            impl services::ConnectorRedirectResponse for $path::$connector {
                fn get_flow_type(
                    &self,
                    _query_params: &str,
                    _json_payload: Option<serde_json::Value>,
                    _action: services::PaymentAction
                ) -> CustomResult<payments::CallConnectorAction, ConnectorError> {
                    Ok(payments::CallConnectorAction::Trigger)
                }
            }
    )*
    };
}

default_imp_for_connector_redirect_response!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Applepay,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bluesnap,
    connector::Braintree,
	connector::Cashtocode,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Globalpay,
    connector::Klarna,
    connector::Multisafepay,
    connector::Payu,
    connector::Rapyd,
    connector::Shift4,
    connector::Worldline,
    connector::Worldpay
);
