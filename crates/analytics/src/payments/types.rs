use api_models::analytics::payments::{PaymentDimensions, PaymentFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for PaymentFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
        /// Adds filter clauses to the query builder based on the non-empty fields of the struct.
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.currency.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::Currency, &self.currency)
                .attach_printable("Error adding currency filter")?;
        }

        if !self.status.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::PaymentStatus, &self.status)
                .attach_printable("Error adding payment status filter")?;
        }

        if !self.connector.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::Connector, &self.connector)
                .attach_printable("Error adding connector filter")?;
        }

        if !self.auth_type.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::AuthType, &self.auth_type)
                .attach_printable("Error adding auth type filter")?;
        }

        if !self.payment_method.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::PaymentMethod, &self.payment_method)
                .attach_printable("Error adding payment method filter")?;
        }

        if !self.payment_method_type.is_empty() {
            builder
                .add_filter_in_range_clause(
                    PaymentDimensions::PaymentMethodType,
                    &self.payment_method_type,
                )
                .attach_printable("Error adding payment method filter")?;
        }
        Ok(())
    }
}
