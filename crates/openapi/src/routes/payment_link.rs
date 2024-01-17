/// Payments Link - Retrieve
///
/// To retrieve the properties of a Payment Link. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/payment_link/{payment_link_id}",
    params(
        ("payment_link_id" = String, Path, description = "The identifier for payment link")
    ),
    request_body=RetrievePaymentLinkRequest,
    responses(
        (status = 200, description = "Gets details regarding payment link", body = RetrievePaymentLinkResponse),
        (status = 404, description = "No payment link found")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment Link",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub async fn payment_link_retrieve() {}
