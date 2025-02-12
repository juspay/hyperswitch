use hyperswitch_domain_models::{
    router_data::AccessToken,
    router_data_v2::{
        flow_common_types::{
            DisputesFlowData, MandateRevokeFlowData, PaymentFlowData, RefundFlowData,
            WebhookSourceVerifyData,
        },
        AccessTokenFlowData, FilesFlowData,
    },
    router_flow_types::{
        dispute::{Accept, Defend, Evidence},
        files::{Retrieve, Upload},
        mandate_revoke::MandateRevoke,
        payments::{
            Approve, Authorize, AuthorizeSessionToken, CalculateTax, Capture, CompleteAuthorize,
            CreateConnectorCustomer, IncrementalAuthorization, PSync, PaymentMethodToken,
            PostProcessing, PostSessionTokens, PreProcessing, Reject, SdkSessionUpdate, Session,
            SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        webhooks::VerifyWebhookSource,
        AccessTokenAuth,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenRequestData, AuthorizeSessionTokenData,
        CompleteAuthorizeData, ConnectorCustomerData, DefendDisputeRequestData,
        MandateRevokeRequestData, PaymentMethodTokenizationData, PaymentsApproveData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsIncrementalAuthorizationData, PaymentsPostProcessingData,
        PaymentsPostSessionTokensData, PaymentsPreProcessingData, PaymentsRejectData,
        PaymentsSessionData, PaymentsSyncData, PaymentsTaxCalculationData, RefundsData,
        RetrieveFileRequestData, SdkPaymentsSessionUpdateData, SetupMandateRequestData,
        SubmitEvidenceRequestData, UploadFileRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, MandateRevokeResponseData,
        PaymentsResponseData, RefundsResponseData, RetrieveFileResponse, SubmitEvidenceResponse,
        TaxCalculationResponseData, UploadFileResponse, VerifyWebhookSourceResponseData,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_data_v2::FrmFlowData,
    router_flow_types::fraud_check::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_data_v2::PayoutFlowData,
    router_flow_types::payouts::{
        PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
        PoSync,
    },
    router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::api::fraud_check_v2::{
    FraudCheckCheckoutV2, FraudCheckFulfillmentV2, FraudCheckRecordReturnV2, FraudCheckSaleV2,
    FraudCheckTransactionV2,
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::api::payouts_v2::{
    PayoutCancelV2, PayoutCreateV2, PayoutEligibilityV2, PayoutFulfillV2, PayoutQuoteV2,
    PayoutRecipientAccountV2, PayoutRecipientV2, PayoutSyncV2,
};
use hyperswitch_interfaces::{
    api::{
        disputes_v2::{AcceptDisputeV2, DefendDisputeV2, DisputeV2, SubmitEvidenceV2},
        files_v2::{FileUploadV2, RetrieveFileV2, UploadFileV2},
        payments_v2::{
            ConnectorCustomerV2, MandateSetupV2, PaymentApproveV2, PaymentAuthorizeSessionTokenV2,
            PaymentAuthorizeV2, PaymentCaptureV2, PaymentIncrementalAuthorizationV2,
            PaymentPostSessionTokensV2, PaymentRejectV2, PaymentSessionUpdateV2, PaymentSessionV2,
            PaymentSyncV2, PaymentTokenV2, PaymentV2, PaymentVoidV2, PaymentsCompleteAuthorizeV2,
            PaymentsPostProcessingV2, PaymentsPreProcessingV2, TaxCalculationV2,
        },
        refunds_v2::{RefundExecuteV2, RefundSyncV2, RefundV2},
        ConnectorAccessTokenV2, ConnectorMandateRevokeV2, ConnectorVerifyWebhookSourceV2,
    },
    connector_integration_v2::ConnectorIntegrationV2,
};

use crate::connectors;

macro_rules! default_imp_for_new_connector_integration_payment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentV2 for $path::$connector{}
            impl PaymentAuthorizeV2 for $path::$connector{}
            impl PaymentAuthorizeSessionTokenV2 for $path::$connector{}
            impl PaymentSyncV2 for $path::$connector{}
            impl PaymentVoidV2 for $path::$connector{}
            impl PaymentApproveV2 for $path::$connector{}
            impl PaymentRejectV2 for $path::$connector{}
            impl PaymentCaptureV2 for $path::$connector{}
            impl PaymentSessionV2 for $path::$connector{}
            impl MandateSetupV2 for $path::$connector{}
            impl PaymentIncrementalAuthorizationV2 for $path::$connector{}
            impl PaymentsCompleteAuthorizeV2 for $path::$connector{}
            impl PaymentTokenV2 for $path::$connector{}
            impl ConnectorCustomerV2 for $path::$connector{}
            impl PaymentsPreProcessingV2 for $path::$connector{}
            impl PaymentsPostProcessingV2 for $path::$connector{}
            impl TaxCalculationV2 for $path::$connector{}
            impl PaymentSessionUpdateV2 for $path::$connector{}
            impl PaymentPostSessionTokensV2 for $path::$connector{}
            impl
            ConnectorIntegrationV2<Authorize,PaymentFlowData, PaymentsAuthorizeData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<PSync,PaymentFlowData, PaymentsSyncData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<Void, PaymentFlowData, PaymentsCancelData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<Approve,PaymentFlowData, PaymentsApproveData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<Reject,PaymentFlowData, PaymentsRejectData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<Capture,PaymentFlowData, PaymentsCaptureData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<Session,PaymentFlowData, PaymentsSessionData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<SetupMandate,PaymentFlowData, SetupMandateRequestData, PaymentsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<
            IncrementalAuthorization,
                PaymentFlowData,
                PaymentsIncrementalAuthorizationData,
                PaymentsResponseData,
            >
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<
            CompleteAuthorize,
            PaymentFlowData,
                CompleteAuthorizeData,
                PaymentsResponseData,
            >            for $path::$connector{}
            impl
            ConnectorIntegrationV2<
            PaymentMethodToken,
            PaymentFlowData,
                PaymentMethodTokenizationData,
                PaymentsResponseData,
            > for   $path::$connector{}
            impl
            ConnectorIntegrationV2<
            CreateConnectorCustomer,
            PaymentFlowData,
                ConnectorCustomerData,
                PaymentsResponseData,
            > for $path::$connector{}
            impl ConnectorIntegrationV2<
            PreProcessing,
            PaymentFlowData,
                PaymentsPreProcessingData,
                PaymentsResponseData,
            > for $path::$connector{}
            impl ConnectorIntegrationV2<
            PostProcessing,
            PaymentFlowData,
                PaymentsPostProcessingData,
                PaymentsResponseData,
            > for $path::$connector{}
            impl
            ConnectorIntegrationV2<
                AuthorizeSessionToken,
                PaymentFlowData,
                AuthorizeSessionTokenData,
                PaymentsResponseData
        > for $path::$connector{}
        impl ConnectorIntegrationV2<
            CalculateTax,
            PaymentFlowData,
            PaymentsTaxCalculationData,
            TaxCalculationResponseData,
            > for $path::$connector{}
         impl ConnectorIntegrationV2<
            SdkSessionUpdate,
            PaymentFlowData,
            SdkPaymentsSessionUpdateData,
            PaymentsResponseData,
            > for $path::$connector{}
        impl
            ConnectorIntegrationV2<
            PostSessionTokens,
            PaymentFlowData,
            PaymentsPostSessionTokensData,
            PaymentsResponseData,
            > for $path::$connector{}
    )*
    };
}

default_imp_for_new_connector_integration_payment!(
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_refund {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl RefundV2 for $path::$connector{}
            impl RefundExecuteV2 for $path::$connector{}
            impl RefundSyncV2 for $path::$connector{}
            impl
            ConnectorIntegrationV2<Execute, RefundFlowData, RefundsData, RefundsResponseData>
            for $path::$connector{}
            impl
            ConnectorIntegrationV2<RSync, RefundFlowData, RefundsData, RefundsResponseData>
            for $path::$connector{}
    )*
    };
}

