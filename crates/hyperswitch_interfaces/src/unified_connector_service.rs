use crate::helpers::ForeignTryFrom;
use crate::types::merchant_context::MerchantContext;
use async_trait::async_trait;
use common_enums::AttemptStatus;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::{ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::*, dispute::*, files::*, fraud_check::*, mandate_revoke::*, payments::*,
        payouts::*, refunds, webhooks::*,
    },
    router_request_types::*,
    router_response_types::*,
};
use unified_connector_service_client::payments::{
    self as payments_grpc, PaymentServiceGetResponse,
};
/// Flow-specific implementations for UCS mapping
pub mod flow_implementations;
/// Unified Connector Service (UCS) related transformers
pub mod transformers;

pub use transformers::WebhookTransformData;

/// Type alias for return type used by unified connector service response handlers
type UnifiedConnectorServiceResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
    ),
    transformers::UnifiedConnectorServiceError,
>;
use crate::api_client::ApiClientWrapper;

#[allow(missing_docs)]
pub fn handle_unified_connector_service_response_for_payment_get(
    response: PaymentServiceGetResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let _router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((_router_data_response, status_code))
}

type UnifiedConnectorServiceRefundResult = CustomResult<
    (Result<RefundsResponseData, ErrorResponse>, u16),
    transformers::UnifiedConnectorServiceError,
>;
#[allow(missing_docs)]
pub fn handle_unified_connector_service_response_for_refund_execute(
    response: payments_grpc::RefundResponse,
) -> UnifiedConnectorServiceRefundResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let _router_data_response: Result<RefundsResponseData, ErrorResponse> =
        Result::<RefundsResponseData, ErrorResponse>::foreign_try_from(response)?;

    Ok((_router_data_response, status_code))
}

