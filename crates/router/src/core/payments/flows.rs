pub mod approve_flow;
pub mod authorize_flow;
pub mod cancel_flow;
pub mod capture_flow;
pub mod complete_authorize_flow;
pub mod incremental_authorization_flow;
pub mod psync_flow;
pub mod reject_flow;
pub mod session_flow;
pub mod setup_mandate_flow;

use async_trait::async_trait;

#[cfg(feature = "frm")]
use crate::types::fraud_check as frm_types;
use crate::{
    connector,
    core::{
        errors::{ConnectorError, CustomResult, RouterResult},
        payments::{self, helpers},
    },
    routes::SessionState,
    services,
    types::{self, api, domain, storage},
};

#[async_trait]
pub trait ConstructFlowSpecificData<F, Req, Res> {
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
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
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &storage::business_profile::BusinessProfile,
    ) -> RouterResult<Self>
    where
        Self: Sized,
        F: Clone,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<types::AddAccessTokenResult>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>;

    async fn add_session_token<'a>(
        self,
        _state: &SessionState,
        _connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        F: Clone,
        Self: Sized,
        dyn api::Connector: services::ConnectorIntegration<F, T, types::PaymentsResponseData>,
    {
        Ok(self)
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        _state: &SessionState,
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
        _state: &SessionState,
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
        _state: &SessionState,
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
        _state: &SessionState,
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Billwerk,
    connector::Bitpay,
    connector::Boku,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payone,
    connector::Payu,
    connector::Placetopay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Bitpay,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Boku,
    connector::Cashtocode,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payone,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Zsl,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Threedsecureio,
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Cybersource,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Opennode,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Cybersource,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Opennode,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Cybersource,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Globepay,
    connector::Forte,
    connector::Globalpay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Opennode,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Coinbase,
    connector::Cryptopay,
    connector::Dlocal,
    connector::Ebanx,
    connector::Iatapay,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gpayments,
    connector::Helcim,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payone,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Shift4,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Dlocal,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_recipient_account {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutRecipientAccount for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::PoRecipientAccount,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutRecipientAccount for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::PoRecipientAccount,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_recipient_account!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
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
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_fraud_check {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheck for $path::$connector {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::FraudCheck for connector::DummyConnector<T> {}

default_imp_for_fraud_check!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_sale {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckSale for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Sale,
            frm_types::FraudCheckSaleData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckSale for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Sale,
        frm_types::FraudCheckSaleData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "frm")]
default_imp_for_frm_sale!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_checkout {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckCheckout for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Checkout,
            frm_types::FraudCheckCheckoutData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckCheckout for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Checkout,
        frm_types::FraudCheckCheckoutData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "frm")]
default_imp_for_frm_checkout!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_transaction {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckTransaction for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Transaction,
            frm_types::FraudCheckTransactionData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckTransaction for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Transaction,
        frm_types::FraudCheckTransactionData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "frm")]
default_imp_for_frm_transaction!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_fulfillment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckFulfillment for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Fulfillment,
            frm_types::FraudCheckFulfillmentData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckFulfillment for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Fulfillment,
        frm_types::FraudCheckFulfillmentData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "frm")]
default_imp_for_frm_fulfillment!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_record_return {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckRecordReturn for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::RecordReturn,
            frm_types::FraudCheckRecordReturnData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckRecordReturn for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegration<
        api::RecordReturn,
        frm_types::FraudCheckRecordReturnData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}

#[cfg(feature = "frm")]
default_imp_for_frm_record_return!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
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
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_incremental_authorization {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PaymentIncrementalAuthorization for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::IncrementalAuthorization,
            types::PaymentsIncrementalAuthorizationData,
            types::PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentIncrementalAuthorization for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::IncrementalAuthorization,
        types::PaymentsIncrementalAuthorizationData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_incremental_authorization!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_revoking_mandates {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::ConnectorMandateRevoke for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::MandateRevoke,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorMandateRevoke for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::MandateRevoke,
        types::MandateRevokeRequestData,
        types::MandateRevokeResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_revoking_mandates!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_connector_authentication {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::ExternalAuthentication for $path::$connector {}
            impl api::ConnectorAuthentication for $path::$connector {}
            impl api::ConnectorPreAuthentication for $path::$connector {}
            impl api::ConnectorPreAuthenticationVersionCall for $path::$connector {}
            impl api::ConnectorPostAuthentication for $path::$connector {}
            impl
            services::ConnectorIntegration<
            api::Authentication,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegration<
            api::PreAuthentication,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegration<
            api::PreAuthenticationVersionCall,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegration<
            api::PostAuthentication,
            types::authentication::ConnectorPostAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ExternalAuthentication for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorPreAuthentication for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorPreAuthenticationVersionCall for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorAuthentication for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorPostAuthentication for connector::DummyConnector<T> {}

#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::Authentication,
        types::authentication::ConnectorAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::PreAuthentication,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::PreAuthenticationVersionCall,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::PostAuthentication,
        types::authentication::ConnectorPostAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_connector_authentication!(
    connector::Adyenplatform,
    connector::Aci,
    connector::Adyen,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Cybersource,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
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
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
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
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_authorize_session_token {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::PaymentAuthorizeSessionToken for $path::$connector {}
            impl
            services::ConnectorIntegration<
                api::AuthorizeSessionToken,
                types::AuthorizeSessionTokenData,
                types::PaymentsResponseData
        > for $path::$connector
        {}
    )*
    };
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentAuthorizeSessionToken for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegration<
        api::AuthorizeSessionToken,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_authorize_session_token!(
    connector::Aci,
    connector::Adyen,
    connector::Adyenplatform,
    connector::Airwallex,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Cybersource,
    connector::Dlocal,
    connector::Ebanx,
    connector::Fiserv,
    connector::Forte,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Riskified,
    connector::Signifyd,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);
