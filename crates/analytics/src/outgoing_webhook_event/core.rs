use api_models::analytics::outgoing_webhook_event::OutgoingWebhookLogsRequest;
use common_utils::errors::ReportSwitchExt;
use error_stack::{IntoReport, ResultExt};

use super::events::{get_outgoing_webhook_event, OutgoingWebhookLogsResult};
use crate::{errors::AnalyticsResult, types::FiltersError, AnalyticsProvider};

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
