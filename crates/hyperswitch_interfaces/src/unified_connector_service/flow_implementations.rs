use async_trait::async_trait;
use common_utils::errors::CustomResult;

use crate::unified_connector_service::{
    transformers::UnifiedConnectorServiceError, UcsConnectorAuthMetadata, UcsHeaders,
    UnifiedConnectorServiceFlow, UnifiedConnectorServiceInterface,
};

use hyperswitch_domain_models::{
    router_data::{AccessToken, AccessTokenAuthenticationResponse, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::*, authentication::*, dispute::*, files::*, fraud_check::*, 
        mandate_revoke::*, payments::*, payouts::*, refunds, revenue_recovery::*,
        subscriptions::*, unified_authentication_service::*, vault::*, webhooks::*,
    },
    router_request_types::{
        AcceptDisputeRequestData, AccessTokenAuthenticationRequestData, AccessTokenRequestData,
        AuthorizeSessionTokenData, CompleteAuthorizeData, ConnectorCustomerData, CreateOrderRequestData,
        DefendDisputeRequestData, DisputeSyncData, ExternalVaultProxyPaymentsData, FetchDisputesRequestData,
        GiftCardBalanceCheckRequestData, MandateRevokeRequestData, PaymentsApproveData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCancelPostCaptureData, PaymentsCaptureData, PaymentsIncrementalAuthorizationData,
        PaymentsPostProcessingData, PaymentsPostSessionTokensData, PaymentsPreProcessingData,
        PaymentsRejectData, PaymentsSessionData, PaymentsSyncData, PaymentsTaxCalculationData,
        PaymentsUpdateMetadataData, PayoutsData, RefundsData, RetrieveFileRequestData,
        SdkPaymentsSessionUpdateData, SetupMandateRequestData, SubmitEvidenceRequestData,
        UploadFileRequestData, VaultRequestData, VerifyWebhookSourceRequestData, PaymentMethodTokenizationData,
        authentication::{ConnectorAuthenticationRequestData, ConnectorPostAuthenticationRequestData, PreAuthNRequestData},
        fraud_check::{FraudCheckCheckoutData, FraudCheckFulfillmentData, FraudCheckRecordReturnData, 
                      FraudCheckSaleData, FraudCheckTransactionData},
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{SubscriptionCreateRequest, GetSubscriptionPlansRequest, 
                       GetSubscriptionPlanPricesRequest, GetSubscriptionEstimateRequest},
        unified_authentication_service::{UasPreAuthenticationRequestData, UasPostAuthenticationRequestData, UasAuthenticationRequestData, UasAuthenticationResponseData, UasConfirmationRequestData},
    },
    router_response_types::{
        AcceptDisputeResponse, AuthenticationResponseData, DefendDisputeResponse, DisputeSyncResponse, FetchDisputesResponse,
        GiftCardBalanceCheckResponseData, MandateRevokeResponseData, PaymentsResponseData,
        PayoutsResponseData, RefundsResponseData, RetrieveFileResponse, SubmitEvidenceResponse,
        TaxCalculationResponseData, UploadFileResponse, VaultResponseData, VerifyWebhookSourceResponseData,
        fraud_check::FraudCheckResponseData,
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{SubscriptionCreateResponse, GetSubscriptionPlansResponse,
                       GetSubscriptionPlanPricesResponse, GetSubscriptionEstimateResponse},
    },
};

