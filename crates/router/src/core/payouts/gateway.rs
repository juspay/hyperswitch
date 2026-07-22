pub mod cancel_gateway;
pub mod context;
pub mod create_gateway;
pub mod fulfill_gateway;
pub mod quote_gateway;
pub mod recipient_account_gateway;
pub mod recipient_gateway;
pub mod sync_gateway;
use common_utils::errors::ErrorSwitch;
use error_stack::Report;
use hyperswitch_interfaces::{
    errors::ConnectorError, unified_connector_service::transformers::UnifiedConnectorServiceError,
};

/// Converts a `Report<UnifiedConnectorServiceError>` into a `Report<ConnectorError>`.
///
/// This preserves connector HTTP errors returned through UCS as structured
/// connector failures instead of collapsing them to generic HS 500s.
pub(crate) fn convert_ucs_error_to_connector_error(
    report: Report<UnifiedConnectorServiceError>,
) -> Report<ConnectorError> {
    let connector_error = report.current_context().switch();
    report.change_context(connector_error)
}
