use api_models::analytics::outgoing_webhook_event::OutgoingWebhookLogsRequest;
use common_utils::errors::ReportSwitchExt;
use error_stack::{IntoReport, ResultExt};

use super::events::{get_outgoing_webhook_event, OutgoingWebhookLogsResult};
use crate::{errors::AnalyticsResult, types::FiltersError, AnalyticsProvider};

/// Executes a query to retrieve outgoing webhook event logs from the analytics provider based on the given request and merchant ID.
/// 
/// # Arguments
/// 
/// * `pool` - The analytics provider pool to execute the query on.
/// * `req` - The request containing filters and pagination options for the outgoing webhook event logs.
/// * `merchant_id` - The ID of the merchant for which the outgoing webhook event logs are being retrieved.
/// 
/// # Returns
/// 
/// A `Vec` of `OutgoingWebhookLogsResult` wrapped in a `Result` representing the result of the query execution.
pub async fn outgoing_webhook_events_core(
    pool: &AnalyticsProvider,
    req: OutgoingWebhookLogsRequest,
    merchant_id: String,
) -> AnalyticsResult<Vec<OutgoingWebhookLogsResult>> {
    let data = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "Outgoing Webhook Events Logs not implemented for SQLX",
        ))
        .into_report()
        .attach_printable("SQL Analytics is not implemented for Outgoing Webhook Events"),
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool) => {
            get_outgoing_webhook_event(&merchant_id, req, ckh_pool).await
        }
    }
    .switch()?;
    Ok(data)
}
