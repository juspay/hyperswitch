use api_models::analytics::connector_events::ConnectorEventsRequest;
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;

use super::events::{get_connector_events, ConnectorEventsResult};
use crate::{enums::AuthInfo, errors::AnalyticsResult, types::FiltersError, AnalyticsProvider};

pub async fn connector_events_core(
    pool: &AnalyticsProvider,
    req: ConnectorEventsRequest,
    auth: &AuthInfo,
) -> AnalyticsResult<Vec<ConnectorEventsResult>> {
    let data = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "Connector Events not implemented for SQLX",
        ))
        .attach_printable("SQL Analytics is not implemented for Connector Events"),
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool) => {
            get_connector_events(auth, req, ckh_pool).await
        }
    }
    .switch()?;
    Ok(data)
}
