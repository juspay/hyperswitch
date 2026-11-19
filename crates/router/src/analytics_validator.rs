use analytics::{enums::AuthInfo, errors::AnalyticsError};
use api_models::analytics::{AnalyticsRequest, ReportRequest};
use common_utils::{errors::CustomResult, id_type};
use currency_conversion::types::ExchangeRates;
use router_env::logger;

use crate::core::currency::get_forex_exchange_rates;

/// Reject scope mismatches detectable from authenticated IDs
/// The analytics query's authorization predicates enforce the remaining ownership boundaries.
pub fn validate_analytics_scope(
    auth: &AuthInfo,
    merchant_ids: &[id_type::MerchantId],
    profile_ids: &[id_type::ProfileId],
) -> CustomResult<(), AnalyticsError> {
    let allowed = match auth {
        AuthInfo::OrgLevel { .. } => true,
        AuthInfo::MerchantLevel {
            merchant_ids: allowed_merchants,
            ..
        } => merchant_ids.iter().all(|id| allowed_merchants.contains(id)),
        AuthInfo::ProfileLevel {
            merchant_id,
            profile_ids: allowed_profiles,
            ..
        } => {
            merchant_ids.iter().all(|id| id == merchant_id)
                && profile_ids.iter().all(|id| allowed_profiles.contains(id))
        }
    };
    if !allowed {
        return Err(AnalyticsError::AccessForbiddenError.into());
    }
    Ok(())
}

pub async fn request_validator(
    req_type: AnalyticsRequest,
    state: &crate::routes::SessionState,
) -> CustomResult<Option<ExchangeRates>, AnalyticsError> {
    let forex_enabled = state.conf.analytics.get_inner().get_forex_enabled();
    let require_forex_functionality = req_type.requires_forex_functionality();

    let ex_rates = if forex_enabled && require_forex_functionality {
        logger::info!("Fetching forex exchange rates");
        Some(get_forex_exchange_rates(state.clone()).await?)
    } else {
        None
    };

    Ok(ex_rates)
}

pub fn validate_report_request(request: &ReportRequest) -> Result<(), AnalyticsError> {
    match request.return_url {
        Some(ref return_url) => return_url
            .verify_https_scheme()
            .map_err(AnalyticsError::InvalidReturnUrl),
        None => Ok(()),
    }
}
