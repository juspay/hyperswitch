use api_models::analytics::refunds::{RefundDimensions, RefundFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for RefundFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
        /// Adds filter clauses to the provided QueryBuilder based on the non-empty fields of the RefundRequest object.
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.currency.is_empty() {
            builder
                .add_filter_in_range_clause(RefundDimensions::Currency, &self.currency)
                .attach_printable("Error adding currency filter")?;
        }

        if !self.refund_status.is_empty() {
            builder
                .add_filter_in_range_clause(RefundDimensions::RefundStatus, &self.refund_status)
                .attach_printable("Error adding refund status filter")?;
        }

        if !self.connector.is_empty() {
            builder
                .add_filter_in_range_clause(RefundDimensions::Connector, &self.connector)
                .attach_printable("Error adding connector filter")?;
        }

        if !self.refund_type.is_empty() {
            builder
                .add_filter_in_range_clause(RefundDimensions::RefundType, &self.refund_type)
                .attach_printable("Error adding auth type filter")?;
        }

        Ok(())
    }
}