default_imp_for_new_connector_integration_refund!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Wellsfargo,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_connector_access_token {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl ConnectorAccessTokenV2 for $path::$connector{}
            impl
            ConnectorIntegrationV2<AccessTokenAuth, AccessTokenFlowData, AccessTokenRequestData, AccessToken>
            for $path::$connector{}
    )*
    };
}

default_imp_for_new_connector_integration_connector_access_token!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_accept_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl DisputeV2 for $path::$connector {}
            impl AcceptDisputeV2 for $path::$connector {}
            impl
                ConnectorIntegrationV2<
                Accept,
                DisputesFlowData,
                AcceptDisputeRequestData,
                AcceptDisputeResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_new_connector_integration_accept_dispute!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_submit_evidence {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl SubmitEvidenceV2 for $path::$connector {}
            impl
                ConnectorIntegrationV2<
                Evidence,
                DisputesFlowData,
                SubmitEvidenceRequestData,
                SubmitEvidenceResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_new_connector_integration_submit_evidence!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_defend_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl DefendDisputeV2 for $path::$connector {}
            impl
                ConnectorIntegrationV2<
                Defend,
                DisputesFlowData,
                DefendDisputeRequestData,
                DefendDisputeResponse,
            > for $path::$connector
            {}
        )*
    };
}

