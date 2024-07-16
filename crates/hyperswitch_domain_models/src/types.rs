use crate::{
    router_data::{AccessToken, PaymentMethodToken, RouterData},
    router_flow_types::{
        self,
        access_token_auth::AccessTokenAuth,
        dispute::{Accept, Defend, Evidence},
        files::{Retrieve, Upload},
        mandate_revoke::MandateRevoke,
        payments::{
            Approve, Authorize, AuthorizeSessionToken, Balance, Capture, CompleteAuthorize,
            CreateConnectorCustomer, IncrementalAuthorization, InitPayment, PSync, PreProcessing,
            Reject, Session, SetupMandate, Void,
        },
        refunds::{Execute, RSync},
        webhooks::VerifyWebhookSource,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenRequestData, AuthorizeSessionTokenData,
        CompleteAuthorizeData, ConnectorCustomerData, DefendDisputeRequestData,
        MandateRevokeRequestData, PaymentMethodTokenizationData, PaymentsApproveData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsIncrementalAuthorizationData, PaymentsPreProcessingData, PaymentsRejectData,
        PaymentsSessionData, PaymentsSyncData, PayoutsData, RefundsData, RetrieveFileRequestData,
        SetupMandateRequestData, SubmitEvidenceRequestData, UploadFileRequestData,
        VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, DefendDisputeResponse, MandateRevokeResponseData,
        PaymentsResponseData, PayoutsResponseData, RefundsResponseData, RetrieveFileResponse,
        SubmitEvidenceResponse, UploadFileResponse, VerifyWebhookSourceResponseData,
    },
};
pub type PaymentsAuthorizeRouterData =
    RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsPreProcessingRouterData =
    RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsAuthorizeSessionTokenRouterData =
    RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>;
pub type PaymentsCompleteAuthorizeRouterData =
    RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
pub type PaymentsInitRouterData =
    RouterData<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsBalanceRouterData =
    RouterData<Balance, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData = RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsIncrementalAuthorizationRouterData = RouterData<
    IncrementalAuthorization,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>;
pub type PaymentsCancelRouterData = RouterData<Void, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsRejectRouterData = RouterData<Reject, PaymentsRejectData, PaymentsResponseData>;
pub type PaymentsApproveRouterData = RouterData<Approve, PaymentsApproveData, PaymentsResponseData>;
pub type PaymentsSessionRouterData = RouterData<Session, PaymentsSessionData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundExecuteRouterData = RouterData<Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<RSync, RefundsData, RefundsResponseData>;
pub type TokenizationRouterData = RouterData<
    router_flow_types::PaymentMethodToken,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>;
pub type ConnectorCustomerRouterData =
    RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;

pub type RefreshTokenRouterData = RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub type PaymentsResponseRouterData<R> =
    ResponseRouterData<Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsBalanceResponseRouterData<R> =
    ResponseRouterData<Balance, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsSessionResponseRouterData<R> =
    ResponseRouterData<Session, R, PaymentsSessionData, PaymentsResponseData>;
pub type PaymentsInitResponseRouterData<R> =
    ResponseRouterData<InitPayment, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsPreprocessingResponseRouterData<R> =
    ResponseRouterData<PreProcessing, R, PaymentsPreProcessingData, PaymentsResponseData>;
pub type TokenizationResponseRouterData<R> =
    ResponseRouterData<PaymentMethodToken, R, PaymentMethodTokenizationData, PaymentsResponseData>;
pub type ConnectorCustomerResponseRouterData<R> =
    ResponseRouterData<CreateConnectorCustomer, R, ConnectorCustomerData, PaymentsResponseData>;

pub type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;

pub type SetupMandateRouterData =
    RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;

pub type AcceptDisputeRouterData =
    RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;

pub type VerifyWebhookSourceRouterData = RouterData<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>;

pub type SubmitEvidenceRouterData =
    RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;

pub type UploadFileRouterData = RouterData<Upload, UploadFileRequestData, UploadFileResponse>;

pub type RetrieveFileRouterData =
    RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>;

pub type DefendDisputeRouterData =
    RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>;

pub type MandateRevokeRouterData =
    RouterData<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutsRouterData<F> = RouterData<F, PayoutsData, PayoutsResponseData>;

#[cfg(feature = "payouts")]
pub type PayoutsResponseRouterData<F, R> =
    ResponseRouterData<F, R, PayoutsData, PayoutsResponseData>;

pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}
