#[cfg(feature = "v2")]
use hyperswitch_domain_models::router_data_v2::RouterDataV2;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::types::{PayoutsData, PayoutsResponseData};
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{
        authentication::{
            Authentication, PostAuthentication, PreAuthentication, PreAuthenticationVersionCall,
        },
        Accept, AccessTokenAuth, Authorize, Capture, CreateOrder, Defend, Dsync, Evidence,
        ExtendAuthorization, Fetch, PSync, PostProcessing, PreProcessing, Retrieve, Session,
        Upload, Void,
    },
    router_request_types::{
        authentication::{
            ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
            PreAuthNRequestData,
        },
        AcceptDisputeRequestData, AccessTokenRequestData, CreateOrderRequestData,
        DefendDisputeRequestData, DisputeSyncData, FetchDisputesRequestData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsExtendAuthorizationData,
        PaymentsPostProcessingData, PaymentsPreProcessingData, PaymentsSessionData,
        PaymentsSyncData, RefundsData, RetrieveFileRequestData, SubmitEvidenceRequestData,
        UploadFileRequestData,
    },
    router_response_types::{
        AcceptDisputeResponse, AuthenticationResponseData, DefendDisputeResponse,
        DisputeSyncResponse, FetchDisputesResponse, PaymentsResponseData, RefundsResponseData,
        RetrieveFileResponse, SubmitEvidenceResponse, UploadFileResponse,
    },
};
#[cfg(feature = "frm")]
use hyperswitch_domain_models::{
    router_flow_types::{Checkout, Fulfillment, RecordReturn, Sale, Transaction},
    router_request_types::fraud_check::{
        FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
        FraudCheckSaleData, FraudCheckTransactionData,
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
pub(crate) type CreateOrderResponseRouterData<R> =
    ResponseRouterData<CreateOrder, R, CreateOrderRequestData, PaymentsResponseData>;
pub(crate) type PaymentsExtendAuthorizationResponseRouterData<R> = ResponseRouterData<
    ExtendAuthorization,
    R,
    PaymentsExtendAuthorizationData,
    PaymentsResponseData,
>;

pub(crate) type AcceptDisputeRouterData =
    RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;
pub(crate) type SubmitEvidenceRouterData =
    RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;
pub(crate) type UploadFileRouterData =
    RouterData<Upload, UploadFileRequestData, UploadFileResponse>;
pub(crate) type DefendDisputeRouterData =
    RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>;
pub(crate) type FetchDisputeRouterData =
    RouterData<Fetch, FetchDisputesRequestData, FetchDisputesResponse>;
pub(crate) type DisputeSyncRouterData = RouterData<Dsync, DisputeSyncData, DisputeSyncResponse>;

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
#[cfg(feature = "v2")]
pub(crate) struct ResponseRouterDataV2<Flow, R, ResourceCommonData, Request, Response> {
    pub response: R,
    pub data: RouterDataV2<Flow, ResourceCommonData, Request, Response>,
    #[allow(dead_code)] // Used for metadata passing but this is not read
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
#[cfg(feature = "frm")]
pub(crate) type FrmSaleRouterData = RouterData<Sale, FraudCheckSaleData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmRecordReturnRouterData =
    RouterData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmRecordReturnType =
    dyn ConnectorIntegration<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>;
#[cfg(feature = "frm")]
pub(crate) type FrmSaleType =
    dyn ConnectorIntegration<Sale, FraudCheckSaleData, FraudCheckResponseData>;

pub(crate) type RetrieveFileRouterData =
    RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>;

#[cfg(feature = "payouts")]
pub(crate) trait PayoutIndividualDetailsExt {
    type Error;
    fn get_external_account_account_holder_type(&self) -> Result<String, Self::Error>;
}

#[cfg(feature = "payouts")]
impl PayoutIndividualDetailsExt for api_models::payouts::PayoutIndividualDetails {
    type Error = error_stack::Report<hyperswitch_interfaces::errors::ConnectorError>;
    fn get_external_account_account_holder_type(&self) -> Result<String, Self::Error> {
        self.external_account_account_holder_type
            .clone()
            .ok_or_else(crate::utils::missing_field_err(
                "external_account_account_holder_type",
            ))
    }
}