default_imp_for_new_connector_integration_defend_dispute!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_file_upload {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FileUploadV2 for $path::$connector {}
            impl UploadFileV2 for $path::$connector {}
            impl
                ConnectorIntegrationV2<
                Upload,
                FilesFlowData,
                UploadFileRequestData,
                UploadFileResponse,
            > for $path::$connector
            {}
            impl RetrieveFileV2 for $path::$connector {}
            impl
                ConnectorIntegrationV2<
                Retrieve,
                FilesFlowData,
                RetrieveFileRequestData,
                RetrieveFileResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_new_connector_integration_file_upload!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_create {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutCreateV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoCreate,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_create!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_eligibility {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutEligibilityV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoEligibility,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_eligibility!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_fulfill {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutFulfillV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoFulfill,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_fulfill!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_cancel {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutCancelV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoCancel,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_cancel!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_quote {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutQuoteV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoQuote,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_quote!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_recipient {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutRecipientV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoRecipient,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_recipient!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_sync {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutSyncV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoSync,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_sync!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_new_connector_integration_payouts_recipient_account {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutRecipientAccountV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            PoRecipientAccount,
            PayoutFlowData,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_new_connector_integration_payouts_recipient_account!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_webhook_source_verification {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl ConnectorVerifyWebhookSourceV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            VerifyWebhookSource,
            WebhookSourceVerifyData,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_new_connector_integration_webhook_source_verification!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_sale {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckSaleV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            Sale,
            FrmFlowData,
            FraudCheckSaleData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_sale!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_checkout {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckCheckoutV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            Checkout,
            FrmFlowData,
            FraudCheckCheckoutData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_checkout!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_transaction {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckTransactionV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            Transaction,
            FrmFlowData,
            FraudCheckTransactionData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_transaction!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_fulfillment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckFulfillmentV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            Fulfillment,
            FrmFlowData,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_fulfillment!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_new_connector_integration_frm_record_return {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckRecordReturnV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            RecordReturn,
            FrmFlowData,
            FraudCheckRecordReturnData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_new_connector_integration_frm_record_return!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_new_connector_integration_revoking_mandates {
    ($($path:ident::$connector:ident),*) => {
        $( impl ConnectorMandateRevokeV2 for $path::$connector {}
            impl
            ConnectorIntegrationV2<
            MandateRevoke,
            MandateRevokeFlowData,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_new_connector_integration_revoking_mandates!(
    connectors::Aci,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bamboraapac,
    connectors::Bankofamerica,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Bluesnap,
    connectors::Braintree,
    connectors::Boku,
    connectors::Cashtocode,
    connectors::Chargebee,
    connectors::Coinbase,
    connectors::Coingate,
    connectors::Cryptopay,
    connectors::CtpMastercard,
    connectors::Cybersource,
    connectors::Datatrans,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Getnet,
    connectors::Globalpay,
    connectors::Globepay,
    connectors::Gocardless,
    connectors::Helcim,
    connectors::Iatapay,
    connectors::Inespay,
    connectors::Itaubank,
    connectors::Jpmorgan,
    connectors::Klarna,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nuvei,
    connectors::Paybox,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Placetopay,
    connectors::Powertranz,
    connectors::Prophetpay,
    connectors::Mifinity,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Rapyd,
    connectors::Razorpay,
    connectors::Redsys,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::UnifiedAuthenticationService,
    connectors::Worldline,
    connectors::Volt,
    connectors::Worldpay,
    connectors::Wellsfargo,
    connectors::Xendit,
    connectors::Zen,
    connectors::Zsl
);
