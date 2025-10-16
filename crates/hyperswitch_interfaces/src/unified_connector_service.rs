use common_enums::AttemptStatus;
use common_utils::{errors::CustomResult, types::MinorUnit};
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_response_types::PaymentsResponseData,
};
use unified_connector_service_client::payments as payments_grpc;

use crate::helpers::ForeignTryFrom;

/// Unified Connector Service (UCS) related transformers
pub mod transformers;

pub use transformers::WebhookTransformData;

/// Fields in RouterData that are updated from the UCS response
#[allow(missing_docs)]
#[derive(Debug)]
pub struct RouterDataUpdate {
    pub amount_captured: Option<i64>,
    pub minor_amount_captured: Option<MinorUnit>,
}

/// Type alias for return type used by unified connector service response handlers
type UnifiedConnectorServiceResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
        RouterDataUpdate,
    ),
    transformers::UnifiedConnectorServiceError,
>;

#[allow(missing_docs)]
pub fn handle_unified_connector_service_response_for_payment_get(
    response: payments_grpc::PaymentServiceGetResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_update = RouterDataUpdate {
        amount_captured: response.captured_amount,
        minor_amount_captured: response.minor_captured_amount.map(MinorUnit::new),
    };

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code, router_data_update))
}
