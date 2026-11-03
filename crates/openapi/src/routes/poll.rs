/// Poll - Retrieve Poll Status
#[utoipa::path(
    get,
    path = "/poll/status/{poll_id}",
    params(
        ("poll_id" = String, Path, description = "The identifier for poll"),
        ("client_secret" = Option<String>, Query, description = "Client secret of the payment or authentication associated with this poll. Required unless supplied in SDK Authorization.")
    ),
    responses(
        (status = 200, description = "The poll status was retrieved successfully", body = PollResponse),
        (status = 400, description = "Client secret missing, invalid, or expired"),
        (status = 404, description = "Poll not found")
    ),
    tag = "Poll",
    operation_id = "Retrieve Poll Status",
    security(("publishable_key" = []))
)]
pub async fn retrieve_poll_status() {}
