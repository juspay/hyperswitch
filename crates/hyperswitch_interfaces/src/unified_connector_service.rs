use crate::helpers::ForeignTryFrom;
use crate::types::merchant_context::MerchantContext;
use async_trait::async_trait;
use common_enums::AttemptStatus;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::{ErrorResponse, RouterData},
    router_flow_types::refunds,
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
