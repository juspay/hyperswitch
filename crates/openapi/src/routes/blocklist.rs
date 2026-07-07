#[utoipa::path(
    post,
    path = "/blocklist/toggle",
    params (
        ("status" = bool, Query, description = "Boolean value to enable/disable blocklist"),
    ),
    responses(
        (status = 200, description = "Blocklist guard enabled/disabled", body = ToggleBlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "Toggle blocklist guard for a particular merchant",
    security(("api_key" = []))
)]
pub async fn toggle_blocklist_guard() {}

#[utoipa::path(
    post,
    path = "/blocklist",
    request_body = BlocklistRequest,
    responses(
        (status = 200, description = "Fingerprint Blocked", body = BlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "Block a Fingerprint",
    security(("api_key" = []))
)]
pub async fn add_entry_to_blocklist() {}

#[utoipa::path(
    delete,
    path = "/blocklist",
    request_body = BlocklistRequest,
    responses(
        (status = 200, description = "Fingerprint Unblocked", body = BlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "Unblock a Fingerprint",
    security(("api_key" = []))
)]
pub async fn remove_entry_from_blocklist() {}

#[utoipa::path(
    get,
    path = "/blocklist",
    params (
        ("data_kind" = BlocklistDataKind, Query, description = "Kind of the fingerprint list requested"),
    ),
    responses(
        (status = 200, description = "Blocked Fingerprints", body = BlocklistResponse),
        (status = 400, description = "Invalid Data")
    ),
    tag = "Blocklist",
    operation_id = "List Blocked fingerprints of a particular kind",
    security(("api_key" = []))
)]
pub async fn list_blocked_payment_methods() {}

#[utoipa::path(
    post,
    path = "/blocklist/batch",
    request_body(
        content = String,
        content_type = "multipart/form-data",
        description = "A multipart/form-data request with a `file` field containing a UTF-8 CSV (max 5 MB). \
            The CSV must have a header row: `type,data,metadata`. \
            `type`: one of `card_bin` (6 digits), `extended_card_bin` (8 digits), `fingerprint`. \
            `metadata`: optional, `key=value` pairs separated by `;` (e.g. `reason=fraud;source=manual`). \
            Maximum 100,000 data rows.",
    ),
    responses(
        (status = 202, description = "Batch blocklist job initiated", body = BatchBlocklistUploadResponse),
        (status = 400, description = "CSV validation error or file exceeds 5 MB limit"),
    ),
    tag = "Blocklist",
    operation_id = "Upload a batch blocklist CSV",
    security(("api_key" = []))
)]
pub async fn upload_batch_blocklist() {}

#[utoipa::path(
    get,
    path = "/blocklist/batch/{job_id}",
    params(
        ("job_id" = String, Path, description = "The job ID returned by the batch upload endpoint"),
    ),
    responses(
        (status = 200, description = "Batch blocklist job status", body = BatchBlocklistJobStatusResponse),
        (status = 404, description = "Job not found"),
    ),
    tag = "Blocklist",
    operation_id = "Get batch blocklist job status",
    security(("api_key" = []))
)]
pub async fn get_batch_blocklist_job_status() {}

#[utoipa::path(
    get,
    path = "/blocklist/batch",
    params(
        ("limit" = Option<u8>, Query, description = "Maximum number of jobs to return (default 10, max 100)"),
        ("offset" = Option<u32>, Query, description = "Zero-based offset for pagination (default 0)"),
    ),
    responses(
        (status = 200, description = "List of batch blocklist jobs", body = ListBatchBlocklistJobsResponse),
    ),
    tag = "Blocklist",
    operation_id = "List batch blocklist jobs",
    security(("api_key" = []))
)]
pub async fn list_batch_blocklist_jobs() {}
