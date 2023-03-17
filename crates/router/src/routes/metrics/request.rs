use super::utils as metric_utils;

pub async fn record_request_time_metric<F, R>(future: F) -> R
where
    F: futures::Future<Output = R>,
{
    let (result, time) = metric_utils::time_future(future).await;
    super::REQUEST_TIME.record(&super::CONTEXT, time.as_secs_f64(), &[]);
    result
}
