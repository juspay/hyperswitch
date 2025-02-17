use hyperswitch_domain_models::router_flow_types::{
    Authenticate, PostAuthenticate, PreAuthenticate,
};
use hyperswitch_interfaces::api::{
    UasAuthenticationV2, UasPostAuthenticationV2, UasPreAuthenticationV2,
    UnifiedAuthenticationServiceV2,
};

#[cfg(feature = "frm")]
use crate::types::fraud_check as frm_types;
use crate::{
    connector, services,
    types::{self, api},
};

#[cfg(feature = "dummy_connector")]
mod dummy_connector_default_impl {
    #[cfg(feature = "frm")]
    use super::frm_types;
    use super::{api, connector, services, types};
    impl<const T: u8> api::PaymentV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentAuthorizeV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentAuthorizeSessionTokenV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentSyncV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentVoidV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentApproveV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentRejectV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentCaptureV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentSessionV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::MandateSetupV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentIncrementalAuthorizationV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentsCompleteAuthorizeV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentTokenV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::ConnectorCustomerV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentsPreProcessingV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentsPostProcessingV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::TaxCalculationV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentSessionUpdateV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::PaymentPostSessionTokensV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Authorize,
            types::PaymentFlowData,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PSync,
            types::PaymentFlowData,
            types::PaymentsSyncData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Void,
            types::PaymentFlowData,
            types::PaymentsCancelData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Approve,
            types::PaymentFlowData,
            types::PaymentsApproveData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Reject,
            types::PaymentFlowData,
            types::PaymentsRejectData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Capture,
            types::PaymentFlowData,
            types::PaymentsCaptureData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Session,
            types::PaymentFlowData,
            types::PaymentsSessionData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::SetupMandate,
            types::PaymentFlowData,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::IncrementalAuthorization,
            types::PaymentFlowData,
            types::PaymentsIncrementalAuthorizationData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::CompleteAuthorize,
            types::PaymentFlowData,
            types::CompleteAuthorizeData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PaymentMethodToken,
            types::PaymentFlowData,
            types::PaymentMethodTokenizationData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::CreateConnectorCustomer,
            types::PaymentFlowData,
            types::ConnectorCustomerData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PreProcessing,
            types::PaymentFlowData,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PostProcessing,
            types::PaymentFlowData,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::CalculateTax,
            types::PaymentFlowData,
            types::PaymentsTaxCalculationData,
            types::TaxCalculationResponseData,
        > for connector::DummyConnector<T>
    {
    }
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::SdkSessionUpdate,
            types::PaymentFlowData,
            types::SdkPaymentsSessionUpdateData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PostSessionTokens,
            types::PaymentFlowData,
            types::PaymentsPostSessionTokensData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::AuthorizeSessionToken,
            types::PaymentFlowData,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        > for connector::DummyConnector<T>
    {
    }
    impl<const T: u8> api::RefundV2 for connector::DummyConnector<T> {}
    impl<const T: u8> api::RefundExecuteV2 for connector::DummyConnector<T> {}
    impl<const T: u8> api::RefundSyncV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Execute,
            types::RefundFlowData,
            types::RefundsData,
            types::RefundsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::RSync,
            types::RefundFlowData,
            types::RefundsData,
            types::RefundsResponseData,
        > for connector::DummyConnector<T>
    {
    }
    impl<const T: u8> api::ConnectorAccessTokenV2 for connector::DummyConnector<T> {}
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::AccessTokenAuth,
            types::AccessTokenFlowData,
            types::AccessTokenRequestData,
            types::AccessToken,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::DisputeV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::AcceptDisputeV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Accept,
            types::DisputesFlowData,
            types::AcceptDisputeRequestData,
            types::AcceptDisputeResponse,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::DefendDisputeV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Defend,
            types::DisputesFlowData,
            types::DefendDisputeRequestData,
            types::DefendDisputeResponse,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::SubmitEvidenceV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Evidence,
            types::DisputesFlowData,
            types::SubmitEvidenceRequestData,
            types::SubmitEvidenceResponse,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::FileUploadV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::UploadFileV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Upload,
            types::FilesFlowData,
            types::UploadFileRequestData,
            types::UploadFileResponse,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::RetrieveFileV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Retrieve,
            types::FilesFlowData,
            types::RetrieveFileRequestData,
            types::RetrieveFileResponse,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::PayoutsV2 for connector::DummyConnector<T> {}

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutCreateV2 for connector::DummyConnector<T> {}

    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoCreate,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutEligibilityV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoEligibility,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutFulfillV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoFulfill,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutCancelV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoCancel,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutQuoteV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoQuote,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutRecipientV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoRecipient,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutSyncV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoSync,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "payouts")]
    impl<const T: u8> api::PayoutRecipientAccountV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "payouts")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PoRecipientAccount,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::ConnectorVerifyWebhookSourceV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::VerifyWebhookSource,
            types::WebhookSourceVerifyData,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::FraudCheckV2 for connector::DummyConnector<T> {}

    #[cfg(feature = "frm")]
    impl<const T: u8> api::FraudCheckSaleV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "frm")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Sale,
            types::FrmFlowData,
            frm_types::FraudCheckSaleData,
            frm_types::FraudCheckResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "frm")]
    impl<const T: u8> api::FraudCheckCheckoutV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "frm")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Checkout,
            types::FrmFlowData,
            frm_types::FraudCheckCheckoutData,
            frm_types::FraudCheckResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "frm")]
    impl<const T: u8> api::FraudCheckTransactionV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "frm")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Transaction,
            types::FrmFlowData,
            frm_types::FraudCheckTransactionData,
            frm_types::FraudCheckResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "frm")]
    impl<const T: u8> api::FraudCheckFulfillmentV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "frm")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Fulfillment,
            types::FrmFlowData,
            frm_types::FraudCheckFulfillmentData,
            frm_types::FraudCheckResponseData,
        > for connector::DummyConnector<T>
    {
    }

    #[cfg(feature = "frm")]
    impl<const T: u8> api::FraudCheckRecordReturnV2 for connector::DummyConnector<T> {}
    #[cfg(feature = "frm")]
    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::RecordReturn,
            types::FrmFlowData,
            frm_types::FraudCheckRecordReturnData,
            frm_types::FraudCheckResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::ConnectorMandateRevokeV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::MandateRevoke,
            types::MandateRevokeFlowData,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8> api::ExternalAuthenticationV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::ConnectorPreAuthenticationV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::ConnectorPreAuthenticationVersionCallV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::ConnectorAuthenticationV2 for connector::DummyConnector<T> {}

    impl<const T: u8> api::ConnectorPostAuthenticationV2 for connector::DummyConnector<T> {}

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::Authentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PreAuthentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PreAuthenticationVersionCall,
            types::ExternalAuthenticationFlowData,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for connector::DummyConnector<T>
    {
    }

    impl<const T: u8>
        services::ConnectorIntegrationV2<
            api::PostAuthentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::ConnectorPostAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for connector::DummyConnector<T>
    {
    }
}

