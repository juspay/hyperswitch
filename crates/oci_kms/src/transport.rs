//! Timeouts and retries shared by every outbound OCI call: the crypto endpoint and proxymux.
//!
//! The AWS and GCP SDKs ship both; this backend has no SDK. Without timeouts a hung endpoint
//! stalls pod startup indefinitely, and without retries a single throttled (429) response
//! during a scale-up fails the pod outright.

use std::{future::Future, time::Duration};

use crate::{environment::Environment, error::OciKmsError};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

const MAX_ATTEMPTS: u32 = 4;
const BASE_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// A `reqwest` client builder with this backend's connect and request timeouts applied.
pub(crate) fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
}

/// A fresh `opc-request-id`: 32 lowercase hex characters, which OCI echoes in its logs and
/// error responses so a failed call can be traced.
pub(crate) fn request_id(environment: &dyn Environment) -> String {
    environment
        .random_bytes(16)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Why a single attempt failed, and whether trying again could help.
pub(crate) enum AttemptError {
    /// Transport failure, timeout, throttling or a server-side error.
    Retryable(OciKmsError),
    /// Bad request, auth failure, or anything else a retry would only repeat.
    Fatal(OciKmsError),
}

impl AttemptError {
    /// Classifies a non-success HTTP response.
    pub(crate) fn from_status(status: reqwest::StatusCode, error: OciKmsError) -> Self {
        if is_retryable_status(status) {
            Self::Retryable(error)
        } else {
            Self::Fatal(error)
        }
    }
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Runs `attempt` up to [`MAX_ATTEMPTS`] times, backing off between retryable failures.
/// Each call to `attempt` must build its request afresh: OCI signatures cover the `date`
/// header, so a replayed request would be rejected.
pub(crate) async fn with_retries<T, F, Fut>(
    environment: &dyn Environment,
    operation: &'static str,
    mut attempt: F,
) -> Result<T, OciKmsError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AttemptError>>,
{
    let mut attempt_number = 1;
    loop {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(AttemptError::Retryable(error)) if attempt_number < MAX_ATTEMPTS => {
                let delay = backoff(environment, attempt_number);
                tracing::warn!(
                    operation,
                    attempt = attempt_number,
                    ?delay,
                    %error,
                    "Retrying OCI request"
                );
                tokio::time::sleep(delay).await;
                attempt_number += 1;
            }
            Err(AttemptError::Retryable(error) | AttemptError::Fatal(error)) => return Err(error),
        }
    }
}

/// Exponential backoff with full jitter: uniform over `[0, min(MAX, BASE * 2^(n-1))]`, so
/// pods started together by a scale-up don't retry in lockstep.
fn backoff(environment: &dyn Environment, attempt_number: u32) -> Duration {
    backoff_ceiling(attempt_number).mul_f64(environment.random_f64_unit())
}

fn backoff_ceiling(attempt_number: u32) -> Duration {
    BASE_BACKOFF
        .saturating_mul(2u32.saturating_pow(attempt_number.saturating_sub(1)))
        .min(MAX_BACKOFF)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::environment::SystemEnvironment;

    fn failure() -> OciKmsError {
        OciKmsError::RequestFailed("test".to_owned())
    }

    #[test]
    fn request_id_is_32_hex_characters() {
        let id = request_id(&SystemEnvironment);
        assert_eq!(id.len(), 32);
        assert!(id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn throttling_and_server_errors_are_retryable() {
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(is_retryable_status(
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        ));
    }

    #[test]
    fn client_errors_are_not_retryable() {
        assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(reqwest::StatusCode::FORBIDDEN));
        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
    }

    #[test]
    fn backoff_ceiling_doubles_then_caps() {
        assert_eq!(backoff_ceiling(1), Duration::from_millis(250));
        assert_eq!(backoff_ceiling(2), Duration::from_millis(500));
        assert_eq!(backoff_ceiling(3), Duration::from_secs(1));
        assert_eq!(backoff_ceiling(10), MAX_BACKOFF);
        assert_eq!(backoff_ceiling(u32::MAX), MAX_BACKOFF);
    }

    #[test]
    fn backoff_stays_within_ceiling() {
        for attempt_number in 1..=MAX_ATTEMPTS {
            assert!(backoff(&SystemEnvironment, attempt_number) <= backoff_ceiling(attempt_number));
        }
    }

    #[tokio::test]
    async fn retryable_failures_are_retried_until_success() {
        let attempts = AtomicU32::new(0);
        let result = with_retries(&SystemEnvironment, "test", || async {
            if attempts.fetch_add(1, Ordering::SeqCst) < 2 {
                Err(AttemptError::Retryable(failure()))
            } else {
                Ok("ok")
            }
        })
        .await;

        assert_eq!(result.ok(), Some("ok"));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn fatal_failures_are_not_retried() {
        let attempts = AtomicU32::new(0);
        let result: Result<(), _> = with_retries(&SystemEnvironment, "test", || async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(AttemptError::Fatal(failure()))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_stop_after_max_attempts() {
        let attempts = AtomicU32::new(0);
        let result: Result<(), _> = with_retries(&SystemEnvironment, "test", || async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err(AttemptError::Retryable(failure()))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), MAX_ATTEMPTS);
    }
}
