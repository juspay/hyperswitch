pub use diesel_models::types::OrderDetailsWithAmount;

use crate::{
    router_data::{AccessToken, AccessTokenAuthenticationResponse, RouterData},
    router_data_v2::{self, RouterDataV2},
    router_flow_types::{
        mandate_revoke::MandateRevoke,
        revenue_recovery::InvoiceRecordBack,
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionPlanPrices, GetSubscriptionPlans,
            SubscriptionCancel, SubscriptionCreate, SubscriptionPause, SubscriptionResume,
        },
        AccessTokenAuth, AccessTokenAuthentication, Authenticate, AuthenticationConfirmation,
        Authorize, AuthorizeSessionToken, BillingConnectorInvoiceSync,
        BillingConnectorPaymentsSync, CalculateTax, Capture, CompleteAuthorize,
        CreateConnectorCustomer, CreateOrder, Execute, ExtendAuthorization, ExternalVaultProxy,
        GiftCardBalanceCheck, IncrementalAuthorization, PSync, PaymentMethodToken,
        PostAuthenticate, PostCaptureVoid, PostSessionTokens, PreAuthenticate, PreProcessing,
        RSync, SdkSessionUpdate, Session, SetupMandate, UpdateMetadata, VerifyWebhookSource, Void,
    },
    router_request_types::{
        revenue_recovery::{
            BillingConnectorInvoiceSyncRequest, BillingConnectorPaymentsSyncRequest,
            InvoiceRecordBackRequest,
        },
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCancelRequest, SubscriptionCreateRequest,
            SubscriptionPauseRequest, SubscriptionResumeRequest,
        },
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AccessTokenAuthenticationRequestData, AccessTokenRequestData, AuthorizeSessionTokenData,
        CompleteAuthorizeData, ConnectorCustomerData, CreateOrderRequestData,
        ExternalVaultProxyPaymentsData, GiftCardBalanceCheckRequestData, MandateRevokeRequestData,
        PaymentMethodTokenizationData, PaymentsAuthenticateData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCancelPostCaptureData, PaymentsCaptureData,
        PaymentsExtendAuthorizationData, PaymentsIncrementalAuthorizationData,
        PaymentsPostAuthenticateData, PaymentsPostSessionTokensData, PaymentsPreAuthenticateData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData,
        PaymentsTaxCalculationData, PaymentsUpdateMetadataData, RefundsData,
        SdkPaymentsSessionUpdateData, SetupMandateRequestData, VaultRequestData,
        VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        revenue_recovery::{
            BillingConnectorInvoiceSyncResponse, BillingConnectorPaymentsSyncResponse,
            InvoiceRecordBackResponse,
        },
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCancelResponse, SubscriptionCreateResponse,
            SubscriptionPauseResponse, SubscriptionResumeResponse,
        },
        GiftCardBalanceCheckResponseData, MandateRevokeResponseData, PaymentsResponseData,
        RefundsResponseData, TaxCalculationResponseData, VaultResponseData,
        VerifyWebhookSourceResponseData,
    },
};
#[cfg(feature = "payouts")]
pub use crate::{router_request_types::PayoutsData, router_response_types::PayoutsResponseData};

pub type PaymentsAuthorizeRouterData =
    RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
pub type ExternalVaultProxyPaymentsRouterData =
    RouterData<ExternalVaultProxy, ExternalVaultProxyPaymentsData, PaymentsResponseData>;
pub type PaymentsAuthorizeSessionTokenRouterData =
    RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>;
pub type PaymentsPreProcessingRouterData =
    RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
pub type PaymentsPreAuthenticateRouterData =
    RouterData<PreAuthenticate, PaymentsPreAuthenticateData, PaymentsResponseData>;
pub type PaymentsAuthenticateRouterData =
    RouterData<Authenticate, PaymentsAuthenticateData, PaymentsResponseData>;
pub type PaymentsPostAuthenticateRouterData =
    RouterData<PostAuthenticate, PaymentsPostAuthenticateData, PaymentsResponseData>;
pub type PaymentsSyncRouterData = RouterData<PSync, PaymentsSyncData, PaymentsResponseData>;
pub type PaymentsCaptureRouterData = RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>;
pub type PaymentsCancelRouterData = RouterData<Void, PaymentsCancelData, PaymentsResponseData>;
pub type PaymentsCancelPostCaptureRouterData =
    RouterData<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>;
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
pub type AccessTokenAuthenticationRouterData = RouterData<
    AccessTokenAuthentication,
    AccessTokenAuthenticationRequestData,
    AccessTokenAuthenticationResponse,
