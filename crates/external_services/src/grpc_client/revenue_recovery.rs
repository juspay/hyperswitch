/// Recovery Decider client
pub mod recovery_decider_client;

use std::fmt::Debug;

use common_utils::consts;
use router_env::logger;

/// Contains recovery grpc headers
#[derive(Debug)]
pub struct GrpcRecoveryHeaders {
    /// Request id
    pub request_id: Option<String>,
}

/// Trait to add necessary recovery headers to the tonic Request
pub(crate) trait AddRecoveryHeaders {
    /// Add necessary recovery header fields to the tonic Request
    fn add_recovery_headers(&mut self, headers: GrpcRecoveryHeaders);
}

impl<T> AddRecoveryHeaders for tonic::Request<T> {
    #[track_caller]
    fn add_recovery_headers(&mut self, headers: GrpcRecoveryHeaders) {
        headers.request_id.map(|request_id| {
            request_id
                .parse()
                .map(|request_id_val| {
                    self
                        .metadata_mut()
                        .append(consts::X_REQUEST_ID, request_id_val)
                })
                .inspect_err(
                    |err| logger::warn!(header_parse_error=?err,"invalid {} received",consts::X_REQUEST_ID),
                )
                .ok();
        });
    }
}

/// Creates a tonic::Request with recovery headers added.
pub(crate) fn create_revenue_recovery_grpc_request<T: Debug>(
    message: T,
    recovery_headers: GrpcRecoveryHeaders,
) -> tonic::Request<T> {
    let mut request = tonic::Request::new(message);
    request.add_recovery_headers(recovery_headers);
    request
}
