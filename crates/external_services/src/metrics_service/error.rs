//! Errors raised while reading metrics.

use common_utils::errors::CustomResult;

/// Result type for metric operations.
pub type MetricsResult<T> = CustomResult<T, MetricsError>;

/// What went wrong reading metrics.
///
/// Describes what happened to the request, not what a caller should do about it.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// The configuration could not be turned into a usable provider.
    #[error("Invalid metrics provider configuration: {0}")]
    Configuration(&'static str),

    /// The provider refused the request before it was sent.
    ///
    /// Providers differ on what they accept — periods, batch sizes, time ranges — so this is
    /// raised where those rules live, not by the request types.
    #[error("The metric request is not one this provider can be asked")]
    InvalidRequest,

    /// The call did not produce a usable response: credentials, network, throttling, or a request
    /// the provider rejected.
    #[error("The call to the metrics provider failed")]
    Transport,

    /// A response arrived but could not be read as a set of series.
    #[error("Could not interpret the metrics provider's response")]
    MalformedResponse,
}
