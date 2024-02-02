use api_models::analytics::api_event::{ApiEventDimensions, ApiEventFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for ApiEventFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
        /// Sets filter clauses for status code, flow type, and API flow on the given QueryBuilder.
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.status_code.is_empty() {
            builder
                .add_filter_in_range_clause(ApiEventDimensions::StatusCode, &self.status_code)
                .attach_printable("Error adding status_code filter")?;
        }
        if !self.flow_type.is_empty() {
            builder
                .add_filter_in_range_clause(ApiEventDimensions::FlowType, &self.flow_type)
                .attach_printable("Error adding flow_type filter")?;
        }
        if !self.api_flow.is_empty() {
            builder
                .add_filter_in_range_clause(ApiEventDimensions::ApiFlow, &self.api_flow)
                .attach_printable("Error adding api_name filter")?;
        }

        Ok(())
    }
}
