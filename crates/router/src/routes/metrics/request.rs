use router_env::metrics::add_attributes;

use super::utils as metric_utils;
use crate::services::ApplicationResponse;

pub async fn record_request_time_metric<F, R>(
    future: F,
    flow: &impl router_env::types::FlowMetric,
) -> R
where
    F: futures::Future<Output = R>,
{
    let key = "request_type";
    super::REQUESTS_RECEIVED.add(
        &super::CONTEXT,
        1,
        &add_attributes([(key, flow.to_string())]),
    );
    let (result, time) = metric_utils::time_future(future).await;
    super::REQUEST_TIME.record(
        &super::CONTEXT,
        time.as_secs_f64(),
        &add_attributes([(key, flow.to_string())]),
    );
    result
}

pub fn status_code_metrics(
    status_code: String,
    flow: String,
    merchant_id: common_utils::id_type::MerchantId,
) {
    super::REQUEST_STATUS.add(
        &super::CONTEXT,
        1,
        &add_attributes([
            ("status_code", status_code),
            ("flow", flow),
            ("merchant_id", merchant_id.get_string_repr().to_owned()),
        ]),
    )
}

pub fn track_response_status_code<Q>(response: &ApplicationResponse<Q>) -> i64 {
    match response {
        ApplicationResponse::Json(_)
        | ApplicationResponse::StatusOk
        | ApplicationResponse::TextPlain(_)
        | ApplicationResponse::Form(_)
        | ApplicationResponse::GenericLinkForm(_)
        | ApplicationResponse::PaymentLinkForm(_)
        | ApplicationResponse::FileData(_)
        | ApplicationResponse::JsonWithHeaders(_) => 200,
        ApplicationResponse::JsonForRedirection(_) => 302,
    }
}
