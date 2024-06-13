#[cfg(feature = "frm")]
use crate::types::fraud_check as frm_types;
use crate::{
    connector, services,
    types::{self, api},
};

macro_rules! default_imp_for_new_connector_integration_payment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PaymentNew for $path::$connector{}
            impl api::PaymentAuthorizeNew for $path::$connector{}
            impl api::PaymentAuthorizeSessionTokenNew for $path::$connector{}
            impl api::PaymentSyncNew for $path::$connector{}
            impl api::PaymentVoidNew for $path::$connector{}
            impl api::PaymentApproveNew for $path::$connector{}
            impl api::PaymentRejectNew for $path::$connector{}
            impl api::PaymentCaptureNew for $path::$connector{}
            impl api::PaymentSessionNew for $path::$connector{}
            impl api::MandateSetupNew for $path::$connector{}
            impl api::PaymentIncrementalAuthorizationNew for $path::$connector{}
            impl api::PaymentsCompleteAuthorizeNew for $path::$connector{}
            impl api::PaymentTokenNew for $path::$connector{}
            impl api::ConnectorCustomerNew for $path::$connector{}
            impl api::PaymentsPreProcessingNew for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Authorize,types::PaymentFlowData, types::PaymentsAuthorizeData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::PSync,types::PaymentFlowData, types::PaymentsSyncData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Void, types::PaymentFlowData, types::PaymentsCancelData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Approve,types::PaymentFlowData, types::PaymentsApproveData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Reject,types::PaymentFlowData, types::PaymentsRejectData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Capture,types::PaymentFlowData, types::PaymentsCaptureData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Session,types::PaymentFlowData, types::PaymentsSessionData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::SetupMandate,types::PaymentFlowData, types::SetupMandateRequestData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<
            api::IncrementalAuthorization,
                types::PaymentFlowData,
                types::PaymentsIncrementalAuthorizationData,
                types::PaymentsResponseData,
            >
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<
            api::CompleteAuthorize,
            types::PaymentFlowData,
                types::CompleteAuthorizeData,
                types::PaymentsResponseData,
            >            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<
            api::PaymentMethodToken,
            types::PaymentFlowData,
                types::PaymentMethodTokenizationData,
                types::PaymentsResponseData,
            > for   $path::$connector{}
            impl
            services::ConnectorIntegrationNew<
            api::CreateConnectorCustomer,
            types::PaymentFlowData,
                types::ConnectorCustomerData,
                types::PaymentsResponseData,
            > for $path::$connector{}
            impl services::ConnectorIntegrationNew<
            api::PreProcessing,
            types::PaymentFlowData,
                types::PaymentsPreProcessingData,
                types::PaymentsResponseData,
            > for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<
                api::AuthorizeSessionToken,
                types::PaymentFlowData,
                types::AuthorizeSessionTokenData,
                types::PaymentsResponseData
        > for $path::$connector{}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentAuthorizeNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentAuthorizeSessionTokenNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentSyncNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentVoidNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentApproveNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentRejectNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentCaptureNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentSessionNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::MandateSetupNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentIncrementalAuthorizationNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentsCompleteAuthorizeNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentTokenNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorCustomerNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PaymentsPreProcessingNew for connector::DummyConnector<T> {}

#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Authorize,
        types::PaymentFlowData,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PSync,
        types::PaymentFlowData,
        types::PaymentsSyncData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Void,
        types::PaymentFlowData,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Approve,
        types::PaymentFlowData,
        types::PaymentsApproveData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Reject,
        types::PaymentFlowData,
        types::PaymentsRejectData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Capture,
        types::PaymentFlowData,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Session,
        types::PaymentFlowData,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::SetupMandate,
        types::PaymentFlowData,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::IncrementalAuthorization,
        types::PaymentFlowData,
        types::PaymentsIncrementalAuthorizationData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::CompleteAuthorize,
        types::PaymentFlowData,
        types::CompleteAuthorizeData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PaymentMethodToken,
        types::PaymentFlowData,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::CreateConnectorCustomer,
        types::PaymentFlowData,
        types::ConnectorCustomerData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PreProcessing,
        types::PaymentFlowData,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::AuthorizeSessionToken,
        types::PaymentFlowData,
        types::AuthorizeSessionTokenData,
        types::PaymentsResponseData,
    > for connector::DummyConnector<T>
{
}

default_imp_for_new_connector_integration_payment!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_refund {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::RefundNew for $path::$connector{}
            impl api::RefundExecuteNew for $path::$connector{}
            impl api::RefundSyncNew for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::Execute, types::RefundFlowData, types::RefundsData, types::RefundsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::RSync, types::RefundFlowData, types::RefundsData, types::RefundsResponseData>
            for $path::$connector{}
    )*
    };
}

