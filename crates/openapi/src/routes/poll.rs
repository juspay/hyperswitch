/// Poll - Retrieve Poll Status
#[utoipa::path(
    get,
    path = "/poll/status/{poll_id}",
    params(
        ("poll_id" = String, Path, description = "The identifier for poll")
    ),
    responses(
        (status = 200, description = "The poll status was retrieved successfully", body = PollResponse)
    ),
    tag = "Poll",
    operation_id = "Retrieve Poll Status",
    security(("publishable_key" = []))
)]
pub async fn retrieve_poll_status() {}
