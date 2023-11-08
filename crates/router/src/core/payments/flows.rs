pub mod approve_flow;
pub mod authorize_flow;
pub mod cancel_flow;
pub mod capture_flow;
pub mod complete_authorize_flow;
pub mod psync_flow;
pub mod reject_flow;
pub mod session_flow;
pub mod setup_mandate_flow;

use async_trait::async_trait;

use crate::{
    connector,
    core::{
        errors::{ConnectorError, CustomResult, RouterResult},
        payments::{self, helpers},
    },
    routes::AppState,
    services,
    types::{self, api, domain},
};

#[async_trait]
pub trait ConstructFlowSpecificData<F, Req, Res> {
    async fn construct_router_data<'a>(
        &self,
        state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<types::RouterData<F, Req, Res>>;
}

#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait Feature<F, T> {
    async fn decide_flows<'a>(
        self,
        state: &AppState,
        connector: &api::ConnectorData,
        maybe_customer: &Option<domain::Customer>,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
        connector_request: Option<services::Request>,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_access_token<'a>(
        &self,
        state: &AppState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_payment_method_token<'a>(
        &mut self,
        _state: &AppState,
        _connector: &api::ConnectorData,
        _tokenization_action: &payments::TokenizationAction,
    ) -> RouterResult<Option<String>>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(None)
    }

    async fn preprocessing_steps<'a>(
        self,
        _state: &AppState,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(self)
    }

    async fn create_connector_customer<'a>(
        &self,
        _state: &AppState,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Option<String>>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(None)
    }

    /// Returns the connector request and a bool which specifies whether to proceed with further
    async fn build_flow_specific_connector_request(
        &mut self,
        _state: &AppState,
        _connector: &api::ConnectorData,
        _call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        Ok((None, true))
    }
}

