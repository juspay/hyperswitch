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
impl UnifiedConnectorServiceFlow<refunds::RSync, RefundsData, RefundsResponseData>
    for refunds::RSync
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<refunds::RSync, RefundsData, RefundsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<refunds::RSync, RefundsData, RefundsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("RSync".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<refunds::Execute, RefundsData, RefundsResponseData>
    for refunds::Execute
{
    async fn execute_ucs_flow(
        ucs_interface: &dyn UnifiedConnectorServiceInterface,
        router_data: &RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
        merchant_context: Option<&MerchantContext>,
        merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
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
impl
    UnifiedConnectorServiceFlow<
        InvoiceRecordBack,
        InvoiceRecordBackRequest,
        InvoiceRecordBackResponse,
    > for InvoiceRecordBack
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            InvoiceRecordBack,
            InvoiceRecordBackRequest,
            InvoiceRecordBackResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("InvoiceRecordBack".to_string()).into())
    }
}

// ===== PAYMENT FLOWS =====

// Core Payment Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    for Authorize
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Authorize".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Capture, PaymentsCaptureData, PaymentsResponseData> for Capture {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Capture".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PSync, PaymentsSyncData, PaymentsResponseData> for PSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("PSync".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Void, PaymentsCancelData, PaymentsResponseData> for Void {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Void".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        PostCaptureVoid,
        PaymentsCancelPostCaptureData,
        PaymentsResponseData,
    > for PostCaptureVoid
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PostCaptureVoid,
            PaymentsCancelPostCaptureData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("PostCaptureVoid".to_string()).into())
    }
}

