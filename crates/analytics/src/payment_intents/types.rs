use api_models::analytics::payment_intents::{PaymentIntentDimensions, PaymentIntentFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for PaymentIntentFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.status.is_empty() {
            builder
                .add_filter_in_range_clause(
                    PaymentIntentDimensions::PaymentIntentStatus,
                    &self.status,
                )
                .attach_printable("Error adding payment intent status filter")?;
        }
        if !self.currency.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::Currency, &self.currency)
                .attach_printable("Error adding currency filter")?;
        }
        if !self.profile_id.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::ProfileId, &self.profile_id)
                .attach_printable("Error adding profile id filter")?;
        }
        Ok(())
    }
}
