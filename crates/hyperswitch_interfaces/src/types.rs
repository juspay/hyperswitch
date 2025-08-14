//! Types interface

use hyperswitch_domain_models::{
    router_data::{AccessToken, AccessTokenAuthenticationResponse},
    router_data_v2::flow_common_types,
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        dispute::{Accept, Defend, Dsync, Evidence, Fetch},
        files::{Retrieve, Upload},
        mandate_revoke::MandateRevoke,
        payments::{
            Authorize, AuthorizeSessionToken, Balance, CalculateTax, Capture, CompleteAuthorize,
            CreateConnectorCustomer, CreateOrder, IncrementalAuthorization, InitPayment, PSync,
            PaymentMethodToken, PostCaptureVoid, PostProcessing, PostSessionTokens, PreProcessing,
            SdkSessionUpdate, Session, SetupMandate, UpdateMetadata, Void,
        },
        refunds::{Execute, RSync},
        revenue_recovery::{BillingConnectorPaymentsSync, RecoveryRecordBack},
        unified_authentication_service::{
            Authenticate, AuthenticationConfirmation, PostAuthenticate, PreAuthenticate,
        },
        vault::{
            ExternalVaultCreateFlow, ExternalVaultDeleteFlow, ExternalVaultInsertFlow,
            ExternalVaultRetrieveFlow,
        },
        webhooks::VerifyWebhookSource,
        AccessTokenAuthentication, BillingConnectorInvoiceSync,
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
        AcceptDisputeRequestData, AccessTokenAuthenticationRequestData, AccessTokenRequestData,
        AuthorizeSessionTokenData, CompleteAuthorizeData, ConnectorCustomerData,
        CreateOrderRequestData, DefendDisputeRequestData, DisputeSyncData,
        FetchDisputesRequestData, MandateRevokeRequestData, PaymentMethodTokenizationData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCancelPostCaptureData,
        PaymentsCaptureData, PaymentsIncrementalAuthorizationData, PaymentsPostProcessingData,
        PaymentsPostSessionTokensData, PaymentsPreProcessingData, PaymentsSessionData,
        PaymentsSyncData, PaymentsTaxCalculationData, PaymentsUpdateMetadataData, RefundsData,
        RetrieveFileRequestData, SdkPaymentsSessionUpdateData, SetupMandateRequestData,
        SubmitEvidenceRequestData, UploadFileRequestData, VaultRequestData,
        VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        revenue_recovery::{
            BillingConnectorInvoiceSyncResponse, BillingConnectorPaymentsSyncResponse,
            RevenueRecoveryRecordBackResponse,
        },
        AcceptDisputeResponse, DefendDisputeResponse, DisputeSyncResponse, FetchDisputesResponse,
        MandateRevokeResponseData, PaymentsResponseData, RefundsResponseData, RetrieveFileResponse,
        SubmitEvidenceResponse, TaxCalculationResponseData, UploadFileResponse, VaultResponseData,
        VerifyWebhookSourceResponseData,
    },
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

use crate::{api::ConnectorIntegration, connector_integration_v2::ConnectorIntegrationV2};
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

/// Type alias for `ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>`
pub type PaymentsAuthorizeType =
    dyn ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>`
pub type PaymentsTaxCalculationType =
    dyn ConnectorIntegration<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>;
/// Type alias for `ConnectorIntegration<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>`
pub type PaymentsPostSessionTokensType = dyn ConnectorIntegration<
    PostSessionTokens,
    PaymentsPostSessionTokensData,
    PaymentsResponseData,
>;
/// Type alias for `ConnectorIntegration<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>`
pub type PaymentsUpdateMetadataType =
    dyn ConnectorIntegration<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>`
pub type SdkSessionUpdateType =
    dyn ConnectorIntegration<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>`
pub type SetupMandateType =
    dyn ConnectorIntegration<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>`
pub type MandateRevokeType =
    dyn ConnectorIntegration<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>;
/// Type alias for `ConnectorIntegration<CreateOrder, CreateOrderRequestData, PaymentsResponseData>`
pub type CreateOrderType =
    dyn ConnectorIntegration<CreateOrder, CreateOrderRequestData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>`
pub type PaymentsPreProcessingType =
    dyn ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>`
pub type PaymentsPostProcessingType =
    dyn ConnectorIntegration<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>`
pub type PaymentsCompleteAuthorizeType =
    dyn ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>`
pub type PaymentsPreAuthorizeType = dyn ConnectorIntegration<
    AuthorizeSessionToken,
    AuthorizeSessionTokenData,
    PaymentsResponseData,
>;
/// Type alias for `ConnectorIntegration<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>`
pub type PaymentsInitType =
    dyn ConnectorIntegration<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<Balance, PaymentsAuthorizeData, PaymentsResponseData`
pub type PaymentsBalanceType =
    dyn ConnectorIntegration<Balance, PaymentsAuthorizeData, PaymentsResponseData>;
/// Type alias for `PaymentsSyncType = dyn ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>`
pub type PaymentsSyncType = dyn ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>`
pub type PaymentsCaptureType =
    dyn ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>`
pub type PaymentsSessionType =
    dyn ConnectorIntegration<Session, PaymentsSessionData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>`
