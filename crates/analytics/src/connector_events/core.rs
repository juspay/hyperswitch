
use api_models::analytics::connector_events::ConnectorEventsRequest;
use common_utils::errors::ReportSwitchExt;
use error_stack::{IntoReport, ResultExt};

use crate::{
    errors::AnalyticsResult,
    types::FiltersError,
    AnalyticsProvider,
};

use super::events::{get_connector_events, ConnectorEventsResult};

pub async fn connector_events_core(
    pool: &AnalyticsProvider,
    req: ConnectorEventsRequest,
    merchant_id: String,
) -> AnalyticsResult<Vec<ConnectorEventsResult>> {
    let data = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "Connector Events not implemented for SQLX",
        ))
        .into_report()
        .attach_printable("SQL Analytics is not implemented for Connector Events"),
        AnalyticsProvider::Clickhouse(pool) => get_connector_events(&merchant_id, req, pool).await,
        AnalyticsProvider::CombinedSqlx(_sqlx_pool, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_sqlx_pool, ckh_pool) => {
            get_connector_events(&merchant_id, req, ckh_pool).await
        }
    }
    .switch()?;
    Ok(data)
}