use api_models::analytics::disputes::{DisputeDimensions, DisputeFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for DisputeFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.connector.is_empty() {
            builder
                .add_filter_in_range_clause(DisputeDimensions::Connector, &self.connector)
                .attach_printable("Error adding connector filter")?;
        }

        if !self.dispute_stage.is_empty() {
            builder
                .add_filter_in_range_clause(DisputeDimensions::DisputeStage, &self.dispute_stage)
                .attach_printable("Error adding dispute stage filter")?;
        }

        Ok(())
    }
}
