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
        AnalyticsDomain::PaymentIntents => GetInfoResponse {
            metrics: utils::get_payment_intent_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_payment_intent_dimensions(),
        },
        AnalyticsDomain::Refunds => GetInfoResponse {
            metrics: utils::get_refund_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_refund_dimensions(),
        },
        AnalyticsDomain::Frm => GetInfoResponse {
            metrics: utils::get_frm_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_frm_dimensions(),
        },
        AnalyticsDomain::SdkEvents => GetInfoResponse {
            metrics: utils::get_sdk_event_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_sdk_event_dimensions(),
        },
        AnalyticsDomain::AuthEvents => GetInfoResponse {
            metrics: utils::get_auth_event_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_auth_event_dimensions(),
        },
        AnalyticsDomain::ApiEvents => GetInfoResponse {
            metrics: utils::get_api_event_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_api_event_dimensions(),
        },
        AnalyticsDomain::Dispute => GetInfoResponse {
            metrics: utils::get_dispute_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_dispute_dimensions(),
        },
        AnalyticsDomain::Routing => GetInfoResponse {
            metrics: utils::get_payment_metrics_info(),
            download_dimensions: None,
            dimensions: utils::get_payment_dimensions(),
        },
    };
    Ok(info)
}