pub type PaymentsVoidType =
    dyn ConnectorIntegration<Void, PaymentsCancelData, PaymentsResponseData>;
/// Type alias for `ConnectorIntegration<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>`
pub type PaymentsPostCaptureVoidType =
    dyn ConnectorIntegration<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>;

/// Type alias for `ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>`
pub type TokenizationType = dyn ConnectorIntegration<
    PaymentMethodToken,
    PaymentMethodTokenizationData,
    PaymentsResponseData,
>;
/// Type alias for `ConnectorIntegration<IncrementalAuthorization, PaymentsIncrementalAuthorizationData, PaymentsResponseData>`
pub type IncrementalAuthorizationType = dyn ConnectorIntegration<
    IncrementalAuthorization,
    PaymentsIncrementalAuthorizationData,
    PaymentsResponseData,
>;

/// Type alias for `ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>`
pub type ConnectorCustomerType =
    dyn ConnectorIntegration<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>;

/// Type alias for `ConnectorIntegration<Execute, RefundsData, RefundsResponseData>`
pub type RefundExecuteType = dyn ConnectorIntegration<Execute, RefundsData, RefundsResponseData>;
/// Type alias for `ConnectorIntegration<RSync, RefundsData, RefundsResponseData>`
pub type RefundSyncType = dyn ConnectorIntegration<RSync, RefundsData, RefundsResponseData>;