// Advanced Payment Operations

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        AuthorizeSessionToken,
        AuthorizeSessionTokenData,
        PaymentsResponseData,
    > for AuthorizeSessionToken
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            AuthorizeSessionToken,
            AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("AuthorizeSessionToken".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>
    for CompleteAuthorize
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("CompleteAuthorize".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Approve, PaymentsApproveData, PaymentsResponseData> for Approve {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Approve, PaymentsApproveData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Approve, PaymentsApproveData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Approve".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Reject, PaymentsRejectData, PaymentsResponseData> for Reject {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Reject, PaymentsRejectData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Reject, PaymentsRejectData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Reject".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Session, PaymentsSessionData, PaymentsResponseData> for Session {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Session, PaymentsSessionData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Session, PaymentsSessionData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Session".to_string()).into())
    }
}

// Payment Setup & Management

#[async_trait]
impl UnifiedConnectorServiceFlow<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>
    for InitPayment
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("InitPayment".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
    for SetupMandate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("SetupMandate".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        PaymentMethodToken,
        PaymentMethodTokenizationData,
        PaymentsResponseData,
    > for PaymentMethodToken
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("PaymentMethodToken".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        CreateConnectorCustomer,
        ConnectorCustomerData,
        PaymentsResponseData,
    > for CreateConnectorCustomer
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("CreateConnectorCustomer".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Balance,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > for Balance
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Balance,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Balance, GiftCardBalanceCheckRequestData, GiftCardBalanceCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Balance".to_string()).into())
    }
}

// Processing Flows

#[async_trait]
impl UnifiedConnectorServiceFlow<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>
    for PreProcessing
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>
    for PostProcessing
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        IncrementalAuthorization,
        PaymentsIncrementalAuthorizationData,
        PaymentsResponseData,
    > for IncrementalAuthorization
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            IncrementalAuthorization,
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
            IncrementalAuthorization,
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
impl UnifiedConnectorServiceFlow<PaymentCreateIntent, PaymentsAuthorizeData, PaymentsResponseData>
    for PaymentCreateIntent
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentCreateIntent, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PaymentCreateIntent, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentGetIntent, PaymentsSyncData, PaymentsResponseData>
    for PaymentGetIntent
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentGetIntent, PaymentsSyncData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PaymentGetIntent, PaymentsSyncData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// Specialized Operations

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        CalculateTax,
        PaymentsTaxCalculationData,
        TaxCalculationResponseData,
    > for CalculateTax
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            CalculateTax,
            PaymentsTaxCalculationData,
            TaxCalculationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        SdkSessionUpdate,
        SdkPaymentsSessionUpdateData,
        PaymentsResponseData,
    > for SdkSessionUpdate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            SdkSessionUpdate,
            SdkPaymentsSessionUpdateData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        PostSessionTokens,
        PaymentsPostSessionTokensData,
        PaymentsResponseData,
    > for PostSessionTokens
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PostSessionTokens,
            PaymentsPostSessionTokensData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentUpdateIntent, PaymentsAuthorizeData, PaymentsResponseData>
    for PaymentUpdateIntent
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentUpdateIntent, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PaymentUpdateIntent, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<RecordAttempt, PaymentsAuthorizeData, PaymentsResponseData>
    for RecordAttempt
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<RecordAttempt, PaymentsAuthorizeData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<RecordAttempt, PaymentsAuthorizeData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>
    for UpdateMetadata
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<CreateOrder, CreateOrderRequestData, PaymentsResponseData>
    for CreateOrder
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<CreateOrder, CreateOrderRequestData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<CreateOrder, CreateOrderRequestData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentGetListAttempts, PaymentsSyncData, PaymentsResponseData>
    for PaymentGetListAttempts
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentGetListAttempts, PaymentsSyncData, PaymentsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PaymentGetListAttempts, PaymentsSyncData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        ExternalVaultProxy,
        ExternalVaultProxyPaymentsData,
        PaymentsResponseData,
    > for ExternalVaultProxy
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            ExternalVaultProxy,
            ExternalVaultProxyPaymentsData,
            PaymentsResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<ExternalVaultProxy, ExternalVaultProxyPaymentsData, PaymentsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        GiftCardBalanceCheck,
        GiftCardBalanceCheckRequestData,
        GiftCardBalanceCheckResponseData,
    > for GiftCardBalanceCheck
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            GiftCardBalanceCheck,
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
            GiftCardBalanceCheck,
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
impl UnifiedConnectorServiceFlow<PoCancel, PayoutsData, PayoutsResponseData> for PoCancel {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoCancel, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoCancel, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoCreate, PayoutsData, PayoutsResponseData> for PoCreate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoCreate, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoCreate, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoEligibility, PayoutsData, PayoutsResponseData>
    for PoEligibility
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoEligibility, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoEligibility, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoFulfill, PayoutsData, PayoutsResponseData> for PoFulfill {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoFulfill, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoFulfill, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoQuote, PayoutsData, PayoutsResponseData> for PoQuote {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoQuote, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoQuote, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoRecipient, PayoutsData, PayoutsResponseData> for PoRecipient {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoRecipient, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoRecipient, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoRecipientAccount, PayoutsData, PayoutsResponseData>
    for PoRecipientAccount
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoRecipientAccount, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoRecipientAccount, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoSync, PayoutsData, PayoutsResponseData> for PoSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoSync, PayoutsData, PayoutsResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PoSync, PayoutsData, PayoutsResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== DISPUTE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>
    for Accept
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>
    for Evidence
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Defend, DefendDisputeRequestData, DefendDisputeResponse>
    for Defend
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Fetch, FetchDisputesRequestData, FetchDisputesResponse> for Fetch {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Fetch, FetchDisputesRequestData, FetchDisputesResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Fetch, FetchDisputesRequestData, FetchDisputesResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Dsync, DisputeSyncData, DisputeSyncResponse> for Dsync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Dsync, DisputeSyncData, DisputeSyncResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Dsync, DisputeSyncData, DisputeSyncResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== ACCESS TOKEN FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<AccessTokenAuth, AccessTokenRequestData, AccessToken>
    for AccessTokenAuth
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        AccessTokenAuthentication,
        AccessTokenAuthenticationRequestData,
        AccessTokenAuthenticationResponse,
    > for AccessTokenAuthentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            AccessTokenAuthentication,
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
            AccessTokenAuthentication,
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
        PreAuthenticate,
        UasPreAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for PreAuthenticate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PreAuthenticate,
            UasPreAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PreAuthenticate, UasPreAuthenticationRequestData, UasAuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        PostAuthenticate,
        UasPostAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for PostAuthenticate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PostAuthenticate,
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
            PostAuthenticate,
            UasPostAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
    for PreAuthentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PreAuthentication,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        PreAuthenticationVersionCall,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > for PreAuthenticationVersionCall
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PreAuthenticationVersionCall,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<PreAuthenticationVersionCall, PreAuthNRequestData, AuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Authentication,
        ConnectorAuthenticationRequestData,
        AuthenticationResponseData,
    > for Authentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Authentication,
            ConnectorAuthenticationRequestData,
            AuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        Authenticate,
        UasAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for Authenticate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            Authenticate,
            UasAuthenticationRequestData,
            UasAuthenticationResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        PostAuthentication,
        ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    > for PostAuthentication
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            PostAuthentication,
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
            PostAuthentication,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        AuthenticationConfirmation,
        UasConfirmationRequestData,
        UasAuthenticationResponseData,
    > for AuthenticationConfirmation
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            AuthenticationConfirmation,
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
            AuthenticationConfirmation,
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
impl UnifiedConnectorServiceFlow<Upload, UploadFileRequestData, UploadFileResponse> for Upload {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>
    for Retrieve
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== FRAUD CHECK FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Sale, FraudCheckSaleData, FraudCheckResponseData> for Sale {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Sale, FraudCheckSaleData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Sale, FraudCheckSaleData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>
    for Checkout
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Transaction, FraudCheckTransactionData, FraudCheckResponseData>
    for Transaction
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Transaction, FraudCheckTransactionData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Transaction, FraudCheckTransactionData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>
    for Fulfillment
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for RecordReturn
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== MANDATE REVOKE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>
    for MandateRevoke
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            MandateRevoke,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== VAULT FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>
    for ExternalVaultInsertFlow
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>
    for ExternalVaultRetrieveFlow
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

// ===== SUBSCRIPTION FLOWS =====

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        SubscriptionCreate,
        SubscriptionCreateRequest,
        SubscriptionCreateResponse,
    > for SubscriptionCreate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            SubscriptionCreate,
            SubscriptionCreateRequest,
            SubscriptionCreateResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<SubscriptionCreate, SubscriptionCreateRequest, SubscriptionCreateResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        GetSubscriptionPlans,
        GetSubscriptionPlansRequest,
        GetSubscriptionPlansResponse,
    > for GetSubscriptionPlans
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            GetSubscriptionPlans,
            GetSubscriptionPlansRequest,
            GetSubscriptionPlansResponse,
        >,
        _merchant_context: Option<&MerchantContext>,
        _merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        _state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<GetSubscriptionPlans, GetSubscriptionPlansRequest, GetSubscriptionPlansResponse>,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}

#[async_trait]
impl
    UnifiedConnectorServiceFlow<
        GetSubscriptionPlanPrices,
        GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlanPricesResponse,
    > for GetSubscriptionPlanPrices
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            GetSubscriptionPlanPrices,
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
            GetSubscriptionPlanPrices,
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
        GetSubscriptionEstimate,
        GetSubscriptionEstimateRequest,
        GetSubscriptionEstimateResponse,
    > for GetSubscriptionEstimate
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            GetSubscriptionEstimate,
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
            GetSubscriptionEstimate,
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
        VerifyWebhookSource,
        VerifyWebhookSourceRequestData,
        VerifyWebhookSourceResponseData,
    > for VerifyWebhookSource
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<
            VerifyWebhookSource,
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
            VerifyWebhookSource,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
        UnifiedConnectorServiceError,
    > {
        Err(UnifiedConnectorServiceError::NotImplemented("Flow".to_string()).into())
    }
}
