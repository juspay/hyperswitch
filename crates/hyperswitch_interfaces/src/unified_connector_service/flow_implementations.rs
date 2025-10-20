use crate::api_client::ApiClientWrapper;
use crate::types::merchant_context::MerchantContext;
use crate::unified_connector_service::{
    transformers::UnifiedConnectorServiceError, UnifiedConnectorServiceFlow,
    UnifiedConnectorServiceInterface,
};
use async_trait::async_trait;
use common_utils::errors::CustomResult;

use hyperswitch_domain_models::{
    router_data::{AccessToken, AccessTokenAuthenticationResponse, RouterData},
    router_flow_types::{
        access_token_auth::*, authentication::*, dispute::*, files::*, fraud_check::*,
        mandate_revoke::*, payments::*, payouts::*, refunds, revenue_recovery::*, subscriptions::*,
        unified_authentication_service::*, vault::*, webhooks::*,
    },
    router_request_types::{
        authentication::{
            ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData,
            PreAuthNRequestData,
        },
        fraud_check::{
            FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData,
            FraudCheckSaleData, FraudCheckTransactionData,
        },
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCreateRequest,
        },
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        AcceptDisputeRequestData, AccessTokenAuthenticationRequestData, AccessTokenRequestData,
        AuthorizeSessionTokenData, CompleteAuthorizeData, ConnectorCustomerData,
        CreateOrderRequestData, DefendDisputeRequestData, DisputeSyncData,
        ExternalVaultProxyPaymentsData, FetchDisputesRequestData, GiftCardBalanceCheckRequestData,
        MandateRevokeRequestData, PaymentMethodTokenizationData, PaymentsApproveData,
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCancelPostCaptureData,
        PaymentsCaptureData, PaymentsIncrementalAuthorizationData, PaymentsPostProcessingData,
        PaymentsPostSessionTokensData, PaymentsPreProcessingData, PaymentsRejectData,
        PaymentsSessionData, PaymentsSyncData, PaymentsTaxCalculationData,
        PaymentsUpdateMetadataData, PayoutsData, RefundsData, RetrieveFileRequestData,
        SdkPaymentsSessionUpdateData, SetupMandateRequestData, SubmitEvidenceRequestData,
        UploadFileRequestData, VaultRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        fraud_check::FraudCheckResponseData,
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCreateResponse,
        },
        AcceptDisputeResponse, AuthenticationResponseData, DefendDisputeResponse,
        DisputeSyncResponse, FetchDisputesResponse, GiftCardBalanceCheckResponseData,
        MandateRevokeResponseData, PaymentsResponseData, PayoutsResponseData, RefundsResponseData,
        RetrieveFileResponse, SubmitEvidenceResponse, TaxCalculationResponseData,
        UploadFileResponse, VaultResponseData, VerifyWebhookSourceResponseData,
    },
};

// ===== EXISTING REFUND FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, RefundsData, RefundsResponseData> for refunds::RSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, RefundsData, RefundsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, RefundsData, RefundsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("RSync".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, RefundsData, RefundsResponseData> for refunds::Execute {
    async fn execute_ucs_flow(
        ucs_interface: &dyn UnifiedConnectorServiceInterface,
        router_data: &RouterData<Self, RefundsData, RefundsResponseData>,
        merchant_context: Option<&MerchantContext>,
        merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, RefundsData, RefundsResponseData>,
        UnifiedConnectorServiceError,
    > {
        let mut data = router_data.clone();
        ucs_interface
            .refund_execute(
                &mut data,
                merchant_context,
                merchant_connector_account,
                state,
            )
            .await
    }
}

// ===== REVENUE RECOVERY FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, InvoiceRecordBackRequest, InvoiceRecordBackResponse>
    for InvoiceRecordBack
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            InvoiceRecordBackRequest,
            InvoiceRecordBackResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, InvoiceRecordBackRequest, InvoiceRecordBackResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("InvoiceRecordBack".to_string()).into())
    }
}

// ===== PAYMENT FLOWS =====

// Core Payment Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsAuthorizeData, PaymentsResponseData> for Authorize {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Authorize".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsCaptureData, PaymentsResponseData> for Capture {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsCaptureData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsCaptureData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Capture".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsSyncData, PaymentsResponseData> for PSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsSyncData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsSyncData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("PSync".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsCancelData, PaymentsResponseData> for Void {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsCancelData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsCancelData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Void".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsCancelPostCaptureData, PaymentsResponseData>
    for PostCaptureVoid
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PaymentsCancelPostCaptureData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsCancelPostCaptureData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("PostCaptureVoid".to_string()).into())
    }
}