/// Type alias for `ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutCancelType = dyn ConnectorIntegration<PoCancel, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutCreateType = dyn ConnectorIntegration<PoCreate, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutEligibilityType =
    dyn ConnectorIntegration<PoEligibility, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutFulfillType = dyn ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutRecipientType =
    dyn ConnectorIntegration<PoRecipient, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutRecipientAccountType =
    dyn ConnectorIntegration<PoRecipientAccount, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutQuoteType = dyn ConnectorIntegration<PoQuote, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData>`
#[cfg(feature = "payouts")]
pub type PayoutSyncType = dyn ConnectorIntegration<PoSync, PayoutsData, PayoutsResponseData>;
/// Type alias for `ConnectorIntegration<AccessTokenAuthentication, AccessTokenAuthenticationRequestData, AccessTokenAuthenticationResponse>`
pub type AuthenticationTokenType = dyn ConnectorIntegration<
    AccessTokenAuthentication,
    AccessTokenAuthenticationRequestData,
    AccessTokenAuthenticationResponse,
>;
/// Type alias for `ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>`
pub type RefreshTokenType =
    dyn ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken>;

/// Type alias for `ConnectorIntegration<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>`
pub type AcceptDisputeType =
    dyn ConnectorIntegration<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>;
/// Type alias for `ConnectorIntegration<VerifyWebhookSource, VerifyWebhookSourceRequestData, VerifyWebhookSourceResponseData>`
pub type VerifyWebhookSourceType = dyn ConnectorIntegration<
    VerifyWebhookSource,
    VerifyWebhookSourceRequestData,
    VerifyWebhookSourceResponseData,
>;

/// Type alias for `ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>`
pub type SubmitEvidenceType =
    dyn ConnectorIntegration<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>;

/// Type alias for `ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse>`
pub type UploadFileType =
    dyn ConnectorIntegration<Upload, UploadFileRequestData, UploadFileResponse>;

/// Type alias for `ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>`
pub type RetrieveFileType =
    dyn ConnectorIntegration<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>;

/// Type alias for `ConnectorIntegration<Defend, DefendDisputeRequestData, DefendDisputeResponse>`
pub type DefendDisputeType =
    dyn ConnectorIntegration<Defend, DefendDisputeRequestData, DefendDisputeResponse>;

/// Type alias for `ConnectorIntegration<Fetch, FetchDisputesRequestData, FetchDisputesResponse>`
pub type FetchDisputesType =
    dyn ConnectorIntegration<Fetch, FetchDisputesRequestData, FetchDisputesResponse>;

/// Type alias for `ConnectorIntegration<Dsync, DisputeSyncData, DisputeSyncResponse>`
pub type DisputeSyncType = dyn ConnectorIntegration<Dsync, DisputeSyncData, DisputeSyncResponse>;

/// Type alias for `ConnectorIntegration<PreAuthenticate, UasPreAuthenticationRequestData, UasAuthenticationResponseData>`
pub type UasPreAuthenticationType = dyn ConnectorIntegration<
    PreAuthenticate,
    UasPreAuthenticationRequestData,
    UasAuthenticationResponseData,
>;

/// Type alias for `ConnectorIntegration<PostAuthenticate, UasPostAuthenticationRequestData, UasAuthenticationResponseData>`
pub type UasPostAuthenticationType = dyn ConnectorIntegration<
    PostAuthenticate,
    UasPostAuthenticationRequestData,
    UasAuthenticationResponseData,
>;

/// Type alias for `ConnectorIntegration<Confirmation, UasConfirmationRequestData, UasAuthenticationResponseData>`
pub type UasAuthenticationConfirmationType = dyn ConnectorIntegration<
    AuthenticationConfirmation,
    UasConfirmationRequestData,
    UasAuthenticationResponseData,
>;

/// Type alias for `ConnectorIntegration<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>`
pub type UasAuthenticationType = dyn ConnectorIntegration<
    Authenticate,
    UasAuthenticationRequestData,
    UasAuthenticationResponseData,
>;

/// Type alias for `ConnectorIntegration<RecoveryRecordBack, RevenueRecoveryRecordBackRequest, RevenueRecoveryRecordBackResponse>`
pub type RevenueRecoveryRecordBackType = dyn ConnectorIntegration<
    RecoveryRecordBack,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>;

/// Type alias for `ConnectorIntegration<BillingConnectorPaymentsSync, BillingConnectorPaymentsSyncRequest, BillingConnectorPaymentsSyncResponse>`
pub type BillingConnectorPaymentsSyncType = dyn ConnectorIntegration<
    BillingConnectorPaymentsSync,
    BillingConnectorPaymentsSyncRequest,
    BillingConnectorPaymentsSyncResponse,
>;

/// Type alias for `ConnectorIntegration<BillingConnectorInvoiceSync, BillingConnectorInvoiceSyncRequest, BillingConnectorInvoiceSyncResponse>`
pub type BillingConnectorInvoiceSyncType = dyn ConnectorIntegration<
    BillingConnectorInvoiceSync,
    BillingConnectorInvoiceSyncRequest,
    BillingConnectorInvoiceSyncResponse,
>;

/// Type alias for `ConnectorIntegrationV2<RecoveryRecordBack, RevenueRecoveryRecordBackData, RevenueRecoveryRecordBackRequest, RevenueRecoveryRecordBackResponse>`
pub type RevenueRecoveryRecordBackTypeV2 = dyn ConnectorIntegrationV2<
    RecoveryRecordBack,
    flow_common_types::RevenueRecoveryRecordBackData,
    RevenueRecoveryRecordBackRequest,
    RevenueRecoveryRecordBackResponse,
>;

/// Type alias for `ConnectorIntegrationV2<BillingConnectorPaymentsSync, BillingConnectorPaymentsSyncRequest, BillingConnectorPaymentsSyncResponse>`
pub type BillingConnectorPaymentsSyncTypeV2 = dyn ConnectorIntegrationV2<
    BillingConnectorPaymentsSync,
    flow_common_types::BillingConnectorPaymentsSyncFlowData,
    BillingConnectorPaymentsSyncRequest,
    BillingConnectorPaymentsSyncResponse,
>;

/// Type alias for `ConnectorIntegrationV2<BillingConnectorInvoiceSync, BillingConnectorInvoiceSyncFlowData, BillingConnectorInvoiceSyncRequest, BillingConnectorInvoiceSyncResponse>`
pub type BillingConnectorInvoiceSyncTypeV2 = dyn ConnectorIntegrationV2<
    BillingConnectorInvoiceSync,
    flow_common_types::BillingConnectorInvoiceSyncFlowData,
    BillingConnectorInvoiceSyncRequest,
    BillingConnectorInvoiceSyncResponse,
>;

/// Type alias for `ConnectorIntegration<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>`
pub type ExternalVaultInsertType =
    dyn ConnectorIntegration<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>;
/// Type alias for `ConnectorIntegration<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>`
pub type ExternalVaultRetrieveType =
    dyn ConnectorIntegration<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>;
/// Type alias for `ConnectorIntegration<ExternalVaultDeleteFlow, VaultRequestData, VaultResponseData>`
pub type ExternalVaultDeleteType =
    dyn ConnectorIntegration<ExternalVaultDeleteFlow, VaultRequestData, VaultResponseData>;
/// Type alias for `ConnectorIntegration<ExternalVaultCreateFlow, VaultRequestData, VaultResponseData>`
pub type ExternalVaultCreateType =
    dyn ConnectorIntegration<ExternalVaultCreateFlow, VaultRequestData, VaultResponseData>;

/// Proxy configuration structure
#[derive(Debug, serde::Deserialize, Clone)]
#[serde(default)]
pub struct Proxy {
    /// The URL of the HTTP proxy server.
    pub http_url: Option<String>,

    /// The URL of the HTTPS proxy server.
    pub https_url: Option<String>,

    /// The timeout duration (in seconds) for idle connections in the proxy pool.
    pub idle_pool_connection_timeout: Option<u64>,

    /// A comma-separated list of hosts that should bypass the proxy.
    pub bypass_proxy_hosts: Option<String>,
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            http_url: Default::default(),
            https_url: Default::default(),
            idle_pool_connection_timeout: Some(90),
            bypass_proxy_hosts: Default::default(),
        }
    }
}
