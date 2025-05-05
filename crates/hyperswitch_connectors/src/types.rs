#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsData, PayoutsResponseData};
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_data_v2::RouterDataV2,
    router_flow_types::{
        authentication::{
            Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
        },
        Accept, AccessTokenAuth, Authorize, Capture, Defend, Evidence, PSync, PostProcessing,
        PreProcessing, Session, Upload, Void,
    },
    router_request_types::{
        authentication::{
            ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
            PreAuthNRequestData,
        },
        AcceptDisputeRequestData, AccessTokenRequestData, DefendDisputeRequestData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsPostProcessingData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData, RefundsData,
        SubmitEvidenceRequestData, UploadFileRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, AuthenticationResponseData, DefendDisputeResponse,
        PaymentsResponseData, RefundsResponseData, SubmitEvidenceResponse, UploadFileResponse,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_flow_types::{Checkout, Fulfillment, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckTransactionData,
    },
    router_response_types::fraud_check::FraudCheckResponseData,
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
pub(crate) type PayoutsResponseRouterData<F, R> =
    ResponseRouterData<F, R, PayoutsData, PayoutsResponseData>;

// TODO: Remove `ResponseRouterData` from router crate after all the related type aliases are moved to this crate.
pub(crate) struct ResponseRouterData<Flow, R, Request, Response> {
    pub(crate) response: R,
    pub(crate) data: RouterData<Flow, Request, Response>,
    pub(crate) http_code: u16,
}
#[cfg(feature = "frm")]
pub(crate) type FrmFulfillmentRouterData =
    RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmCheckoutType =
    dyn ConnectorIntegration<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmTransactionType =
    dyn ConnectorIntegration<Transaction, FraudCheckTransactionData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmTransactionRouterData =
    RouterData<Transaction, FraudCheckTransactionData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmFulfillmentType =
    dyn ConnectorIntegration<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmCheckoutRouterData =
    RouterData<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>;

pub(crate) struct ResponseRouterDataV2<Flow, R, ResourceCommonData, Request, Response> {
    pub response: R,
    pub data: RouterDataV2<Flow, ResourceCommonData, Request, Response>,
    pub http_code: u16,
}

pub(crate) type PreAuthNRouterData =
    RouterData<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>;
pub(crate) type PreAuthNVersionCallRouterData =
    RouterData<PreAuthenticationVersionCall, PreAuthNRequestData, AuthenticationResponseData>;
pub(crate) type ConnectorAuthenticationRouterData =
    RouterData<Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData>;
pub(crate) type ConnectorPostAuthenticationRouterData = RouterData<
    PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>;
pub(crate) type ConnectorAuthenticationType = dyn ConnectorIntegration<
    Authentication,
    ConnectorAuthenticationRequestData,
    AuthenticationResponseData,
>;
pub(crate) type ConnectorPostAuthenticationType = dyn ConnectorIntegration<
    PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
>;
pub(crate) type ConnectorPreAuthenticationType =
    dyn ConnectorIntegration<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>;
pub(crate) type ConnectorPreAuthenticationVersionCallType = dyn ConnectorIntegration<
    PreAuthenticationVersionCall,
    PreAuthNRequestData,
    AuthenticationResponseData,
>;

pub(crate) type PaymentsPostProcessingRouterData =
    RouterData<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>;
