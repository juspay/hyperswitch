pub use diesel_models::types::OrderDetailsWithAmount;

use crate::{
    router_data::{AccessToken, AccessTokenAuthenticationResponse, RouterData},
    router_data_v2::{self, RouterDataV2},
    router_flow_types::{
        mandate_revoke::MandateRevoke, revenue_recovery::RecoveryRecordBack, AccessTokenAuth,
        AccessTokenAuthentication, Authenticate, AuthenticationConfirmation, Authorize,
        AuthorizeSessionToken, BillingConnectorInvoiceSync, BillingConnectorPaymentsSync,
        CalculateTax, Capture, CompleteAuthorize, CreateConnectorCustomer, CreateOrder, Execute,
        ExternalVaultProxy, IncrementalAuthorization, PSync, PaymentMethodToken, PostAuthenticate,
        PostCaptureVoid, PostSessionTokens, PreAuthenticate, PreProcessing, RSync,
        SdkSessionUpdate, Session, SetupMandate, UpdateMetadata, VerifyWebhookSource, Void,
    },
    router_request_types::{
        revenue_recovery::{
            BillingConnectorInvoiceSyncRequest, BillingConnectorPaymentsSyncRequest,
            RevenueRecoveryRecordBackRequest,
        },
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AccessTokenAuthenticationRequestData, AccessTokenRequestData, AuthorizeSessionTokenData,
        CompleteAuthorizeData, ConnectorCustomerData, CreateOrderRequestData,
        ExternalVaultProxyPaymentsData, MandateRevokeRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCancelPostCaptureData,
        PaymentsCaptureData, PaymentsIncrementalAuthorizationData, PaymentsPostSessionTokensData,
        PaymentsPreProcessingData, PaymentsSessionData, PaymentsSyncData,
        PaymentsTaxCalculationData, PaymentsUpdateMetadataData, RefundsData,
        SdkPaymentsSessionUpdateData, SetupMandateRequestData, VaultRequestData,
        VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        revenue_recovery::{
            BillingConnectorInvoiceSyncResponse, BillingConnectorPaymentsSyncResponse,
            RevenueRecoveryRecordBackResponse,
        },
        MandateRevokeResponseData, PaymentsResponseData, RefundsResponseData,
        TaxCalculationResponseData, VaultResponseData, VerifyWebhookSourceResponseData,
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

pub type RevenueRecoveryRecordBackRouterDataV2 = RouterDataV2<
    RecoveryRecordBack,
    router_data_v2::flow_common_types::RevenueRecoveryRecordBackData,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
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
