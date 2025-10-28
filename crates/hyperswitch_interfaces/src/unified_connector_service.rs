use common_enums::AttemptStatus;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_response_types::PaymentsResponseData,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::helpers::ForeignTryFrom;

/// Unified Connector Service (UCS) related transformers
pub mod transformers;

/// UCS trait-based architecture for GRPC flows
pub mod ucs_traits;

pub use transformers::WebhookTransformData;
pub use ucs_traits::{
    UcsContext, UcsExecutionContextProvider, UcsFlowExecutor, UcsGrpcExecutor,
    UcsRequestTransformer, UcsResponseHandler, UcsStateProvider,
};

/// Type alias for return type used by unified connector service response handlers
type UnifiedConnectorServiceResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
    ),
    transformers::UnifiedConnectorServiceError,
>;

#[allow(missing_docs)]
pub fn handle_unified_connector_service_response_for_payment_get(
    response: payments_grpc::PaymentServiceGetResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}
