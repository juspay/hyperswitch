use api_models::analytics::routing_events::RoutingEventsRequest;
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;

use super::events::{get_routing_events, RoutingEventsResult};
use crate::{errors::AnalyticsResult, types::FiltersError, AnalyticsProvider};

pub async fn routing_events_core(
    pool: &AnalyticsProvider,
    req: RoutingEventsRequest,
    merchant_id: &common_utils::id_type::MerchantId,
) -> AnalyticsResult<Vec<RoutingEventsResult>> {
    let data = match pool {
        AnalyticsProvider::Sqlx(_) => Err(FiltersError::NotImplemented(
            "Connector Events not implemented for SQLX",
        ))
        .attach_printable("SQL Analytics is not implemented for Connector Events"),
        AnalyticsProvider::Clickhouse(ckh_pool)
        | AnalyticsProvider::CombinedSqlx(_, ckh_pool)
        | AnalyticsProvider::CombinedCkh(_, ckh_pool) => {
            get_routing_events(merchant_id, req, ckh_pool).await
        }
    }
    .switch()?;
    Ok(data)
}
