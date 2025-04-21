use api_models::analytics::auth_events::{AuthEventDimensions, AuthEventFilters};
use error_stack::ResultExt;

use crate::{
    query::{QueryBuilder, QueryFilter, QueryResult, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource},
};

impl<T> QueryFilter<T> for AuthEventFilters
where
    T: AnalyticsDataSource,
    AnalyticsCollection: ToSql<T>,
{
    fn set_filter_clause(&self, builder: &mut QueryBuilder<T>) -> QueryResult<()> {
        if !self.authentication_status.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::AuthenticationStatus,
                    &self.authentication_status,
                )
                .attach_printable("Error adding authentication status filter")?;
        }

        if !self.trans_status.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::TransactionStatus,
                    &self.trans_status,
                )
                .attach_printable("Error adding transaction status filter")?;
        }

        if !self.error_message.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::ErrorMessage, &self.error_message)
                .attach_printable("Error adding error message filter")?;
        }

        if !self.authentication_connector.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::AuthenticationConnector,
                    &self.authentication_connector,
                )
                .attach_printable("Error adding authentication connector filter")?;
        }

        if !self.message_version.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::MessageVersion,
                    &self.message_version,
                )
                .attach_printable("Error adding message version filter")?;
        }

        if !self.acs_reference_number.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::AcsReferenceNumber,
                    &self.acs_reference_number,
                )
                .attach_printable("Error adding acs reference number filter")?;
        }
        Ok(())
    }
}