>;
pub type PaymentsGiftCardBalanceCheckRouterData = RouterData<
    GiftCardBalanceCheck,
    GiftCardBalanceCheckRequestData,
    GiftCardBalanceCheckResponseData,
>;
pub type RefreshTokenRouterData = RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>;
pub type PaymentsPostSessionTokensRouterData =
    RouterData<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>;
pub type PaymentsSessionRouterData = RouterData<Session, PaymentsSessionData, PaymentsResponseData>;
pub type PaymentsUpdateMetadataRouterData =
    RouterData<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>;

pub type CreateOrderRouterData =
    RouterData<CreateOrder, CreateOrderRequestData, PaymentsResponseData>;
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
pub type PaymentsExtendAuthorizationRouterData =
    RouterData<ExtendAuthorization, PaymentsExtendAuthorizationData, PaymentsResponseData>;
pub type SdkSessionUpdateRouterData =
    RouterData<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>;

pub type VerifyWebhookSourceRouterData = RouterData<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>;

#[cfg(feature = "payouts")]
pub type PayoutsRouterData<F> = RouterData<F, PayoutsData, PayoutsResponseData>;

pub type InvoiceRecordBackRouterData =
    RouterData<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>;

pub type GetSubscriptionPlansRouterData =
    RouterData<GetSubscriptionPlans, GetSubscriptionPlansRequest, GetSubscriptionPlansResponse>;

pub type GetSubscriptionEstimateRouterData = RouterData<
    GetSubscriptionEstimate,
    GetSubscriptionEstimateRequest,
    GetSubscriptionEstimateResponse,
>;

pub type SubscriptionPauseRouterData =
    RouterData<SubscriptionPause, SubscriptionPauseRequest, SubscriptionPauseResponse>;

pub type SubscriptionResumeRouterData =
    RouterData<SubscriptionResume, SubscriptionResumeRequest, SubscriptionResumeResponse>;

pub type SubscriptionCancelRouterData =
    RouterData<SubscriptionCancel, SubscriptionCancelRequest, SubscriptionCancelResponse>;

pub type UasAuthenticationRouterData =
    RouterData<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>;

pub type BillingConnectorPaymentsSyncRouterData = RouterData<
    BillingConnectorPaymentsSync,
    BillingConnectorPaymentsSyncRequest,
    BillingConnectorPaymentsSyncResponse,
>;

pub type BillingConnectorInvoiceSyncRouterData = RouterData<
    BillingConnectorInvoiceSync,
    BillingConnectorInvoiceSyncRequest,
    BillingConnectorInvoiceSyncResponse,
>;

pub type BillingConnectorInvoiceSyncRouterDataV2 = RouterDataV2<
    BillingConnectorInvoiceSync,
    router_data_v2::flow_common_types::BillingConnectorInvoiceSyncFlowData,
    BillingConnectorInvoiceSyncRequest,
    BillingConnectorInvoiceSyncResponse,
>;

pub type BillingConnectorPaymentsSyncRouterDataV2 = RouterDataV2<
    BillingConnectorPaymentsSync,
    router_data_v2::flow_common_types::BillingConnectorPaymentsSyncFlowData,
    BillingConnectorPaymentsSyncRequest,
    BillingConnectorPaymentsSyncResponse,
>;

pub type InvoiceRecordBackRouterDataV2 = RouterDataV2<
    InvoiceRecordBack,
    router_data_v2::flow_common_types::InvoiceRecordBackData,
    InvoiceRecordBackRequest,
    InvoiceRecordBackResponse,
>;

pub type GetSubscriptionPlanPricesRouterData = RouterData<
    GetSubscriptionPlanPrices,
    GetSubscriptionPlanPricesRequest,
    GetSubscriptionPlanPricesResponse,
>;

pub type VaultRouterData<F> = RouterData<F, VaultRequestData, VaultResponseData>;

pub type VaultRouterDataV2<F> = RouterDataV2<
    F,
    router_data_v2::flow_common_types::VaultConnectorFlowData,
    VaultRequestData,
    VaultResponseData,
>;

pub type ExternalVaultProxyPaymentsRouterDataV2 = RouterDataV2<
    ExternalVaultProxy,
    router_data_v2::flow_common_types::ExternalVaultProxyFlowData,
    ExternalVaultProxyPaymentsData,
    PaymentsResponseData,
>;

pub type SubscriptionCreateRouterData =
    RouterData<SubscriptionCreate, SubscriptionCreateRequest, SubscriptionCreateResponse>;