macro_rules! default_imp_for_complete_authorize {
    ($($path:ident::$connector:ident),*) => {
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

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentsCompleteAuthorize for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::CompleteAuthorize,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_complete_authorize!(
    connector::Aci,
    connector::Adyen,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Boku,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payu,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);
macro_rules! default_imp_for_webhook_source_verification {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorVerifyWebhookSource for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::VerifyWebhookSource,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorVerifyWebhookSource for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::VerifyWebhookSource,
        types::VerifyWebhookSourceRequestData,
        types::VerifyWebhookSourceResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_webhook_source_verification!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Braintree,
    connector::Boku,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_create_customer {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorCustomer for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::CreateConnectorCustomer,
            types::ConnectorCustomerData,
            types::PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorCustomer for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::CreateConnectorCustomer,
        types::ConnectorCustomerData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_create_customer!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Paypal,
    connector::Payme,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_connector_redirect_response {
    ($($path:ident::$connector:ident),*) => {
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

#[cfg(feature = "dummy_connector")]
impl<const T: u8> services::ConnectorRedirectResponse for connector::DummyConnector<T> {
    fn get_flow_type(
        &self,
        _query_params: &str,
        _json_payload: Option<serde_json::Value>,
        _action: services::PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, ConnectorError> {
        Ok(payments::CallConnectorAction::Trigger)
    }
}

default_imp_for_connector_redirect_response!(
    connector::Aci,
    connector::Adyen,
    connector::Bitpay,
    connector::Bankofamerica,
    connector::Boku,
    connector::Cashtocode,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay
);

macro_rules! default_imp_for_connector_request_id {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorTransactionId for $path::$connector {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorTransactionId for connector::DummyConnector<T> {}

default_imp_for_connector_request_id!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_accept_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::Dispute for $path::$connector {}
            impl api::AcceptDispute for $path::$connector {}
            impl
                services::ConnectorIntegration<
                api::Accept,
                types::AcceptDisputeRequestData,
                types::AcceptDisputeResponse,
            > for $path::$connector
            {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::Dispute for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::AcceptDispute for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Accept,
        types::AcceptDisputeRequestData,
        types::AcceptDisputeResponse,
    > for connector::DummyConnector<T>
{
}

default_imp_for_accept_dispute!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Paypal,
    connector::Payme,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_file_upload {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FileUpload for $path::$connector {}
            impl api::UploadFile for $path::$connector {}
            impl
                services::ConnectorIntegration<
                api::Upload,
                types::UploadFileRequestData,
                types::UploadFileResponse,
            > for $path::$connector
            {}
            impl api::RetrieveFile for $path::$connector {}
            impl
                services::ConnectorIntegration<
                api::Retrieve,
                types::RetrieveFileRequestData,
                types::RetrieveFileResponse,
            > for $path::$connector
            {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::FileUpload for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::UploadFile for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Upload,
        types::UploadFileRequestData,
        types::UploadFileResponse,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::RetrieveFile for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Retrieve,
        types::RetrieveFileRequestData,
        types::RetrieveFileResponse,
    > for connector::DummyConnector<T>
{
}

default_imp_for_file_upload!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Payeezy,
    connector::Paypal,
    connector::Payme,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Opennode,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_submit_evidence {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::SubmitEvidence for $path::$connector {}
            impl
                services::ConnectorIntegration<
                api::Evidence,
                types::SubmitEvidenceRequestData,
                types::SubmitEvidenceResponse,
            > for $path::$connector
            {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::SubmitEvidence for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Evidence,
        types::SubmitEvidenceRequestData,
        types::SubmitEvidenceResponse,
    > for connector::DummyConnector<T>
{
}

default_imp_for_submit_evidence!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Cybersource,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Payeezy,
    connector::Paypal,
    connector::Payme,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Opennode,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_defend_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::DefendDispute for $path::$connector {}
            impl
                services::ConnectorIntegration<
                api::Defend,
                types::DefendDisputeRequestData,
                types::DefendDisputeResponse,
            > for $path::$connector
            {}
        )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::DefendDispute for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Defend,
        types::DefendDisputeRequestData,
        types::DefendDisputeResponse,
    > for connector::DummyConnector<T>
{
}

default_imp_for_defend_dispute!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Cybersource,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Dlocal,
    connector::Fiserv,
    connector::Globepay,
    connector::Forte,
    connector::Globalpay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Payeezy,
    connector::Paypal,
    connector::Payme,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Opennode,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_pre_processing_steps{
    ($($path:ident::$connector:ident),*)=> {
        $(
            impl api::PaymentsPreProcessing for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentsPreProcessing for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_pre_processing_steps!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Iatapay,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Helcim,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Shift4,
    connector::Square,
    connector::Stax,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_payouts {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::Payouts for $path::$connector {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::Payouts for connector::DummyConnector<T> {}

default_imp_for_payouts!(
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_create {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutCreate for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoCreate,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutCreate for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<api::PoCreate, types::PayoutsData, types::PayoutsResponseData>
    for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_create!(
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_eligibility {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutEligibility for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoEligibility,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutEligibility for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::PoEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_eligibility!(
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_fulfill {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutFulfill for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoFulfill,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutFulfill for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_fulfill!(
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_cancel {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutCancel for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoCancel,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutCancel for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<api::PoCancel, types::PayoutsData, types::PayoutsResponseData>
    for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_cancel!(
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_quote {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutQuote for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoQuote,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutQuote for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<api::PoQuote, types::PayoutsData, types::PayoutsResponseData>
    for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_quote!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_recipient {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutRecipient for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoRecipient,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutRecipient for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<api::PoRecipient, types::PayoutsData, types::PayoutsResponseData>
    for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_recipient!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_approve {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PaymentApprove for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Approve,
            types::PaymentsApproveData,
            types::PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentApprove for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Approve,
        types::PaymentsApproveData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_approve!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);

macro_rules! default_imp_for_reject {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PaymentReject for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Reject,
            types::PaymentsRejectData,
            types::PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentReject for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Reject,
        types::PaymentsRejectData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_reject!(
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mollie,
    connector::Multisafepay,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen
);
