use std::time;

#[inline]
pub async fn time_future<F, R>(future: F) -> (R, time::Duration)
where
    F: futures::Future<Output = R>,
{
    let start = time::Instant::now();
    let result = future.await;
    let time_spent = start.elapsed();
    (result, time_spent)
}
