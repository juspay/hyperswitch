//! The clock and entropy this crate reads, behind a trait a service can replace.

use rand::RngCore;

/// Every clock and entropy read the client makes, other than key material.
///
/// Services that route such reads through their own instrumented functions (Hyperswitch does,
/// so that `deja` can replay them) implement this and pass it to
/// [`OciKmsClient::with_environment`](crate::OciKmsClient::with_environment). Key material,
/// such as the Workload Identity session key, always comes from the OS RNG.
pub trait Environment: Send + Sync + std::fmt::Debug {
    /// Seconds since the Unix epoch. Used for the signed `date` header and token expiry.
    fn now_unix_timestamp(&self) -> i64;

    /// A uniform draw from `[0, 1)`. Used for retry backoff jitter.
    fn random_f64_unit(&self) -> f64;

    /// `len` random bytes. Used for `opc-request-id` values.
    fn random_bytes(&self, len: usize) -> Vec<u8>;
}

/// Reads the system clock and the thread-local RNG directly.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn now_unix_timestamp(&self) -> i64 {
        #[allow(
            clippy::disallowed_methods,
            reason = "this function IS the seam, for services that don't supply their own"
        )]
        time::OffsetDateTime::now_utc().unix_timestamp()
    }

    fn random_f64_unit(&self) -> f64 {
        #[allow(
            clippy::disallowed_methods,
            reason = "this function IS the seam, for services that don't supply their own"
        )]
        rand::random::<f64>()
    }

    fn random_bytes(&self, len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        #[allow(
            clippy::disallowed_methods,
            reason = "this function IS the seam, for services that don't supply their own"
        )]
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    }
}
