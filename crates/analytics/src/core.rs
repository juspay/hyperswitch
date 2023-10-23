use api_models::analytics::GetInfoResponse;

use crate::{types::AnalyticsDomain, utils};

pub async fn get_domain_info(
    domain: AnalyticsDomain,
) -> crate::errors::AnalyticsResult<GetInfoResponse> {
    let info = match domain {
        AnalyticsDomain::Payments => GetInfoResponse {
            metrics: utils::get_payment_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_payment_dimensions(),
        },
        AnalyticsDomain::Refunds => GetInfoResponse {
            metrics: utils::get_refund_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_refund_dimensions(),
        },

    };
    Ok(info)
}