macro_rules! default_imp_for_new_connector_integration_payment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PaymentV2 for $path::$connector{}
            impl api::PaymentAuthorizeV2 for $path::$connector{}
            impl api::PaymentAuthorizeSessionTokenV2 for $path::$connector{}
            impl api::PaymentSyncV2 for $path::$connector{}
            impl api::PaymentVoidV2 for $path::$connector{}
            impl api::PaymentApproveV2 for $path::$connector{}
            impl api::PaymentRejectV2 for $path::$connector{}
            impl api::PaymentCaptureV2 for $path::$connector{}
            impl api::PaymentSessionV2 for $path::$connector{}
            impl api::MandateSetupV2 for $path::$connector{}
            impl api::PaymentIncrementalAuthorizationV2 for $path::$connector{}
            impl api::PaymentsCompleteAuthorizeV2 for $path::$connector{}
            impl api::PaymentTokenV2 for $path::$connector{}
            impl api::ConnectorCustomerV2 for $path::$connector{}
            impl api::PaymentsPreProcessingV2 for $path::$connector{}
            impl api::PaymentsPostProcessingV2 for $path::$connector{}
            impl api::TaxCalculationV2 for $path::$connector{}
            impl api::PaymentSessionUpdateV2 for $path::$connector{}
            impl api::PaymentPostSessionTokensV2 for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Authorize,types::PaymentFlowData, types::PaymentsAuthorizeData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::PSync,types::PaymentFlowData, types::PaymentsSyncData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Void, types::PaymentFlowData, types::PaymentsCancelData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Approve,types::PaymentFlowData, types::PaymentsApproveData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Reject,types::PaymentFlowData, types::PaymentsRejectData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Capture,types::PaymentFlowData, types::PaymentsCaptureData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Session,types::PaymentFlowData, types::PaymentsSessionData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::SetupMandate,types::PaymentFlowData, types::SetupMandateRequestData, types::PaymentsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<
            api::IncrementalAuthorization,
                types::PaymentFlowData,
                types::PaymentsIncrementalAuthorizationData,
                types::PaymentsResponseData,
            >
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<
            api::CompleteAuthorize,
            types::PaymentFlowData,
                types::CompleteAuthorizeData,
                types::PaymentsResponseData,
            >            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<
            api::PaymentMethodToken,
            types::PaymentFlowData,
                types::PaymentMethodTokenizationData,
                types::PaymentsResponseData,
            > for   $path::$connector{}
            impl
            services::ConnectorIntegrationV2<
            api::CreateConnectorCustomer,
            types::PaymentFlowData,
                types::ConnectorCustomerData,
                types::PaymentsResponseData,
            > for $path::$connector{}
            impl services::ConnectorIntegrationV2<
            api::PreProcessing,
            types::PaymentFlowData,
                types::PaymentsPreProcessingData,
                types::PaymentsResponseData,
            > for $path::$connector{}
            impl services::ConnectorIntegrationV2<
            api::PostProcessing,
            types::PaymentFlowData,
                types::PaymentsPostProcessingData,
                types::PaymentsResponseData,
            > for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<
                api::AuthorizeSessionToken,
                types::PaymentFlowData,
                types::AuthorizeSessionTokenData,
                types::PaymentsResponseData
        > for $path::$connector{}
        impl services::ConnectorIntegrationV2<
            api::CalculateTax,
            types::PaymentFlowData,
                types::PaymentsTaxCalculationData,
                types::TaxCalculationResponseData,
            > for $path::$connector{}

            impl services::ConnectorIntegrationV2<
            api::SdkSessionUpdate,
            types::PaymentFlowData,
                types::SdkPaymentsSessionUpdateData,
                types::PaymentsResponseData,
            > for $path::$connector{}

            impl services::ConnectorIntegrationV2<
            api::PostSessionTokens,
            types::PaymentFlowData,
                types::PaymentsPostSessionTokensData,
                types::PaymentsResponseData,
                > for $path::$connector{}
    )*
    };
}

