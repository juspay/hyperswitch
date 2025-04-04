pub use diesel_models::types::OrderDetailsWithAmount;

use crate::{
    router_data::{AccessToken, RouterData},
    router_flow_types::{
        mandate_revoke::MandateRevoke, revenue_recovery::RecoveryRecordBack, AccessTokenAuth,
        Authenticate, AuthenticationConfirmation, Authorize, AuthorizeSessionToken, CalculateTax,
        Capture, CompleteAuthorize, CreateConnectorCustomer, Execute,
        GetAdditionalRevenueRecoveryDetails, IncrementalAuthorization, PSync, PaymentMethodToken,
        PostAuthenticate, PostSessionTokens, PreAuthenticate, PreProcessing, RSync,
        SdkSessionUpdate, Session, SetupMandate, VerifyWebhookSource, Void,
    },
    router_request_types::{
        revenue_recovery::{
            GetAdditionalRevenueRecoveryRequestData, RevenueRecoveryRecordBackRequest,
        },
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AccessTokenRequestData, AuthorizeSessionTokenData, CompleteAuthorizeData,
        ConnectorCustomerData, MandateRevokeRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsIncrementalAuthorizationData, PaymentsPostSessionTokensData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData,
        PaymentsTaxCalculationData, RefundsData, SdkPaymentsSessionUpdateData,
        SetupMandateRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        revenue_recovery::{
            GetAdditionalRevenueRecoveryResponseData, RevenueRecoveryRecordBackResponse,
        },
        MandateRevokeResponseData, PaymentsResponseData, RefundsResponseData,
        TaxCalculationResponseData, VerifyWebhookSourceResponseData,
    },
};
#[cfg(feature = "payouts")]
pub use crate::{router_request_types::PayoutsData, router_response_types::PayoutsResponseData};

pub type PaymentsAuthorizeRouterData =
    RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type PaymentsAuthorizeSessionTokenRouterData =
    RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>;
pub type PaymentsPreProcessingRouterData =
    RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData = RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<Void, PaymentsCancelData, PaymentsResponseData>;
pub type SetupMandateRouterData =
    RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
pub type RefundsRouterData<F> = RouterData<F, RefundsData, RefundsResponseData>;
pub type RefundExecuteRouterData = RouterData<Execute, RefundsData, RefundsResponseData>;
pub type RefundSyncRouterData = RouterData<RSync, RefundsData, RefundsResponseData>;
pub type TokenizationRouterData =
    RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>;
pub type ConnectorCustomerRouterData =
    RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;
pub type PaymentsCompleteAuthorizeRouterData =
    RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
pub type PaymentsTaxCalculationRouterData =
    RouterData<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>;
pub type RefreshTokenRouterData = RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;
pub type PaymentsPostSessionTokensRouterData =
    RouterData<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>;
pub type PaymentsSessionRouterData = RouterData<Session, PaymentsSessionData, PaymentsResponseData>;

pub type UasPostAuthenticationRouterData =
    RouterData<PostAuthenticate, UasPostAuthenticationRequestData, UasAuthenticationResponseData>;
pub type UasPreAuthenticationRouterData =
    RouterData<PreAuthenticate, UasPreAuthenticationRequestData, UasAuthenticationResponseData>;

pub type UasAuthenticationConfirmationRouterData = RouterData<
    AuthenticationConfirmation,
    UasConfirmationRequestData,
    UasAuthenticationResponseData,
>;

pub type MandateRevokeRouterData =
    RouterData<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>;
pub type PaymentsIncrementalAuthorizationRouterData = RouterData<
    IncrementalAuthorization,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>;
pub type SdkSessionUpdateRouterData =
    RouterData<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>;

pub type VerifyWebhookSourceRouterData = RouterData<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>;

#[cfg(feature = "payouts")]
pub type PayoutsRouterData<F> = RouterData<F, PayoutsData, PayoutsResponseData>;

pub type RevenueRecoveryRecordBackRouterData = RouterData<
    RecoveryRecordBack,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>;

pub type UasAuthenticationRouterData =
    RouterData<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>;

pub type AdditionalRevenueRecoveryDetailsRouterData = RouterData<
    GetAdditionalRevenueRecoveryDetails,
    GetAdditionalRevenueRecoveryRequestData,
    GetAdditionalRevenueRecoveryResponseData,
>;