#[async_trait]
#[allow(missing_docs)]
pub trait UnifiedConnectorServiceInterface: Send + Sync {
    /// Performs Payment Authorization
    async fn payment_authorize(
        &self,
        _router_data: &mut RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    // ===== PAYMENT FLOWS =====

    async fn payment_authorize_session_token(
        &self,
        _router_data: &mut RouterData<
            AuthorizeSessionToken,
            AuthorizeSessionTokenData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_complete_authorize(
        &self,
        _fraud_check_router_data: &mut RouterData<
            CompleteAuthorize,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_approve(
        &self,
        _router_data: &mut RouterData<Approve, PaymentsApproveData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_balance(
        &self,
        _router_data: &mut RouterData<
            Balance,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_init_payment(
        &self,
        _router_data: &mut RouterData<InitPayment, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_capture(
        &self,
        _router_data: &mut RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_sync(
        &self,
        _router_data: &mut RouterData<PSync, PaymentsSyncData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_void(
        &self,
        _router_data: &mut RouterData<Void, PaymentsCancelData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_post_capture_void(
        &self,
        _router_data: &mut RouterData<
            PostCaptureVoid,
            PaymentsCancelPostCaptureData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_reject(
        &self,
        _router_data: &mut RouterData<Reject, PaymentsRejectData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_session(
        &self,
        _router_data: &mut RouterData<Session, PaymentsSessionData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_method_token(
        &self,
        _router_data: &mut RouterData<
            PaymentMethodToken,
            PaymentMethodTokenizationData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_create_connector_customer(
        &self,
        _router_data: &mut RouterData<
            CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_setup_mandate(
        &self,
        _router_data: &mut RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_pre_processing(
        &self,
        _router_data: &mut RouterData<
            PreProcessing,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_incremental_authorization(
        &self,
        _router_data: &mut RouterData<
            IncrementalAuthorization,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_post_processing(
        &self,
        _router_data: &mut RouterData<
            PostProcessing,
            PaymentsPostProcessingData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_calculate_tax(
        &self,
        _router_data: &mut RouterData<
            CalculateTax,
            PaymentsTaxCalculationData,
            TaxCalculationResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_sdk_session_update(
        &self,
        _router_data: &mut RouterData<
            SdkSessionUpdate,
            SdkPaymentsSessionUpdateData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_create_intent(
        &self,
        _router_data: &mut RouterData<
            PaymentCreateIntent,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_get_intent(
        &self,
        _router_data: &mut RouterData<PaymentGetIntent, PaymentsSyncData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_update_intent(
        &self,
        _router_data: &mut RouterData<
            PaymentUpdateIntent,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_post_session_tokens(
        &self,
        _router_data: &mut RouterData<
            PostSessionTokens,
            PaymentsPostSessionTokensData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_record_attempt(
        &self,
        _router_data: &mut RouterData<RecordAttempt, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_update_metadata(
        &self,
        _router_data: &mut RouterData<
            UpdateMetadata,
            PaymentsUpdateMetadataData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_create_order(
        &self,
        _router_data: &mut RouterData<CreateOrder, CreateOrderRequestData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn payment_get_list_attempts(
        &self,
        _router_data: &mut RouterData<
            PaymentGetListAttempts,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_external_vault_proxy(
        &self,
        _router_data: &mut RouterData<
            ExternalVaultProxy,
            ExternalVaultProxyPaymentsData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    async fn payment_gift_card_balance_check(
        &self,
        _router_data: &mut RouterData<
            GiftCardBalanceCheck,
            GiftCardBalanceCheckRequestData,
            GiftCardBalanceCheckResponseData,
        >,
    ) {
        todo!()
    }

    // ===== REFUND FLOWS =====

    async fn refund_sync(
        &self,
        _router_data: &mut RouterData<refunds::RSync, RefundsData, RefundsResponseData>,
    ) {
        todo!()
    }

    async fn refund_execute(
        &self,
        router_data: &mut RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
        merchant_context: Option<&MerchantContext>,
        merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        state: &dyn ApiClientWrapper,
    ) -> CustomResult<
        RouterData<refunds::Execute, RefundsData, RefundsResponseData>,
        transformers::UnifiedConnectorServiceError,
    >;

    // ===== PAYOUT FLOWS =====

    #[cfg(feature = "payouts")]
    async fn payout_cancel(
        &self,
        _router_data: &mut RouterData<PoCancel, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_create(
        &self,
        _router_data: &mut RouterData<PoCreate, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_eligibility(
        &self,
        _router_data: &mut RouterData<PoEligibility, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_fulfill(
        &self,
        _router_data: &mut RouterData<PoFulfill, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_quote(
        &self,
        _router_data: &mut RouterData<PoQuote, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_recipient(
        &self,
        _router_data: &mut RouterData<PoRecipient, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_recipient_account(
        &self,
        _router_data: &mut RouterData<PoRecipientAccount, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn payout_sync(
        &self,
        _router_data: &mut RouterData<PoSync, PayoutsData, PayoutsResponseData>,
    ) {
        todo!()
    }

    // ===== DISPUTE FLOWS =====

    async fn dispute_accept(
        &self,
        _router_data: &mut RouterData<Accept, AcceptDisputeRequestData, AcceptDisputeResponse>,
    ) {
        todo!()
    }

    async fn dispute_evidence(
        &self,
        _router_data: &mut RouterData<Evidence, SubmitEvidenceRequestData, SubmitEvidenceResponse>,
    ) {
        todo!()
    }

    async fn dispute_defend(
        &self,
        _router_data: &mut RouterData<Defend, DefendDisputeRequestData, DefendDisputeResponse>,
    ) {
        todo!()
    }

    async fn dispute_fetch(
        &self,
        _router_data: &mut RouterData<Fetch, FetchDisputesRequestData, FetchDisputesResponse>,
    ) {
        todo!()
    }

    async fn dispute_sync(
        &self,
        _router_data: &mut RouterData<Dsync, DisputeSyncData, DisputeSyncResponse>,
    ) {
        todo!()
    }

    // ===== ACCESS TOKEN FLOWS =====

    async fn access_token_auth(
        &self,
        _router_data: &mut RouterData<
            AccessTokenAuth,
            AccessTokenAuthenticationRequestData,
            PaymentsResponseData,
        >,
    ) {
        todo!()
    }

    // ===== FILE FLOWS =====

    async fn file_upload(
        &self,
        _router_data: &mut RouterData<Upload, UploadFileRequestData, UploadFileResponse>,
    ) {
        todo!()
    }

    async fn file_retrieve(
        &self,
        _router_data: &mut RouterData<Retrieve, RetrieveFileRequestData, RetrieveFileResponse>,
    ) {
        todo!()
    }

    // ===== FRAUD CHECK FLOWS =====

    async fn fraud_check_sale(
        &self,
        _router_data: &mut RouterData<Sale, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn fraud_check_checkout(
        &self,
        _router_data: &mut RouterData<Checkout, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn fraud_check_transaction(
        &self,
        _router_data: &mut RouterData<Transaction, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn fraud_check_fulfillment(
        &self,
        _router_data: &mut RouterData<Fulfillment, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    async fn fraud_check_record_return(
        &self,
        _router_data: &mut RouterData<RecordReturn, PaymentsAuthorizeData, PaymentsResponseData>,
    ) {
        todo!()
    }

    // ===== MANDATE REVOKE FLOWS =====

    async fn mandate_revoke(
        &self,
        _router_data: &mut RouterData<
            MandateRevoke,
            MandateRevokeRequestData,
            MandateRevokeResponseData,
        >,
    ) {
        todo!()
    }

    // ===== WEBHOOK FLOWS =====

    async fn webhook_verify_source(
        &self,
        _router_data: &mut RouterData<
            VerifyWebhookSource,
            VerifyWebhookSourceRequestData,
            VerifyWebhookSourceResponseData,
        >,
    ) {
        todo!()
    }
}

/// Trait that enforces RouterData flows to implement UCS method mapping
///
/// This trait must be implemented by any RouterData flow type that is used with
/// `execute_connector_processing_step`. It defines how the flow maps to the
/// appropriate UnifiedConnectorServiceInterface method.
#[async_trait]
pub trait UnifiedConnectorServiceFlow<T, Req, Resp>: Send + Sync
where
    Req: std::fmt::Debug + Clone + Send + Sync + 'static,
    Resp: std::fmt::Debug + Clone + Send + Sync + 'static,
{
    /// Execute the appropriate UCS method for this flow type
    async fn execute_ucs_flow(
        ucs_interface: &dyn UnifiedConnectorServiceInterface,
        _router_data: &RouterData<T, Req, Resp>,
        merchant_context: Option<&MerchantContext>,
        merchant_connector_account: Option<
            &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
        >,
        state: &dyn ApiClientWrapper,
    ) -> CustomResult<RouterData<T, Req, Resp>, transformers::UnifiedConnectorServiceError>;
}