impl<const T: u8> api::RefundNew for connector::DummyConnector<T> {}
impl<const T: u8> api::RefundExecuteNew for connector::DummyConnector<T> {}
impl<const T: u8> api::RefundSyncNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Execute,
        types::RefundFlowData,
        types::RefundsData,
        types::RefundsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::RSync,
        types::RefundFlowData,
        types::RefundsData,
        types::RefundsResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_new_connector_integration_refund!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_connector_access_token {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorAccessTokenNew for $path::$connector{}
            impl
            services::ConnectorIntegrationNew<api::AccessTokenAuth, types::AccessTokenFlowData, types::AccessTokenRequestData, types::AccessToken>
            for $path::$connector{}
    )*
    };
}

impl<const T: u8> api::ConnectorAccessTokenNew for connector::DummyConnector<T> {}
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::AccessTokenAuth,
        types::AccessTokenFlowData,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for connector::DummyConnector<T>
{
}

default_imp_for_new_connector_integration_connector_access_token!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_accept_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::DisputeNew for $path::$connector {}
            impl api::AcceptDisputeNew for $path::$connector {}
            impl
                services::ConnectorIntegrationNew<
                api::Accept,
                types::DisputesFlowData,
                types::AcceptDisputeRequestData,
                types::AcceptDisputeResponse,
            > for $path::$connector
            {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::DisputeNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::AcceptDisputeNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Accept,
        types::DisputesFlowData,
        types::AcceptDisputeRequestData,
        types::AcceptDisputeResponse,
    > for connector::DummyConnector<T>
{
}
macro_rules! default_imp_for_new_connector_integration_defend_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::DefendDisputeNew for $path::$connector {}
            impl
                services::ConnectorIntegrationNew<
                api::Defend,
                types::DisputesFlowData,
                types::DefendDisputeRequestData,
                types::DefendDisputeResponse,
            > for $path::$connector
            {}
        )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::DefendDisputeNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Defend,
        types::DisputesFlowData,
        types::DefendDisputeRequestData,
        types::DefendDisputeResponse,
    > for connector::DummyConnector<T>
{
}

