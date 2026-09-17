/// Payments Link - Retrieve
///
/// To retrieve the properties of a Payment Link. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/payment_link/{payment_link_id}",
    params(
        ("payment_link_id" = String, Path, description = "The identifier for payment link"),
        ("client_secret" = Option<String>, Query, description = "This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK"),
    ),
    responses(
        (status = 200, description = "Gets details regarding payment link", body = RetrievePaymentLinkResponse),
        (status = 404, description = "No payment link found")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment Link",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub async fn payment_link_retrieve() {}

/// Payment Link - List
///
/// To list the payment links across all profiles for a merchant
#[utoipa::path(
    post,
    path = "/payment_link/list",
    request_body(
        content = PaymentLinkListConstraints,
        examples(
            (
                "List with default params" = (
                    value = json!({})
                )
            ),
            (
                "List with pagination and time range" = (
                    value = json!({
                        "limit": 10,
                        "offset": 0,
                        "start_time": "2025-01-01T00:00:00Z",
                        "end_time": "2025-03-31T23:59:59Z"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Payment links listed successfully", body = PaymentLinkListResponse),
        (status = 400, description = "Invalid request (e.g. end_time before start_time, range > 3 months)"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Payment Links",
    operation_id = "List Payment Links",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn payments_link_list() {}

/// Payment Link - Profile List
///
/// To list payment links scoped to the authenticated business profile
#[utoipa::path(
    post,
    path = "/payment_link/profile/list",
    request_body(
        content = PaymentLinkListConstraints,
        examples(
            (
                "List with default params" = (
                    value = json!({})
                )
            ),
            (
                "List with pagination" = (
                    value = json!({
                        "limit": 5,
                        "offset": 0,
                        "start_time": "2025-01-01T00:00:00Z"
                    })
                )
            )
        )
    ),
    responses(
        (status = 200, description = "Payment links listed successfully", body = PaymentLinkListResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Payment Links",
    operation_id = "List Profile Payment Links",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub async fn profile_payment_link_list() {}
