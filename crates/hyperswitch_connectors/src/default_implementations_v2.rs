use hyperswitch_domain_models::{
    router_data::AccessToken,
    router_data_v2::{
        flow_common_types::{
            DisputesFlowData, MandateRevokeFlowData, PayoutFlowData, WebhookSourceVerifyData,
        },
        AccessTokenFlowData, FilesFlowData, FrmFlowData,
    },
    router_flow_types::{
        dispute::{Accept, Defend, Evidence},
        files::{Retrieve, Upload},
        fraud_check::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
        mandate_revoke::MandateRevoke,
        payouts::{
            PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
            PoSync,
        },
        webhooks::VerifyWebhookSource,
        AccessTokenAuth,
    },
    router_request_types::{
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
            FraudCheckSaleData, FraudCheckTransactionData,
        },
        AcceptDisputeRequestData, AccessTokenRequestData, DefendDisputeRequestData,
        MandateRevokeRequestData, PayoutsData, RetrieveFileRequestData, SubmitEvidenceRequestData,
        UploadFileRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        fraud_check::FraudCheckResponseData, AcceptDisputeResponse, DefendDisputeResponse,
        MandateRevokeResponseData, PayoutsResponseData, RetrieveFileResponse,
        SubmitEvidenceResponse, UploadFileResponse, VerifyWebhookSourceResponseData,
    },
};
use hyperswitch_interfaces::{
    api::{
        disputes_v2::{AcceptDisputeV2, DefendDisputeV2, DisputeV2, SubmitEvidenceV2},
        files_v2::{FileUploadV2, RetrieveFileV2, UploadFileV2},
        fraud_check_v2::{
            FraudCheckCheckoutV2, FraudCheckFulfillmentV2, FraudCheckRecordReturnV2,
            FraudCheckSaleV2, FraudCheckTransactionV2,
        },
        payouts_v2::{
            PayoutCancelV2, PayoutCreateV2, PayoutEligibilityV2, PayoutFulfillV2, PayoutQuoteV2,
            PayoutRecipientAccountV2, PayoutRecipientV2, PayoutSyncV2,
        },
        ConnectorAccessTokenV2, ConnectorMandateRevokeV2, ConnectorVerifyWebhookSourceV2,
    },
    connector_integration_v2::ConnectorIntegrationV2,
};

use crate::connectors;

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

default_imp_for_new_connector_integration_connector_access_token!(connectors::Helcim);

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

default_imp_for_new_connector_integration_accept_dispute!(connectors::Helcim);

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

default_imp_for_new_connector_integration_submit_evidence!(connectors::Helcim);

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

default_imp_for_new_connector_integration_defend_dispute!(connectors::Helcim);

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

default_imp_for_new_connector_integration_file_upload!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_create!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_eligibility!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_fulfill!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_cancel!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_quote!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_recipient!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_sync!(connectors::Helcim);

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
default_imp_for_new_connector_integration_payouts_recipient_account!(connectors::Helcim);

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

default_imp_for_new_connector_integration_webhook_source_verification!(connectors::Helcim);

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
default_imp_for_new_connector_integration_frm_sale!(connectors::Helcim);

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
default_imp_for_new_connector_integration_frm_checkout!(connectors::Helcim);

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
default_imp_for_new_connector_integration_frm_transaction!(connectors::Helcim);

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
default_imp_for_new_connector_integration_frm_fulfillment!(connectors::Helcim);

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
default_imp_for_new_connector_integration_frm_record_return!(connectors::Helcim);

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

default_imp_for_new_connector_integration_revoking_mandates!(connectors::Helcim);
