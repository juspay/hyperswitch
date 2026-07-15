/// Disputes - Retrieve Dispute
/// Retrieves a dispute
#[utoipa::path(
    get,
    path = "/disputes/{dispute_id}",
    params(
        ("dispute_id" = String, Path, description = "The identifier for dispute"),
        ("force_sync" = Option<bool>, Query, description = "Decider to enable or disable the connector call for dispute retrieve request"),
    ),
    responses(
        (status = 200, description = "The dispute was retrieved successfully", body = api_models::disputes::DisputeResponse),
        (status = 404, description = "Dispute does not exist in our records")
    ),
    tag = "Disputes",
    operation_id = "Retrieve a Dispute",
    security(("api_key" = []))
)]
pub async fn retrieve_dispute() {}

/// Disputes - List Disputes
/// Lists all the Disputes for a merchant
#[utoipa::path(
    get,
    path = "/disputes/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of Dispute Objects to include in the response"),
        ("dispute_status" = Option<api_models::enums::DisputeStatus>, Query, description = "The status of dispute"),
        ("dispute_stage" = Option<api_models::enums::DisputeStage>, Query, description = "The stage of dispute"),
        ("reason" = Option<String>, Query, description = "The reason for dispute"),
        ("connector" = Option<String>, Query, description = "The connector linked to dispute"),
        ("received_time" = Option<PrimitiveDateTime>, Query, description = "The time at which dispute is received"),
        ("received_time.lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the dispute received time"),
        ("received_time.gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the dispute received time"),
        ("received_time.lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the dispute received time"),
        ("received_time.gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the dispute received time"),
    ),
    responses(
        (status = 200, description = "The dispute list was retrieved successfully", body = Vec<api_models::disputes::DisputeResponse>),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Disputes",
    operation_id = "List Disputes",
    security(("api_key" = []))
)]
pub async fn retrieve_disputes_list() {}

/// Disputes - Accept Dispute
/// Accepts a dispute
#[utoipa::path(
    post,
    path = "/disputes/accept/{dispute_id}",
    params(
        ("dispute_id" = String, Path, description = "The identifier for dispute")
    ),
    responses(
        (status = 200, description = "The dispute was accepted successfully", body = api_models::disputes::DisputeResponse),
        (status = 404, description = "Dispute does not exist in our records", body = api_models::errors::types::GenericErrorResponseOpenApi),
    ),
    tag = "Disputes",
    operation_id = "Accept a Dispute",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn accept_dispute() {}

/// Disputes - Attach Evidence to Dispute
/// Attaches an uploaded evidence file to a dispute
#[utoipa::path(
    put,
    path = "/disputes/evidence",
    request_body(
        content = String,
        content_type = "multipart/form-data",
        description = "A multipart/form-data request with a `file` field containing the evidence file.",
    ),
    responses(
        (status = 200, description = "Evidence attached to dispute", body = api_models::files::CreateFileResponse),
        (status = 400, description = "Bad Request", body = api_models::errors::types::GenericErrorResponseOpenApi),
    ),
    tag = "Disputes",
    operation_id = "Attach Evidence to Dispute",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn attach_dispute_evidence() {}

/// Disputes - Uploads Dispute Evidence
/// Uploads evidence for a dispute
#[utoipa::path(
    post,
    path = "/disputes/evidence",
    request_body = api_models::disputes::SubmitEvidenceRequest,
    responses(
        (status = 200, description = "The dispute evidence submitted successfully", body = api_models::disputes::DisputeResponse),
        (status = 404, description = "Dispute does not exist in our records", body = api_models::errors::types::GenericErrorResponseOpenApi)
    ),
    tag = "Disputes",
    operation_id = "Submit Dispute Evidence",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn submit_dispute_evidence() {}