// ===== EXISTING REFUND FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<refunds::RSync, RefundsData, RefundsResponseData>
    for refunds::RSync
{
    async fn execute_ucs_flow(
        ucs_interface: &dyn UnifiedConnectorServiceInterface,
        router_data: &RouterData<refunds::RSync, RefundsData, RefundsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        let mut data = router_data.clone();
        let response = ucs_interface.refund_sync(&mut data).await;
        // let refunds_response = RefundsResponseData {
        //     // fill fields from `response`
        //     ..response
        // };
        // Transform response appropriately
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<refunds::Execute, RefundsData, RefundsResponseData>
    for refunds::Execute
{
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== REVENUE RECOVERY FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse> for InvoiceRecordBack {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<InvoiceRecordBack, InvoiceRecordBackRequest, InvoiceRecordBackResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== PAYMENT FLOWS =====

// Core Payment Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Authorize {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Capture, PaymentsCaptureData, PaymentsResponseData> for Capture {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PSync, PaymentsSyncData, PaymentsResponseData> for PSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Void, PaymentsCancelData, PaymentsResponseData> for Void {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData> for PostCaptureVoid {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PostCaptureVoid, PaymentsCancelPostCaptureData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// Advanced Payment Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData> for AuthorizeSessionToken {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<AuthorizeSessionToken, AuthorizeSessionTokenData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData> for CompleteAuthorize {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Approve, PaymentsApproveData, PaymentsResponseData> for Approve {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Approve, PaymentsApproveData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Reject, PaymentsRejectData, PaymentsResponseData> for Reject {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Reject, PaymentsRejectData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Session, PaymentsSessionData, PaymentsResponseData> for Session {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Session, PaymentsSessionData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// Payment Setup & Management

#[async_trait]
impl UnifiedConnectorServiceFlow<InitPayment, PaymentsAuthorizeData, PaymentsResponseData> for InitPayment {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<SetupMandate, SetupMandateRequestData, PaymentsResponseData> for SetupMandate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData> for PaymentMethodToken {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData> for CreateConnectorCustomer {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<CreateConnectorCustomer, ConnectorCustomerData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Balance, GiftCardBalanceCheckRequestData, GiftCardBalanceCheckResponseData> for Balance {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Balance, GiftCardBalanceCheckRequestData, GiftCardBalanceCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// Processing Flows

#[async_trait]
impl UnifiedConnectorServiceFlow<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData> for PreProcessing {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData> for PostProcessing {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PostProcessing, PaymentsPostProcessingData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<IncrementalAuthorization, PaymentsIncrementalAuthorizationData, PaymentsResponseData> for IncrementalAuthorization {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<IncrementalAuthorization, PaymentsIncrementalAuthorizationData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// Intent Management

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentCreateIntent, PaymentsAuthorizeData, PaymentsResponseData> for PaymentCreateIntent {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentCreateIntent, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentGetIntent, PaymentsSyncData, PaymentsResponseData> for PaymentGetIntent {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentGetIntent, PaymentsSyncData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentUpdateIntent, PaymentsAuthorizeData, PaymentsResponseData> for PaymentUpdateIntent {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentUpdateIntent, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// Specialized Operations

#[async_trait]
impl UnifiedConnectorServiceFlow<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData> for CalculateTax {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<CalculateTax, PaymentsTaxCalculationData, TaxCalculationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData> for SdkSessionUpdate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<SdkSessionUpdate, SdkPaymentsSessionUpdateData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData> for PostSessionTokens {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PostSessionTokens, PaymentsPostSessionTokensData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<RecordAttempt, PaymentsAuthorizeData, PaymentsResponseData> for RecordAttempt {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<RecordAttempt, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData> for UpdateMetadata {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<UpdateMetadata, PaymentsUpdateMetadataData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<CreateOrder, CreateOrderRequestData, PaymentsResponseData> for CreateOrder {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<CreateOrder, CreateOrderRequestData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PaymentGetListAttempts, PaymentsSyncData, PaymentsResponseData> for PaymentGetListAttempts {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PaymentGetListAttempts, PaymentsSyncData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<ExternalVaultProxy, ExternalVaultProxyPaymentsData, PaymentsResponseData> for ExternalVaultProxy {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<ExternalVaultProxy, ExternalVaultProxyPaymentsData, PaymentsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<GiftCardBalanceCheck, GiftCardBalanceCheckRequestData, GiftCardBalanceCheckResponseData> for GiftCardBalanceCheck {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<GiftCardBalanceCheck, GiftCardBalanceCheckRequestData, GiftCardBalanceCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== PAYOUT FLOWS (FEATURE GATED) =====

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoCancel, PayoutsData, PayoutsResponseData> for PoCancel {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoCancel, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoCreate, PayoutsData, PayoutsResponseData> for PoCreate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoCreate, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoEligibility, PayoutsData, PayoutsResponseData> for PoEligibility {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoEligibility, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoFulfill, PayoutsData, PayoutsResponseData> for PoFulfill {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoFulfill, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoQuote, PayoutsData, PayoutsResponseData> for PoQuote {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoQuote, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoRecipient, PayoutsData, PayoutsResponseData> for PoRecipient {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoRecipient, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoRecipientAccount, PayoutsData, PayoutsResponseData> for PoRecipientAccount {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoRecipientAccount, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[cfg(feature = "payouts")]
#[async_trait]
impl UnifiedConnectorServiceFlow<PoSync, PayoutsData, PayoutsResponseData> for PoSync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PoSync, PayoutsData, PayoutsResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== DISPUTE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Accept, AcceptDisputeRequestData, AcceptDisputeResponse> for Accept {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse> for Evidence {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Defend, DefendDisputeRequestData, DefendDisputeResponse> for Defend {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Fetch, FetchDisputesRequestData, FetchDisputesResponse> for Fetch {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Fetch, FetchDisputesRequestData, FetchDisputesResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Dsync, DisputeSyncData, DisputeSyncResponse> for Dsync {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Dsync, DisputeSyncData, DisputeSyncResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== ACCESS TOKEN FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<AccessTokenAuth, AccessTokenRequestData, AccessToken> for AccessTokenAuth {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<AccessTokenAuthentication, AccessTokenAuthenticationRequestData, AccessTokenAuthenticationResponse> for AccessTokenAuthentication {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<AccessTokenAuthentication, AccessTokenAuthenticationRequestData, AccessTokenAuthenticationResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== AUTHENTICATION FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<PreAuthenticate, UasPreAuthenticationRequestData, UasAuthenticationResponseData> for PreAuthenticate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PreAuthenticate, UasPreAuthenticationRequestData, UasAuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PostAuthenticate, UasPostAuthenticationRequestData, UasAuthenticationResponseData> for PostAuthenticate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PostAuthenticate, UasPostAuthenticationRequestData, UasAuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData> for PreAuthentication {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PreAuthenticationVersionCall, PreAuthNRequestData, AuthenticationResponseData> for PreAuthenticationVersionCall {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PreAuthenticationVersionCall, PreAuthNRequestData, AuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData> for Authentication {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Authentication, ConnectorAuthenticationRequestData, AuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData> for Authenticate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Authenticate, UasAuthenticationRequestData, UasAuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<PostAuthentication, ConnectorPostAuthenticationRequestData, AuthenticationResponseData> for PostAuthentication {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<PostAuthentication, ConnectorPostAuthenticationRequestData, AuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<AuthenticationConfirmation, UasConfirmationRequestData, UasAuthenticationResponseData> for AuthenticationConfirmation {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<AuthenticationConfirmation, UasConfirmationRequestData, UasAuthenticationResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== FILE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Upload, UploadFileRequestData, UploadFileResponse> for Upload {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Retrieve, RetrieveFileRequestData, RetrieveFileResponse> for Retrieve {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== FRAUD CHECK FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<Sale, FraudCheckSaleData, FraudCheckResponseData> for Sale {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Sale, FraudCheckSaleData, FraudCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Checkout, FraudCheckCheckoutData, FraudCheckResponseData> for Checkout {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Checkout, FraudCheckCheckoutData, FraudCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Transaction, FraudCheckTransactionData, FraudCheckResponseData> for Transaction {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Transaction, FraudCheckTransactionData, FraudCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData> for Fulfillment {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<Fulfillment, FraudCheckFulfillmentData, FraudCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData> for RecordReturn {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== MANDATE REVOKE FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData> for MandateRevoke {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<MandateRevoke, MandateRevokeRequestData, MandateRevokeResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== VAULT FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData> for ExternalVaultInsertFlow {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<ExternalVaultInsertFlow, VaultRequestData, VaultResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData> for ExternalVaultRetrieveFlow {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<ExternalVaultRetrieveFlow, VaultRequestData, VaultResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== SUBSCRIPTION FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<SubscriptionCreate, SubscriptionCreateRequest, SubscriptionCreateResponse> for SubscriptionCreate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<SubscriptionCreate, SubscriptionCreateRequest, SubscriptionCreateResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<GetSubscriptionPlans, GetSubscriptionPlansRequest, GetSubscriptionPlansResponse> for GetSubscriptionPlans {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<GetSubscriptionPlans, GetSubscriptionPlansRequest, GetSubscriptionPlansResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<GetSubscriptionPlanPrices, GetSubscriptionPlanPricesRequest, GetSubscriptionPlanPricesResponse> for GetSubscriptionPlanPrices {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<GetSubscriptionPlanPrices, GetSubscriptionPlanPricesRequest, GetSubscriptionPlanPricesResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

#[async_trait]
impl UnifiedConnectorServiceFlow<GetSubscriptionEstimate, GetSubscriptionEstimateRequest, GetSubscriptionEstimateResponse> for GetSubscriptionEstimate {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<GetSubscriptionEstimate, GetSubscriptionEstimateRequest, GetSubscriptionEstimateResponse>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}

// ===== WEBHOOK FLOWS =====

#[async_trait]
impl UnifiedConnectorServiceFlow<VerifyWebhookSource, VerifyWebhookSourceRequestData, VerifyWebhookSourceResponseData> for VerifyWebhookSource {
    async fn execute_ucs_flow(
        _ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<VerifyWebhookSource, VerifyWebhookSourceRequestData, VerifyWebhookSourceResponseData>,
    ) -> CustomResult<String, UnifiedConnectorServiceError> {
        Err(UnifiedConnectorServiceError::InternalError.into())
    }
}
