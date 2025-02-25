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
                .attach_printable("Error adding payment method type filter")?;
        }
        if !self.client_source.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::ClientSource, &self.client_source)
                .attach_printable("Error adding client source filter")?;
        }
        if !self.client_version.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::ClientVersion, &self.client_version)
                .attach_printable("Error adding client version filter")?;
        }
        if !self.profile_id.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::ProfileId, &self.profile_id)
                .attach_printable("Error adding profile id filter")?;
        }
        if !self.card_network.is_empty() {
            let card_networks: Vec<String> = self
                .card_network
                .iter()
                .flat_map(|cn| {
                    [
                        format!("\"{cn}\""),
                        cn.to_string(),
                        format!("\"{cn}\"").to_uppercase(),
                    ]
                })
                .collect();
            builder
                .add_filter_in_range_clause(
                    PaymentDimensions::CardNetwork,
                    card_networks.as_slice(),
                )
                .attach_printable("Error adding card network filter")?;
        }
        if !self.merchant_id.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::MerchantId, &self.merchant_id)
                .attach_printable("Error adding merchant id filter")?;
        }
        if !self.card_last_4.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::CardLast4, &self.card_last_4)
                .attach_printable("Error adding card last 4 filter")?;
        }
        if !self.card_issuer.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::CardIssuer, &self.card_issuer)
                .attach_printable("Error adding card issuer filter")?;
        }
        if !self.error_reason.is_empty() {
            builder
                .add_filter_in_range_clause(PaymentDimensions::ErrorReason, &self.error_reason)
                .attach_printable("Error adding error reason filter")?;
        }
        if !self.first_attempt.is_empty() {
            builder
                .add_filter_in_range_clause("first_attempt", &self.first_attempt)
                .attach_printable("Error adding first attempt filter")?;
        }
        Ok(())
    }
}
