// impl api::PaymentIncrementalAuthorization for Helcim {}
// impl api::ConnectorCustomer for Helcim {}
// impl api::PaymentsPreProcessing for Helcim {}
// impl api::PaymentReject for Helcim {}
// impl api::PaymentApprove for Helcim {}
use common_utils::errors::CustomResult;
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_flow_types::fraud_check::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::{
        PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
        PoSync,
    },
    router_request_types::PayoutsData,
    router_response_types::PayoutsResponseData,
};
use hyperswitch_domain_models::{
    router_flow_types::{
        dispute::{Accept, Defend, Evidence},
        files::{Retrieve, Upload},
        mandate_revoke::MandateRevoke,
        payments::{
            Approve, AuthorizeSessionToken, CalculateTax, CompleteAuthorize,
            CreateConnectorCustomer, IncrementalAuthorization, PostProcessing, PostSessionTokens,
            PreProcessing, Reject, SdkSessionUpdate,
        },
        webhooks::VerifyWebhookSource,
    },
    router_request_types::{
        AcceptDisputeRequestData, AuthorizeSessionTokenData, CompleteAuthorizeData,
        ConnectorCustomerData, DefendDisputeRequestData, MandateRevokeRequestData,
        PaymentsApproveData, PaymentsIncrementalAuthorizationData, PaymentsPostProcessingData,
        PaymentsPostSessionTokensData, PaymentsPreProcessingData, PaymentsRejectData,
        PaymentsTaxCalculationData, RetrieveFileRequestData, SdkPaymentsSessionUpdateData,
        SubmitEvidenceRequestData, UploadFileRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, MandateRevokeResponseData,
        PaymentsResponseData, RetrieveFileResponse, SubmitEvidenceResponse,
        TaxCalculationResponseData, UploadFileResponse, VerifyWebhookSourceResponseData,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_interfaces::api::fraud_check::{
    FraudCheckCheckout, FraudCheckFulfillment, FraudCheckRecordReturn, FraudCheckSale,
    FraudCheckTransaction,
};
#[cfg(feature = "payouts")]
use hyperswitch_interfaces::api::payouts::{
    PayoutCancel, PayoutCreate, PayoutEligibility, PayoutFulfill, PayoutQuote, PayoutRecipient,
    PayoutRecipientAccount, PayoutSync,
};
use hyperswitch_interfaces::{
    api::{
        self,
        disputes::{AcceptDispute, DefendDispute, Dispute, SubmitEvidence},
        files::{FileUpload, RetrieveFile, UploadFile},
        payments::{
            ConnectorCustomer, PaymentApprove, PaymentAuthorizeSessionToken,
            PaymentIncrementalAuthorization, PaymentPostSessionTokens, PaymentReject,
            PaymentSessionUpdate, PaymentsCompleteAuthorize, PaymentsPostProcessing,
            PaymentsPreProcessing, TaxCalculation,
        },
        ConnectorIntegration, ConnectorMandateRevoke, ConnectorRedirectResponse,
    },
    errors::ConnectorError,
};

macro_rules! default_imp_for_authorize_session_token {
    ($($path:ident::$connector:ident),*) => {
        $( impl PaymentAuthorizeSessionToken for $path::$connector {}
            impl
            ConnectorIntegration<
                AuthorizeSessionToken,
                AuthorizeSessionTokenData,
                PaymentsResponseData
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_authorize_session_token!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Taxjar,
    connectors::Volt,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_calculate_tax {
    ($($path:ident::$connector:ident),*) => {
        $( impl TaxCalculation for $path::$connector {}
            impl
            ConnectorIntegration<
                CalculateTax,
                PaymentsTaxCalculationData,
                TaxCalculationResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_calculate_tax!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Volt,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_session_update {
    ($($path:ident::$connector:ident),*) => {
        $( impl PaymentSessionUpdate for $path::$connector {}
            impl
            ConnectorIntegration<
                SdkSessionUpdate,
                SdkPaymentsSessionUpdateData,
                PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_session_update!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Forte,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Fiuu,
    connectors::Globepay,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Zen,
    connectors::Zsl,
    connectors::Powertranz,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Deutschebank,
    connectors::Volt
);

macro_rules! default_imp_for_post_session_tokens {
    ($($path:ident::$connector:ident),*) => {
        $( impl PaymentPostSessionTokens for $path::$connector {}
            impl
            ConnectorIntegration<
                PostSessionTokens,
                PaymentsPostSessionTokensData,
                PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_post_session_tokens!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Bitpay,
    connectors::Billwerk,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Square,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Forte,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Taxjar,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Fiuu,
    connectors::Globepay,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Powertranz,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Deutschebank,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

use crate::connectors;
macro_rules! default_imp_for_complete_authorize {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentsCompleteAuthorize for $path::$connector {}
            impl
            ConnectorIntegration<
            CompleteAuthorize,
            CompleteAuthorizeData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_complete_authorize!(
    connectors::Inespay,
    connectors::Amazonpay,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Multisafepay,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Razorpay,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_incremental_authorization {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentIncrementalAuthorization for $path::$connector {}
            impl
            ConnectorIntegration<
            IncrementalAuthorization,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_incremental_authorization!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_create_customer {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl ConnectorCustomer for $path::$connector {}
            impl
            ConnectorIntegration<
            CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_create_customer!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_connector_redirect_response {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl ConnectorRedirectResponse for $path::$connector {
                fn get_flow_type(
                    &self,
                    _query_params: &str,
                    _json_payload: Option<serde_json::Value>,
                    _action: common_enums::enums::PaymentAction
                ) -> CustomResult<common_enums::enums::CallConnectorAction, ConnectorError> {
                    Ok(common_enums::enums::CallConnectorAction::Trigger)
                }
            }
    )*
    };
}

default_imp_for_connector_redirect_response!(
    connectors::Inespay,
    connectors::Amazonpay,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Multisafepay,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nomupay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Volt,
    connectors::Zsl
);

macro_rules! default_imp_for_pre_processing_steps{
    ($($path:ident::$connector:ident),*)=> {
        $(
            impl PaymentsPreProcessing for $path::$connector {}
            impl
            ConnectorIntegration<
            PreProcessing,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_pre_processing_steps!(
    connectors::Inespay,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_post_processing_steps{
    ($($path:ident::$connector:ident),*)=> {
        $(
            impl PaymentsPostProcessing for $path::$connector {}
            impl
            ConnectorIntegration<
            PostProcessing,
            PaymentsPostProcessingData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_post_processing_steps!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_approve {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentApprove for $path::$connector {}
            impl
            ConnectorIntegration<
            Approve,
            PaymentsApproveData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_approve!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_reject {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentReject for $path::$connector {}
            impl
            ConnectorIntegration<
            Reject,
            PaymentsRejectData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_reject!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_webhook_source_verification {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::ConnectorVerifyWebhookSource for $path::$connector {}
            impl
            ConnectorIntegration<
            VerifyWebhookSource,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_webhook_source_verification!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_accept_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl Dispute for $path::$connector {}
            impl AcceptDispute for $path::$connector {}
            impl
                ConnectorIntegration<
                Accept,
                AcceptDisputeRequestData,
                AcceptDisputeResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_accept_dispute!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_submit_evidence {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl SubmitEvidence for $path::$connector {}
            impl
                ConnectorIntegration<
                Evidence,
                SubmitEvidenceRequestData,
                SubmitEvidenceResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_submit_evidence!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_defend_dispute {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl DefendDispute for $path::$connector {}
            impl
                ConnectorIntegration<
                Defend,
                DefendDisputeRequestData,
                DefendDisputeResponse,
            > for $path::$connector
            {}
        )*
    };
}

default_imp_for_defend_dispute!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Jpmorgan,
    connectors::Helcim,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_file_upload {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FileUpload for $path::$connector {}
            impl UploadFile for $path::$connector {}
            impl
                ConnectorIntegration<
                Upload,
                UploadFileRequestData,
                UploadFileResponse,
            > for $path::$connector
            {}
            impl RetrieveFile for $path::$connector {}
            impl
                ConnectorIntegration<
                Retrieve,
                RetrieveFileRequestData,
                RetrieveFileResponse,
            > for $path::$connector
            {}
    )*
    };
}

default_imp_for_file_upload!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_payouts {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::Payouts for $path::$connector {}
    )*
    };
}

default_imp_for_payouts!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Cryptopay,
    connectors::Coinbase,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Square,
    connectors::Stax,
    connectors::Taxjar,
    connectors::Tsys,
    connectors::Volt,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_create {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutCreate for $path::$connector {}
            impl
            ConnectorIntegration<
            PoCreate,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_create!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_retrieve {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutSync for $path::$connector {}
            impl
            ConnectorIntegration<
            PoSync,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_retrieve!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_eligibility {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutEligibility for $path::$connector {}
            impl
            ConnectorIntegration<
            PoEligibility,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_eligibility!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_fulfill {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutFulfill for $path::$connector {}
            impl
            ConnectorIntegration<
            PoFulfill,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_fulfill!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_cancel {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutCancel for $path::$connector {}
            impl
            ConnectorIntegration<
            PoCancel,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_cancel!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_quote {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutQuote for $path::$connector {}
            impl
            ConnectorIntegration<
            PoQuote,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_quote!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_recipient {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutRecipient for $path::$connector {}
            impl
            ConnectorIntegration<
            PoRecipient,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_recipient!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "payouts")]
macro_rules! default_imp_for_payouts_recipient_account {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PayoutRecipientAccount for $path::$connector {}
            impl
            ConnectorIntegration<
            PoRecipientAccount,
            PayoutsData,
            PayoutsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "payouts")]
default_imp_for_payouts_recipient_account!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_sale {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckSale for $path::$connector {}
            impl
            ConnectorIntegration<
            Sale,
            FraudCheckSaleData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_frm_sale!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_checkout {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckCheckout for $path::$connector {}
            impl
            ConnectorIntegration<
            Checkout,
            FraudCheckCheckoutData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_frm_checkout!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_transaction {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckTransaction for $path::$connector {}
            impl
            ConnectorIntegration<
            Transaction,
            FraudCheckTransactionData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_frm_transaction!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_fulfillment {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckFulfillment for $path::$connector {}
            impl
            ConnectorIntegration<
            Fulfillment,
            FraudCheckFulfillmentData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_frm_fulfillment!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

#[cfg(feature = "frm")]
macro_rules! default_imp_for_frm_record_return {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl FraudCheckRecordReturn for $path::$connector {}
            impl
            ConnectorIntegration<
            RecordReturn,
            FraudCheckRecordReturnData,
            FraudCheckResponseData,
        > for $path::$connector
        {}
    )*
    };
}

#[cfg(feature = "frm")]
default_imp_for_frm_record_return!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);

macro_rules! default_imp_for_revoking_mandates {
    ($($path:ident::$connector:ident),*) => {
        $( impl ConnectorMandateRevoke for $path::$connector {}
            impl
            ConnectorIntegration<
            MandateRevoke,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_revoking_mandates!(
    connectors::Inespay,
    connectors::Airwallex,
    connectors::Amazonpay,
    connectors::Bambora,
    connectors::Billwerk,
    connectors::Bitpay,
    connectors::Cashtocode,
    connectors::Coinbase,
    connectors::Cryptopay,
    connectors::Deutschebank,
    connectors::Digitalvirgo,
    connectors::Dlocal,
    connectors::Elavon,
    connectors::Fiserv,
    connectors::Fiservemea,
    connectors::Fiuu,
    connectors::Forte,
    connectors::Globepay,
    connectors::Helcim,
    connectors::Jpmorgan,
    connectors::Nomupay,
    connectors::Novalnet,
    connectors::Nexinets,
    connectors::Nexixpay,
    connectors::Payeezy,
    connectors::Payu,
    connectors::Powertranz,
    connectors::Mollie,
    connectors::Multisafepay,
    connectors::Razorpay,
    connectors::Shift4,
    connectors::Stax,
    connectors::Square,
    connectors::Taxjar,
    connectors::Thunes,
    connectors::Tsys,
    connectors::Worldline,
    connectors::Worldpay,
    connectors::Volt,
    connectors::Zen,
    connectors::Zsl
);
