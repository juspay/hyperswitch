#[cfg(feature = "v1")]
/// Merchant Acquirer - Create
///
/// Create a new Merchant Acquirer for accessing our APIs from your servers.
#[utoipa::path(
    post,
    path = "/account/{account_id}/merchant_acquirers",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account")
    ),
    request_body = MerchantAcquirerCreate,
    responses(
        (status = 200, description = "Merchant Acquirer created", body = MerchantAcquirerResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Merchant Acquirer",
    operation_id = "Create a Merchant Acquirer",
    security(("api_key" = []))
)]
pub async fn merchant_acquirer_create() { /* … */
}
