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
                .attach_printable("Error adding platform filter")?;
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
            builder
                .add_filter_in_range_clause(AuthEventDimensions::Mcc, &self.mcc)
                .attach_printable("Failed to add MCC filter")?;
        }
        if !self.currency.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::Currency, &self.currency)
                .attach_printable("Failed to add currency filter")?;
        }
        if !self.merchant_country.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::MerchantCountry,
                    &self.merchant_country,
                )
                .attach_printable("Failed to add merchant country filter")?;
        }
        if !self.billing_country.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::BillingCountry,
                    &self.billing_country,
                )
                .attach_printable("Failed to add billing country filter")?;
        }
        if !self.shipping_country.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::ShippingCountry,
                    &self.shipping_country,
                )
                .attach_printable("Failed to add shipping country filter")?;
        }
        if !self.issuer_country.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::IssuerCountry,
                    &self.issuer_country,
                )
                .attach_printable("Failed to add issuer country filter")?;
        }
        if !self.earliest_supported_version.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::EarliestSupportedVersion,
                    &self.earliest_supported_version,
                )
                .attach_printable("Failed to add earliest supported version filter")?;
        }
        if !self.latest_supported_version.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::LatestSupportedVersion,
                    &self.latest_supported_version,
                )
                .attach_printable("Failed to add latest supported version filter")?;
        }
        if !self.whitelist_decision.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::WhitelistDecision,
                    &self.whitelist_decision,
                )
                .attach_printable("Failed to add whitelist decision filter")?;
        }
        if !self.device_manufacturer.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::DeviceManufacturer,
                    &self.device_manufacturer,
                )
                .attach_printable("Failed to add device manufacturer filter")?;
        }
        if !self.device_type.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::DeviceType, &self.device_type)
                .attach_printable("Failed to add device type filter")?;
        }
        if !self.device_brand.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::DeviceBrand, &self.device_brand)
                .attach_printable("Failed to add device brand filter")?;
        }
        if !self.device_os.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::DeviceOs, &self.device_os)
                .attach_printable("Failed to add device OS filter")?;
        }
        if !self.device_display.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::DeviceDisplay,
                    &self.device_display,
                )
                .attach_printable("Failed to add device display filter")?;
        }
        if !self.browser_name.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::BrowserName, &self.browser_name)
                .attach_printable("Failed to add browser name filter")?;
        }
        if !self.browser_version.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::BrowserVersion,
                    &self.browser_version,
                )
                .attach_printable("Failed to add browser version filter")?;
        }
        if !self.issuer_id.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::IssuerId, &self.issuer_id)
                .attach_printable("Failed to add issuer ID filter")?;
        }
        if !self.scheme_name.is_empty() {
            builder
                .add_filter_in_range_clause(AuthEventDimensions::SchemeName, &self.scheme_name)
                .attach_printable("Failed to add scheme name filter")?;
        }
        if !self.exemption_requested.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::ExemptionRequested,
                    &self.exemption_requested,
                )
                .attach_printable("Failed to add exemption requested filter")?;
        }
        if !self.exemption_accepted.is_empty() {
            builder
                .add_filter_in_range_clause(
                    AuthEventDimensions::ExemptionAccepted,
                    &self.exemption_accepted,
                )
                .attach_printable("Failed to add exemption accepted filter")?;
        }

        Ok(())
    }
}
