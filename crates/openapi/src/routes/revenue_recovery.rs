#[cfg(feature = "v2")]
/// Revenue Recovery - Retrieve
///
/// Retrieve the Revenue Recovery Payment Info
#[utoipa::path(
    get,
    path = "/v2/process-trackers/revenue-recovery-workflow/{revenue_recovery_id}",
    params(
        ("recovery_recovery_id" = String, Path, description = "The payment intent id"),
    ),
    responses(
        (status = 200, description = "Revenue Recovery Info Retrieved Successfully", body = RevenueRecoveryResponse),
        (status = 500, description = "Internal server error"),
        (status = 404, description = "Resource missing"),
        (status = 422, description = "Unprocessable request"),
        (status = 403, description = "Forbidden"),
    ),
   tag = "Revenue Recovery",
   operation_id = "Retrieve Revenue Recovery Info",
   security(("jwt_key" = []))
)]
pub async fn revenue_recovery_pt_retrieve_api() {}
