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

        if !self.platform.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::Platform, &self.platform)
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

        if !self.mcc.is_empty() {
            builder.add_filter_in_range_clause(AuthEventDimensions::Mcc, &self.mcc)?;
        }
        if !self.amount.is_empty() {
            builder.add_filter_in_range_clause(AuthEventDimensions::Amount, &self.amount)?;
        }
        if !self.currency.is_empty() {
            builder.add_filter_in_range_clause(AuthEventDimensions::Currency, &self.currency)?;
        }
        if !self.merchant_country.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::MerchantCountry,
                &self.merchant_country,
            )?;
        }
        if !self.billing_country.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::BillingCountry,
                &self.billing_country,
            )?;
        }
        if !self.shipping_country.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::ShippingCountry,
                &self.shipping_country,
            )?;
        }
        if !self.issuer_country.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::IssuerCountry,
                &self.issuer_country,
            )?;
        }
        if !self.earliest_supported_version.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::EarliestSupportedVersion,
                &self.earliest_supported_version,
            )?;
        }
        if !self.latest_supported_version.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::LatestSupportedVersion,
                &self.latest_supported_version,
            )?;
        }
        if !self.whitelist_decision.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::WhitelistDecision,
                &self.whitelist_decision,
            )?;
        }
        if !self.device_manufacturer.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::DeviceManufacturer,
                &self.device_manufacturer,
            )?;
        }
        if !self.device_type.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::DeviceType, &self.device_type)?;
        }
        if !self.device_brand.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::DeviceBrand, &self.device_brand)?;
        }
        if !self.device_os.is_empty() {
            builder.add_filter_in_range_clause(AuthEventDimensions::DeviceOs, &self.device_os)?;
        }
        if !self.device_display.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::DeviceDisplay,
                &self.device_display,
            )?;
        }
        if !self.browser_name.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::BrowserName, &self.browser_name)?;
        }
        if !self.browser_version.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::BrowserVersion,
                &self.browser_version,
            )?;
        }
        if !self.issuer_id.is_empty() {
            builder.add_filter_in_range_clause(AuthEventDimensions::IssuerId, &self.issuer_id)?;
        }
        if !self.scheme_name.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::SchemeName, &self.scheme_name)?;
        }
        if !self.exemption_requested.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::ExemptionRequested,
                &self.exemption_requested,
            )?;
        }
        if !self.exemption_accepted.is_empty() {
            builder.add_filter_in_range_clause(
                AuthEventDimensions::ExemptionAccepted,
                &self.exemption_accepted,
            )?;
        }

        Ok(())
    }
}
