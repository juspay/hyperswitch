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
        if !self.connector.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::Connector, &self.connector)
                .attach_printable("Error adding connector filter")?;
        }
        if !self.auth_type.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::AuthType, &self.auth_type)
                .attach_printable("Error adding auth type filter")?;
        }
        if !self.payment_method.is_empty() {
            builder
                .add_filter_in_range_clause(
                    PaymentIntentDimensions::PaymentMethod,
                    &self.payment_method,
                )
                .attach_printable("Error adding payment method filter")?;
        }
        if !self.payment_method_type.is_empty() {
            builder
                .add_filter_in_range_clause(
                    PaymentIntentDimensions::PaymentMethodType,
                    &self.payment_method_type,
                )
                .attach_printable("Error adding payment method type filter")?;
        }
        if !self.card_network.is_empty() {
            builder
                .add_filter_in_range_clause(
                    PaymentIntentDimensions::CardNetwork,
                    &self.card_network,
                )
                .attach_printable("Error adding card network filter")?;
        }
        if !self.merchant_id.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::MerchantId, &self.merchant_id)
                .attach_printable("Error adding merchant id filter")?;
        }
        if !self.card_last_4.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::CardLast4, &self.card_last_4)
                .attach_printable("Error adding card last 4 filter")?;
        }
        if !self.card_issuer.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentIntentDimensions::CardIssuer, &self.card_issuer)
                .attach_printable("Error adding card issuer filter")?;
        }
        if !self.error_reason.is_empty() {
            builder
                .add_filter_in_range_clause(
                    PaymentIntentDimensions::ErrorReason,
                    &self.error_reason,
                )
                .attach_printable("Error adding error reason filter")?;
        }
        if !self.customer_id.is_empty() {
            builder
                .add_filter_in_range_clause("customer_id", &self.customer_id)
                .attach_printable("Error adding customer id filter")?;
        }
        Ok(())
    }
}
