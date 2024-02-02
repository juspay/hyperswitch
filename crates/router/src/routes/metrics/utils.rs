use std::time;

#[inline]
/// Executes the provided asynchronous future and measures the time it takes to complete.
/// 
/// # Arguments
/// 
/// * `future` - The asynchronous future to execute and measure the time for.
/// 
/// # Returns
/// 
/// A tuple containing the result of the future and the duration it took to complete.
pub async fn time_future<F, R>(future: F) -> (R, time::Duration)
where
    F: futures::Future<Output = R>,
{
    let start = time::Instant::now();
    let result = future.await;
    let time_spent = start.elapsed();
    (result, time_spent)
}
