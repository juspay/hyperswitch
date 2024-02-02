use api_models::analytics::sdk_events::{SdkEventDimensions, SdkEventFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for SdkEventFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
        /// Sets filter clauses for the given query builder based on the non-empty fields of the current object.
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.payment_method.is_empty() {
            builder
                .add_filter_in_range_clause(SdkEventDimensions::PaymentMethod, &self.payment_method)
                .attach_printable("Error adding payment method filter")?;
        }
        if !self.platform.is_empty() {
            builder
                .add_filter_in_range_clause(SdkEventDimensions::Platform, &self.platform)
                .attach_printable("Error adding platform filter")?;
        }
        if !self.browser_name.is_empty() {
            builder
                .add_filter_in_range_clause(SdkEventDimensions::BrowserName, &self.browser_name)
                .attach_printable("Error adding browser name filter")?;
        }
        if !self.source.is_empty() {
            builder
                .add_filter_in_range_clause(SdkEventDimensions::Source, &self.source)
                .attach_printable("Error adding source filter")?;
        }
        if !self.component.is_empty() {
            builder
                .add_filter_in_range_clause(SdkEventDimensions::Component, &self.component)
                .attach_printable("Error adding component filter")?;
        }
        if !self.payment_experience.is_empty() {
            builder
                .add_filter_in_range_clause(
                    SdkEventDimensions::PaymentExperience,
                    &self.payment_experience,
                )
                .attach_printable("Error adding payment experience filter")?;
        }
        Ok(())
    }
}