default_imp_for_new_connector_integration_payment!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wellsfargopayout,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_refund {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::RefundV2 for $path::$connector{}
            impl api::RefundExecuteV2 for $path::$connector{}
            impl api::RefundSyncV2 for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::Execute, types::RefundFlowData, types::RefundsData, types::RefundsResponseData>
            for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::RSync, types::RefundFlowData, types::RefundsData, types::RefundsResponseData>
            for $path::$connector{}
    )*
    };
}

default_imp_for_new_connector_integration_refund!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_connector_access_token {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorAccessTokenV2 for $path::$connector{}
            impl
            services::ConnectorIntegrationV2<api::AccessTokenAuth, types::AccessTokenFlowData, types::AccessTokenRequestData, types::AccessToken>
            for $path::$connector{}
    )*
    };
}

default_imp_for_new_connector_integration_connector_access_token!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_accept_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::DisputeV2 for $path::$connector {}
            impl api::AcceptDisputeV2 for $path::$connector {}
            impl
                services::ConnectorIntegrationV2<
                api::Accept,
                types::DisputesFlowData,
                types::AcceptDisputeRequestData,
                types::AcceptDisputeResponse,
            > for $path::$connector
            {}
    )*
    };
}

macro_rules! default_imp_for_new_connector_integration_submit_evidence {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::SubmitEvidenceV2 for $path::$connector {}
            impl
                services::ConnectorIntegrationV2<
                api::Evidence,
                types::DisputesFlowData,
                types::SubmitEvidenceRequestData,
                types::SubmitEvidenceResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_new_connector_integration_accept_dispute!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_defend_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::DefendDisputeV2 for $path::$connector {}
            impl
                services::ConnectorIntegrationV2<
                api::Defend,
                types::DisputesFlowData,
                types::DefendDisputeRequestData,
                types::DefendDisputeResponse,
            > for $path::$connector
            {}
        )*
    };
}
default_imp_for_new_connector_integration_defend_dispute!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);
default_imp_for_new_connector_integration_submit_evidence!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_file_upload {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FileUploadV2 for $path::$connector {}
            impl api::UploadFileV2 for $path::$connector {}
            impl
                services::ConnectorIntegrationV2<
                api::Upload,
                types::FilesFlowData,
                types::UploadFileRequestData,
                types::UploadFileResponse,
            > for $path::$connector
            {}
            impl api::RetrieveFileV2 for $path::$connector {}
            impl
                services::ConnectorIntegrationV2<
                api::Retrieve,
                types::FilesFlowData,
                types::RetrieveFileRequestData,
                types::RetrieveFileResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_new_connector_integration_file_upload!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_payouts {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutsV2 for $path::$connector {}
    )*
    };
}