macro_rules! default_imp_for_new_connector_integration_submit_evidence {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::SubmitEvidenceNew for $path::$connector {}
            impl
                services::ConnectorIntegrationNew<
                api::Evidence,
                types::DisputesFlowData,
                types::SubmitEvidenceRequestData,
                types::SubmitEvidenceResponse,
            > for $path::$connector
            {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::SubmitEvidenceNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Evidence,
        types::DisputesFlowData,
        types::SubmitEvidenceRequestData,
        types::SubmitEvidenceResponse,
    > for connector::DummyConnector<T>
{
}

default_imp_for_new_connector_integration_accept_dispute!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);
default_imp_for_new_connector_integration_defend_dispute!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);
default_imp_for_new_connector_integration_submit_evidence!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_file_upload {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FileUploadNew for $path::$connector {}
            impl api::UploadFileNew for $path::$connector {}
            impl
                services::ConnectorIntegrationNew<
                api::Upload,
                types::FilesFlowData,
                types::UploadFileRequestData,
                types::UploadFileResponse,
            > for $path::$connector
            {}
            impl api::RetrieveFileNew for $path::$connector {}
            impl
                services::ConnectorIntegrationNew<
                api::Retrieve,
                types::FilesFlowData,
                types::RetrieveFileRequestData,
                types::RetrieveFileResponse,
            > for $path::$connector
            {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::FileUploadNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::UploadFileNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Upload,
        types::FilesFlowData,
        types::UploadFileRequestData,
        types::UploadFileResponse,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::RetrieveFileNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Retrieve,
        types::FilesFlowData,
        types::RetrieveFileRequestData,
        types::RetrieveFileResponse,
    > for connector::DummyConnector<T>
{
}

default_imp_for_new_connector_integration_file_upload!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_payouts {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutsNew for $path::$connector {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutsNew for connector::DummyConnector<T> {}
default_imp_for_new_connector_integration_payouts!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_create {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutCreateNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoCreate,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutCreateNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoCreate,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_create!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_eligibility {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutEligibilityNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoEligibility,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutEligibilityNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoEligibility,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_eligibility!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_fulfill {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutFulfillNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoFulfill,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutFulfillNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoFulfill,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_fulfill!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_cancel {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutCancelNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoCancel,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutCancelNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoCancel,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_cancel!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_quote {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutQuoteNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoQuote,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutQuoteNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoQuote,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_quote!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_recipient {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutRecipientNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoRecipient,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutRecipientNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoRecipient,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_recipient!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_recipient_account {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutRecipientAccountNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::PoRecipientAccount,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::PayoutRecipientAccountNew for connector::DummyConnector<T> {}
#[cfg(feature = "payouts")]
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PoRecipientAccount,
        types::PayoutFlowData,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_recipient_account!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);
#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_webhook_source_verification {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorVerifyWebhookSourceNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::VerifyWebhookSource,
            types::WebhookSourceVerifyData,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorVerifyWebhookSourceNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::VerifyWebhookSource,
        types::WebhookSourceVerifyData,
        types::VerifyWebhookSourceRequestData,
        types::VerifyWebhookSourceResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_webhook_source_verification!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckNew for $path::$connector {}
    )*
    };
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::FraudCheckNew for connector::DummyConnector<T> {}
#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_sale {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckSaleNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::Sale,
            types::FrmFlowData,
            frm_types::FraudCheckSaleData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckSaleNew for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Sale,
        types::FrmFlowData,
        frm_types::FraudCheckSaleData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_sale!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_checkout {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckCheckoutNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::Checkout,
            types::FrmFlowData,
            frm_types::FraudCheckCheckoutData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckCheckoutNew for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Checkout,
        types::FrmFlowData,
        frm_types::FraudCheckCheckoutData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_checkout!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_transaction {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckTransactionNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::Transaction,
            types::FrmFlowData,
            frm_types::FraudCheckTransactionData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckTransactionNew for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Transaction,
        types::FrmFlowData,
        frm_types::FraudCheckTransactionData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_transaction!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_fulfillment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckFulfillmentNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::Fulfillment,
            types::FrmFlowData,
            frm_types::FraudCheckFulfillmentData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckFulfillmentNew for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Fulfillment,
        types::FrmFlowData,
        frm_types::FraudCheckFulfillmentData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_fulfillment!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_record_return {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckRecordReturnNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::RecordReturn,
            types::FrmFlowData,
            frm_types::FraudCheckRecordReturnData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8> api::FraudCheckRecordReturnNew for connector::DummyConnector<T> {}
#[cfg(all(feature = "frm", feature = "dummy_connector"))]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::RecordReturn,
        types::FrmFlowData,
        frm_types::FraudCheckRecordReturnData,
        frm_types::FraudCheckResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_record_return!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_revoking_mandates {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::ConnectorMandateRevokeNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::MandateRevoke,
            types::MandateRevokeFlowData,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorMandateRevokeNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::MandateRevoke,
        types::MandateRevokeFlowData,
        types::MandateRevokeRequestData,
        types::MandateRevokeResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_new_connector_integration_revoking_mandates!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);

macro_rules! default_imp_for_new_connector_integration_connector_authentication {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::ExternalAuthenticationNew for $path::$connector {}
            impl api::ConnectorAuthenticationNew for $path::$connector {}
            impl api::ConnectorPreAuthenticationNew for $path::$connector {}
            impl api::ConnectorPreAuthenticationVersionCallNew for $path::$connector {}
            impl api::ConnectorPostAuthenticationNew for $path::$connector {}
            impl
            services::ConnectorIntegrationNew<
            api::Authentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationNew<
            api::PreAuthentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationNew<
            api::PreAuthenticationVersionCall,
            types::ExternalAuthenticationFlowData,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationNew<
            api::PostAuthentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::ConnectorPostAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ExternalAuthenticationNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorPreAuthenticationNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorPreAuthenticationVersionCallNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorAuthenticationNew for connector::DummyConnector<T> {}
#[cfg(feature = "dummy_connector")]
impl<const T: u8> api::ConnectorPostAuthenticationNew for connector::DummyConnector<T> {}

#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::Authentication,
        types::ExternalAuthenticationFlowData,
        types::authentication::ConnectorAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PreAuthentication,
        types::ExternalAuthenticationFlowData,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PreAuthenticationVersionCall,
        types::ExternalAuthenticationFlowData,
        types::authentication::PreAuthNRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
#[cfg(feature = "dummy_connector")]
impl<const T: u8>
    services::ConnectorIntegrationNew<
        api::PostAuthentication,
        types::ExternalAuthenticationFlowData,
        types::authentication::ConnectorPostAuthenticationRequestData,
        types::authentication::AuthenticationResponseData,
    > for connector::DummyConnector<T>
{
}
default_imp_for_new_connector_integration_connector_authentication!(
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
    connector::Threedsecureio,
    connector::Tsys,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Zsl
);