/// Disputes - Retrieve Dispute Evidence
/// Retrieves evidence for a dispute
#[utoipa::path(
    get,
    path = "/disputes/evidence/{dispute_id}",
    params(
        ("dispute_id" = String, Path, description = "The identifier for dispute")
    ),
    responses(
        (status = 200, description = "The dispute evidence was retrieved successfully", body = Vec<api_models::disputes::DisputeEvidenceBlock>),
        (status = 404, description = "Dispute does not exist in our records", body = api_models::errors::types::GenericErrorResponseOpenApi)
    ),
    tag = "Disputes",
    operation_id = "Retrieve a Dispute Evidence",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn retrieve_dispute_evidence() {}

/// Disputes - Delete Evidence attached to a Dispute
/// Deletes an evidence file attached to a dispute
#[utoipa::path(
    delete,
    path = "/disputes/evidence",
    request_body = api_models::disputes::DeleteEvidenceRequest,
    responses(
        (status = 200, description = "Evidence deleted from a dispute"),
        (status = 400, description = "Bad Request")
    ),
    tag = "Disputes",
    operation_id = "Delete Evidence attached to a Dispute",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn delete_dispute_evidence() {}

/// Disputes - Get Disputes Aggregate
/// Gets a count of disputes grouped by their status for a merchant within a time range
#[utoipa::path(
    get,
    path = "/disputes/aggregate",
    params(
        ("start_time" = String, Query, description = "The start time for the aggregate query")
    ),
    responses(
        (status = 200, description = "Disputes aggregate retrieved successfully", body = api_models::disputes::DisputesAggregateResponse),
    ),
    tag = "Disputes",
    operation_id = "Get Disputes Aggregate",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn get_disputes_aggregate() {}

/// Disputes - Get Disputes Aggregate for Profiles
/// Gets a count of disputes grouped by their status for the given profiles within a time range
#[utoipa::path(
    get,
    path = "/disputes/profile/aggregate",
    params(
        ("start_time" = String, Query, description = "The start time for the aggregate query")
    ),
    responses(
        (status = 200, description = "Disputes aggregate retrieved successfully", body = api_models::disputes::DisputesAggregateResponse),
    ),
    tag = "Disputes",
    operation_id = "Get Disputes Aggregate for Profiles",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn get_disputes_aggregate_profile() {}

/// Disputes - List Disputes for The Given Profiles
/// Lists all the Disputes for a merchant
#[utoipa::path(
    get,
    path = "/disputes/profile/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of Dispute Objects to include in the response"),
        ("dispute_status" = Option<api_models::enums::DisputeStatus>, Query, description = "The status of dispute"),
        ("dispute_stage" = Option<api_models::enums::DisputeStage>, Query, description = "The stage of dispute"),
        ("reason" = Option<String>, Query, description = "The reason for dispute"),
        ("connector" = Option<String>, Query, description = "The connector linked to dispute"),
        ("received_time" = Option<PrimitiveDateTime>, Query, description = "The time at which dispute is received"),
        ("received_time.lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the dispute received time"),
        ("received_time.gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the dispute received time"),
        ("received_time.lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the dispute received time"),
        ("received_time.gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the dispute received time"),
    ),
    responses(
        (status = 200, description = "The dispute list was retrieved successfully", body = Vec<api_models::disputes::DisputeResponse>),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Disputes",
    operation_id = "List Disputes for The given Profiles",
    security(("api_key" = []))
)]
pub async fn retrieve_disputes_list_profile() {}

/// Disputes - Disputes Filters
/// Lists all the filters associated with disputes
#[utoipa::path(
    get,
    path = "/disputes/filter",
    responses(
        (status = 200, description = "List of filters", body = api_models::disputes::DisputeListFilters),
    ),
    tag = "Disputes",
    operation_id = "List all filters for disputes",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn get_disputes_filters() {}

/// Disputes - Disputes Filters Profile
/// Lists all the filters associated with disputes for the given profiles
#[utoipa::path(
    get,
    path = "/disputes/profile/filter",
    responses(
        (status = 200, description = "List of filters", body = api_models::disputes::DisputeListFilters),
    ),
    tag = "Disputes",
    operation_id = "List all filters for disputes for the given Profiles",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn get_disputes_filters_profile() {}
