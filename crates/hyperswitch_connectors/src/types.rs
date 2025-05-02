#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsData, PayoutsResponseData};
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_data_v2::RouterDataV2,
    router_flow_types::{
        Accept, AccessTokenAuth, Authorize, Capture, Checkout, Defend, Evidence, Fulfillment,
        PSync, PreProcessing, Session, Transaction, Upload, Void,
    },
    router_request_types::{
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckTransactionData,
        },
        AcceptDisputeRequestData, AccessTokenRequestData, DefendDisputeRequestData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsPreProcessingData,
        PaymentsSessionData, PaymentsSyncData, RefundsData, SubmitEvidenceRequestData,
        UploadFileRequestData,
    },
    router_response_types::{
        fraud_check::FraudCheckResponseData, AcceptDisputeResponse, DefendDisputeResponse,
        PaymentsResponseData, RefundsResponseData, SubmitEvidenceResponse, UploadFileResponse,
    },
};
use hyperswitch_interfaces::api::ConnectorIntegration;

pub(crate) type PaymentsSyncResponseRouterData<R> =
    ResponseRouterData<PSync, R, PaymentsSyncData, PaymentsResponseData>;
pub(crate) type PaymentsResponseRouterData<R> =
    ResponseRouterData<Authorize, R, PaymentsAuthorizeData, PaymentsResponseData>;
pub(crate) type PaymentsCaptureResponseRouterData<R> =
    ResponseRouterData<Capture, R, PaymentsCaptureData, PaymentsResponseData>;
pub(crate) type RefundsResponseRouterData<F, R> =
    ResponseRouterData<F, R, RefundsData, RefundsResponseData>;
pub(crate) type RefreshTokenRouterData =
    RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

pub(crate) type PaymentsCancelResponseRouterData<R> =
    ResponseRouterData<Void, R, PaymentsCancelData, PaymentsResponseData>;
pub(crate) type PaymentsPreprocessingResponseRouterData<R> =
    ResponseRouterData<PreProcessing, R, PaymentsPreProcessingData, PaymentsResponseData>;
pub(crate) type PaymentsSessionResponseRouterData<R> =
    ResponseRouterData<Session, R, PaymentsSessionData, PaymentsResponseData>;

pub(crate) type AcceptDisputeRouterData =
    RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;
pub(crate) type SubmitEvidenceRouterData =
    RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;
pub(crate) type UploadFileRouterData =
    RouterData<Upload, UploadFileRequestData, UploadFileResponse>;
pub(crate) type DefendDisputeRouterData =
    RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>;

#[cfg(feature = "payouts")]
pub type PayoutsResponseRouterData<F, R> =
    ResponseRouterData<F, R, PayoutsData, PayoutsResponseData>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
pub struct ResponseRouterData<Flow, R, Request, Response> {
    pub response: R,
    pub data: RouterData<Flow, Request, Response>,
    pub http_code: u16,
}
pub type FrmFulfillmentRouterData =
    RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>;
pub type FrmCheckoutType =
    dyn ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>;
pub type FrmTransactionType =
    dyn ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>;
pub type FrmTransactionRouterData =
    RouterData<Transaction, FraudCheckTransactionData, FraudCheckResponseData>;
pub type FrmFulfillmentType =
    dyn ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>;
pub type FrmCheckoutRouterData =
    RouterData<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>;

pub struct ResponseRouterDataV2<Flow, R, ResourceCommonData, Request, Response> {
    pub response: R,
    pub data: RouterDataV2<Flow, ResourceCommonData, Request, Response>,
    pub http_code: u16,
}
