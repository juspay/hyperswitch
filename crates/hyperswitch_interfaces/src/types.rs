//! Types interface
use hyperswitch_domain_models::{
    router_data::{AccessToken, PaymentMethodToken},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        dispute::{Accept, Defend, Evidence},
        files::{Retrieve, Upload},
        mandate_revoke::MandateRevoke,
        payments::{
            Authorize, AuthorizeSessionToken, Balance, Capture, CompleteAuthorize,
            CreateConnectorCustomer, IncrementalAuthorization, InitPayment, PSync, PreProcessing,
            Session, SetupMandate, Void,
        },
        payouts::{
            PoCancel, PoCreate, PoEligibility, PoFulfill, PoQuote, PoRecipient, PoRecipientAccount,
        },
        refunds::{Execute, RSync},
        webhooks::VerifyWebhookSource,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenRequestData, AuthorizeSessionTokenData,
        CompleteAuthorizeData, ConnectorCustomerData, DefendDisputeRequestData,
        MandateRevokeRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData, PayoutsData, RefundsData,
        RetrieveFileRequestData, SetupMandateRequestData, SubmitEvidenceRequestData,
        UploadFileRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, MandateRevokeResponseData,
        PaymentsResponseData, PayoutsResponseData, RefundsResponseData, RetrieveFileResponse,
        SubmitEvidenceResponse, UploadFileResponse, VerifyWebhookSourceResponseData,
    },
};

use crate::api::ConnectorIntegration;
/// struct Response
#[derive(Clone, Debug)]
pub struct Response {
    /// headers
    pub headers: Option<http::HeaderMap>,
    /// response
    pub response: bytes::Bytes,
    /// status code
    pub status_code: u16,
}

pub type PaymentsAuthorizeType =
    dyn ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type SetupMandateType =
    dyn ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
pub type MandateRevokeType =
    dyn ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>;
pub type PaymentsPreProcessingType =
    dyn ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsCompleteAuthorizeType =
    dyn ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
pub type PaymentsPreAuthorizeType = dyn ConnectorIntegration<
    AuthorizeSessionToken,
    AuthorizeSessionTokenData,
    PaymentsResponseData,
>;
pub type PaymentsInitType =
    dyn ConnectorIntegration<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsBalanceType =
    dyn ConnectorIntegration<Balance, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncType = dyn ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureType =
    dyn ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsSessionType =
    dyn ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>;
pub type PaymentsVoidType =
    dyn ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>;
pub type TokenizationType = dyn ConnectorIntegration<
    PaymentMethodToken,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>;
pub type IncrementalAuthorizationType = dyn ConnectorIntegration<
    IncrementalAuthorization,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>;

pub type ConnectorCustomerType =
    dyn ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;

pub type RefundExecuteType = dyn ConnectorIntegration<Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncType = dyn ConnectorIntegration<RSync, RefundsData, RefundsResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutCancelType = dyn ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData>;
#[cfg(feature = "payouts")]
pub type PayoutCreateType = dyn ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData>;
#[cfg(feature = "payouts")]
pub type PayoutEligibilityType =
    dyn ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData>;
#[cfg(feature = "payouts")]
pub type PayoutFulfillType = dyn ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData>;
#[cfg(feature = "payouts")]
pub type PayoutRecipientType =
    dyn ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData>;
#[cfg(feature = "payouts")]
pub type PayoutRecipientAccountType =
    dyn ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData>;
#[cfg(feature = "payouts")]
pub type PayoutQuoteType = dyn ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData>;

pub type RefreshTokenType =
    dyn ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type AcceptDisputeType =
    dyn ConnectorIntegration<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;
pub type VerifyWebhookSourceType = dyn ConnectorIntegration<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>;

pub type SubmitEvidenceType =
    dyn ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;

pub type UploadFileType =
    dyn ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse>;

pub type RetrieveFileType =
    dyn ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>;

pub type DefendDisputeType =
    dyn ConnectorIntegration<Defend, DefendDisputeRequestData, DefendDisputeResponse>;
