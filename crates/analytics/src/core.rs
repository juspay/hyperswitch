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
        AnalyticsDomain::SdkEvents => GetInfoResponse {
            metrics: utils::get_sdk_event_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_sdk_event_dimensions(),
        },
        AnalyticsDomain::ApiEvents => GetInfoResponse {
            metrics: utils::get_api_event_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_api_event_dimensions(),
        },
    };
    Ok(info)
}