default_imp_for_new_connector_integration_payouts!(
    connector::Aci,
    connector::Adyen,
    connector::Adyenplatform,
    connector::Airwallex,
    connector::Amazonpay,
    connector::Authorizedotnet,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Chargebee,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Cybersource,
    connector::Deutschebank,
    connector::Digitalvirgo,
    connector::Dlocal,
    connector::Ebanx,
    connector::Elavon,
    connector::Fiserv,
    connector::Fiservemea,
    connector::Fiuu,
    connector::Forte,
    connector::Getnet,
    connector::Globalpay,
    connector::Globepay,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Inespay,
    connector::Itaubank,
    connector::Jpmorgan,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Moneris,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nexixpay,
    connector::Nmi,
    connector::Nomupay,
    connector::Noon,
    connector::Novalnet,
    connector::Opayo,
    connector::Opennode,
    connector::Paybox,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Rapyd,
    connector::Razorpay,
    connector::Redsys,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Taxjar,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Thunes,
    connector::Tsys,
    connector::UnifiedAuthenticationService,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Plaid,
    connector::CtpMastercard
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_create {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutCreateV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_create!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_eligibility {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutEligibilityV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_eligibility!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_fulfill {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutFulfillV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_fulfill!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_cancel {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutCancelV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_cancel!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_quote {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutQuoteV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_quote!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_recipient {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutRecipientV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_recipient!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_sync {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutSyncV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::PoSync,
            types::PayoutFlowData,
            types::PayoutsData,
            types::PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_sync!(
    connector::Adyenplatform,
    connector::Adyen,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_recipient_account {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::PayoutRecipientAccountV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
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
default_imp_for_new_connector_integration_payouts_recipient_account!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_webhook_source_verification {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorVerifyWebhookSourceV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::VerifyWebhookSource,
            types::WebhookSourceVerifyData,
            types::VerifyWebhookSourceRequestData,
            types::VerifyWebhookSourceResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_new_connector_integration_webhook_source_verification!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_frm {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckV2 for $path::$connector {}
    )*
    };
}

default_imp_for_new_connector_integration_frm!(
    connector::Aci,
    connector::Adyen,
    connector::Adyenplatform,
    connector::Airwallex,
    connector::Amazonpay,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bamboraapac,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Chargebee,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Deutschebank,
    connector::Digitalvirgo,
    connector::Dlocal,
    connector::Ebanx,
    connector::Elavon,
    connector::Fiserv,
    connector::Fiservemea,
    connector::Forte,
    connector::Fiuu,
    connector::Getnet,
    connector::Globalpay,
    connector::Globepay,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Inespay,
    connector::Itaubank,
    connector::Jpmorgan,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Moneris,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nexixpay,
    connector::Nmi,
    connector::Nomupay,
    connector::Noon,
    connector::Novalnet,
    connector::Opayo,
    connector::Opennode,
    connector::Paybox,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Powertranz,
    connector::Rapyd,
    connector::Razorpay,
    connector::Redsys,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Taxjar,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Thunes,
    connector::Tsys,
    connector::UnifiedAuthenticationService,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Zen,
    connector::Plaid,
    connector::CtpMastercard
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_sale {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckSaleV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::Sale,
            types::FrmFlowData,
            frm_types::FraudCheckSaleData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_sale!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_checkout {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckCheckoutV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::Checkout,
            types::FrmFlowData,
            frm_types::FraudCheckCheckoutData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_checkout!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_transaction {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckTransactionV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::Transaction,
            types::FrmFlowData,
            frm_types::FraudCheckTransactionData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_transaction!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_fulfillment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckFulfillmentV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::Fulfillment,
            types::FrmFlowData,
            frm_types::FraudCheckFulfillmentData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_fulfillment!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_record_return {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::FraudCheckRecordReturnV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::RecordReturn,
            types::FrmFlowData,
            frm_types::FraudCheckRecordReturnData,
            frm_types::FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_record_return!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_revoking_mandates {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::ConnectorMandateRevokeV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::MandateRevoke,
            types::MandateRevokeFlowData,
            types::MandateRevokeRequestData,
            types::MandateRevokeResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_new_connector_integration_revoking_mandates!(
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Wise,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_connector_authentication {
    ($($path:ident::$connector:ident),*) => {
        $( impl api::ExternalAuthenticationV2 for $path::$connector {}
            impl api::ConnectorAuthenticationV2 for $path::$connector {}
            impl api::ConnectorPreAuthenticationV2 for $path::$connector {}
            impl api::ConnectorPreAuthenticationVersionCallV2 for $path::$connector {}
            impl api::ConnectorPostAuthenticationV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            api::Authentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::ConnectorAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationV2<
            api::PreAuthentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationV2<
            api::PreAuthenticationVersionCall,
            types::ExternalAuthenticationFlowData,
            types::authentication::PreAuthNRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationV2<
            api::PostAuthentication,
            types::ExternalAuthenticationFlowData,
            types::authentication::ConnectorPostAuthenticationRequestData,
            types::authentication::AuthenticationResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_new_connector_integration_connector_authentication!(
    connector::Aci,
    connector::Adyen,
    connector::Adyenplatform,
    connector::Authorizedotnet,
    connector::Bambora,
    connector::Bamboraapac,
    connector::Bankofamerica,
    connector::Billwerk,
    connector::Bitpay,
    connector::Bluesnap,
    connector::Boku,
    connector::Braintree,
    connector::Cashtocode,
    connector::Chargebee,
    connector::Checkout,
    connector::Cryptopay,
    connector::Coinbase,
    connector::Cybersource,
    connector::Datatrans,
    connector::Deutschebank,
    connector::Digitalvirgo,
    connector::Dlocal,
    connector::Ebanx,
    connector::Elavon,
    connector::Fiserv,
    connector::Fiservemea,
    connector::Forte,
    connector::Fiuu,
    connector::Getnet,
    connector::Globalpay,
    connector::Globepay,
    connector::Gocardless,
    connector::Gpayments,
    connector::Helcim,
    connector::Iatapay,
    connector::Inespay,
    connector::Itaubank,
    connector::Jpmorgan,
    connector::Klarna,
    connector::Mifinity,
    connector::Mollie,
    connector::Moneris,
    connector::Multisafepay,
    connector::Netcetera,
    connector::Nexinets,
    connector::Nexixpay,
    connector::Nmi,
    connector::Nomupay,
    connector::Noon,
    connector::Novalnet,
    connector::Nuvei,
    connector::Opayo,
    connector::Opennode,
    connector::Paybox,
    connector::Payeezy,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Payu,
    connector::Placetopay,
    connector::Powertranz,
    connector::Prophetpay,
    connector::Rapyd,
    connector::Razorpay,
    connector::Redsys,
    connector::Riskified,
    connector::Signifyd,
    connector::Square,
    connector::Stax,
    connector::Stripe,
    connector::Shift4,
    connector::Taxjar,
    connector::Trustpay,
    connector::Threedsecureio,
    connector::Thunes,
    connector::Tsys,
    connector::UnifiedAuthenticationService,
    connector::Volt,
    connector::Wise,
    connector::Worldline,
    connector::Worldpay,
    connector::Xendit,
    connector::Zen,
    connector::Zsl,
    connector::Plaid
);

macro_rules! default_imp_for_new_connector_integration_uas {
    ($($path:ident::$connector:ident),*) => {
        $( impl UnifiedAuthenticationServiceV2 for $path::$connector {}
            impl UasPreAuthenticationV2 for $path::$connector {}
            impl UasPostAuthenticationV2 for $path::$connector {}
            impl UasAuthenticationV2 for $path::$connector {}
            impl
            services::ConnectorIntegrationV2<
            PreAuthenticate,
            types::UasFlowData,
            types::UasPreAuthenticationRequestData,
            types::UasAuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationV2<
            PostAuthenticate,
            types::UasFlowData,
            types::UasPostAuthenticationRequestData,
            types::UasAuthenticationResponseData,
        > for $path::$connector
        {}
        impl
            services::ConnectorIntegrationV2<
            Authenticate,
            types::UasFlowData,
            types::UasAuthenticationRequestData,
            types::UasAuthenticationResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_new_connector_integration_uas!(
    connector::Adyenplatform,
    connector::Adyen,
    connector::Authorizedotnet,
    connector::Checkout,
    connector::Ebanx,
    connector::Gpayments,
    connector::Netcetera,
    connector::Nmi,
    connector::Noon,
    connector::Opayo,
    connector::Opennode,
    connector::Payme,
    connector::Payone,
    connector::Paypal,
    connector::Plaid,
    connector::Riskified,
    connector::Signifyd,
    connector::Stripe,
    connector::Threedsecureio,
    connector::Trustpay,
    connector::Wellsfargopayout,
    connector::Wise
);