// Advanced Payment Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, AuthorizeSessionTokenData, PaymentsResponseData>
    for AuthorizeSessionToken
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, AuthorizeSessionTokenData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(
            UnifiedConnectorServiceError::NotImplemented("AuthorizeSessionToken".to_string())
                .into(),
        )
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, CompleteAuthorizeData, PaymentsResponseData>
    for CompleteAuthorize
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, CompleteAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, CompleteAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("CompleteAuthorize".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsApproveData, PaymentsResponseData> for Approve {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsApproveData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsApproveData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Approve".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsRejectData, PaymentsResponseData> for Reject {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsRejectData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsRejectData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Reject".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsSessionData, PaymentsResponseData> for Session {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsSessionData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsSessionData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Session".to_string()).into())
    }
}

// Payment Setup & Management

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsAuthorizeData, PaymentsResponseData>
    for InitPayment
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("InitPayment".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, SetupMandateRequestData, PaymentsResponseData>
    for SetupMandate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, SetupMandateRequestData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, SetupMandateRequestData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("SetupMandate".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentMethodTokenizationData, PaymentsResponseData>
    for PaymentMethodToken
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentMethodTokenizationData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("PaymentMethodToken".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, ConnectorCustomerData, PaymentsResponseData>
    for CreateConnectorCustomer
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, ConnectorCustomerData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(
            UnifiedConnectorServiceError::NotImplemented("CreateConnectorCustomer".to_string())
                .into(),
        )
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > for Balance
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, GiftCardBalanceCheckRequestData, GiftCardBalanceCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Balance".to_string()).into())
    }
}

// Processing Flows

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsPreProcessingData, PaymentsResponseData>
    for PreProcessing
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsPreProcessingData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsPreProcessingData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsPostProcessingData, PaymentsResponseData>
    for PostProcessing
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsPostProcessingData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsPostProcessingData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsIncrementalAuthorizationData, PaymentsResponseData>
    for IncrementalAuthorization
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// Intent Management

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsAuthorizeData, PaymentsResponseData>
    for PaymentCreateIntent
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsSyncData, PaymentsResponseData>
    for PaymentGetIntent
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsSyncData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsSyncData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// Specialized Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsTaxCalculationData, TaxCalculationResponseData>
    for CalculateTax
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PaymentsTaxCalculationData,
            TaxCalculationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsTaxCalculationData, TaxCalculationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, SdkPaymentsSessionUpdateData, PaymentsResponseData>
    for SdkSessionUpdate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            SdkPaymentsSessionUpdateData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, SdkPaymentsSessionUpdateData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsPostSessionTokensData, PaymentsResponseData>
    for PostSessionTokens
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PaymentsPostSessionTokensData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsPostSessionTokensData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsAuthorizeData, PaymentsResponseData>
    for PaymentUpdateIntent
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsAuthorizeData, PaymentsResponseData>
    for RecordAttempt
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsUpdateMetadataData, PaymentsResponseData>
    for UpdateMetadata
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsUpdateMetadataData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsUpdateMetadataData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, CreateOrderRequestData, PaymentsResponseData>
    for CreateOrder
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, CreateOrderRequestData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, CreateOrderRequestData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PaymentsSyncData, PaymentsResponseData>
    for PaymentGetListAttempts
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PaymentsSyncData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PaymentsSyncData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, ExternalVaultProxyPaymentsData, PaymentsResponseData>
    for ExternalVaultProxy
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            ExternalVaultProxyPaymentsData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, ExternalVaultProxyPaymentsData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > for GiftCardBalanceCheck
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== PAYOUT FLOWS (FEATURE GATED) =====

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoCancel {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoCreate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoEligibility {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoFulfill {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoQuote {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoRecipient {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoRecipientAccount {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PayoutsData, PayoutsResponseData> for PoSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== DISPUTE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, AcceptDisputeRequestData, AcceptDisputeResponse> for Accept {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, AcceptDisputeRequestData, AcceptDisputeResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, AcceptDisputeRequestData, AcceptDisputeResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, SubmitEvidenceRequestData, SubmitEvidenceResponse>
    for Evidence
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, SubmitEvidenceRequestData, SubmitEvidenceResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, SubmitEvidenceRequestData, SubmitEvidenceResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, DefendDisputeRequestData, DefendDisputeResponse> for Defend {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, DefendDisputeRequestData, DefendDisputeResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, DefendDisputeRequestData, DefendDisputeResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, FetchDisputesRequestData, FetchDisputesResponse> for Fetch {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, FetchDisputesRequestData, FetchDisputesResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, FetchDisputesRequestData, FetchDisputesResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, DisputeSyncData, DisputeSyncResponse> for Dsync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, DisputeSyncData, DisputeSyncResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, DisputeSyncData, DisputeSyncResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== ACCESS TOKEN FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, AccessTokenRequestData, AccessToken> for AccessTokenAuth {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, AccessTokenRequestData, AccessToken>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, AccessTokenRequestData, AccessToken>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        AccessTokenAuthenticationRequestData,
        AccessTokenAuthenticationResponse,
    > for AccessTokenAuthentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            AccessTokenAuthenticationRequestData,
            AccessTokenAuthenticationResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            AccessTokenAuthenticationRequestData,
            AccessTokenAuthenticationResponse,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== AUTHENTICATION FLOWS =====

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        UasPreAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for PreAuthenticate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            UasPreAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, UasPreAuthenticationRequestData, UasAuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        UasPostAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for PostAuthenticate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            UasPostAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            UasPostAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PreAuthNRequestData, AuthenticationResponseData>
    for PreAuthentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PreAuthNRequestData, AuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, PreAuthNRequestData, AuthenticationResponseData>
    for PreAuthenticationVersionCall
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, PreAuthNRequestData, AuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for Authentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            ConnectorAuthenticationRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, ConnectorAuthenticationRequestData, AuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, UasAuthenticationRequestData, UasAuthenticationResponseData>
    for Authenticate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            UasAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, UasAuthenticationRequestData, UasAuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    > for PostAuthentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, UasConfirmationRequestData, UasAuthenticationResponseData>
    for AuthenticationConfirmation
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            UasConfirmationRequestData,
            UasAuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            UasConfirmationRequestData,
            UasAuthenticationResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== FILE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, UploadFileRequestData, UploadFileResponse> for Upload {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, UploadFileRequestData, UploadFileResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, UploadFileRequestData, UploadFileResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, RetrieveFileRequestData, RetrieveFileResponse> for Retrieve {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, RetrieveFileRequestData, RetrieveFileResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, RetrieveFileRequestData, RetrieveFileResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== FRAUD CHECK FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, FraudCheckSaleData, FraudCheckResponseData> for Sale {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, FraudCheckSaleData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, FraudCheckSaleData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, FraudCheckCheckoutData, FraudCheckResponseData>
    for Checkout
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, FraudCheckCheckoutData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, FraudCheckCheckoutData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, FraudCheckTransactionData, FraudCheckResponseData>
    for Transaction
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, FraudCheckTransactionData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, FraudCheckTransactionData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, FraudCheckFulfillmentData, FraudCheckResponseData>
    for Fulfillment
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, FraudCheckFulfillmentData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, FraudCheckFulfillmentData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, FraudCheckRecordReturnData, FraudCheckResponseData>
    for RecordReturn
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, FraudCheckRecordReturnData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, FraudCheckRecordReturnData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== MANDATE REVOKE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, MandateRevokeRequestData, MandateRevokeResponseData>
    for MandateRevoke
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, MandateRevokeRequestData, MandateRevokeResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== VAULT FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, VaultRequestData, VaultResponseData>
    for ExternalVaultInsertFlow
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, VaultRequestData, VaultResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, VaultRequestData, VaultResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, VaultRequestData, VaultResponseData>
    for ExternalVaultRetrieveFlow
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Self, VaultRequestData, VaultResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, VaultRequestData, VaultResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== SUBSCRIPTION FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, SubscriptionCreateRequest, SubscriptionCreateResponse>
    for SubscriptionCreate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            SubscriptionCreateRequest,
            SubscriptionCreateResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, SubscriptionCreateRequest, SubscriptionCreateResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Self, GetSubscriptionPlansRequest, GetSubscriptionPlansResponse>
    for GetSubscriptionPlans
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            GetSubscriptionPlansRequest,
            GetSubscriptionPlansResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Self, GetSubscriptionPlansRequest, GetSubscriptionPlansResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlanPricesResponse,
    > for GetSubscriptionPlanPrices
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlanPricesResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlanPricesResponse,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(
            UnifiedConnectorServiceError::NotImplemented("GetSubscriptionPlanPrices".to_string())
                .into(),
        )
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        GetSubscriptionEstimateRequest,
        GetSubscriptionEstimateResponse,
    > for GetSubscriptionEstimate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            GetSubscriptionEstimateRequest,
            GetSubscriptionEstimateResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            GetSubscriptionEstimateRequest,
            GetSubscriptionEstimateResponse,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== WEBHOOK FLOWS =====

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Self,
        VerifyWebhookSourceRequestData,
        VerifyWebhookSourceResponseData,
    > for VerifyWebhookSource
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Self,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<
            Self,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}
