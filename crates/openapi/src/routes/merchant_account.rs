/// Merchant Account - Create
///
/// Create a new account for a merchant and the merchant could be a seller or retailer or client who likes to receive and send payments.
#[utoipa::path(
    post,
    path = "/accounts",
    request_body= MerchantAccountCreate,
    responses(
        (status = 200, description = "Merchant Account Created", body = MerchantAccountResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Merchant Account",
    operation_id = "Create a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn merchant_account_create() {}

/// Merchant Account - Retrieve
///
/// Retrieve a merchant account details.
#[utoipa::path(
    get,
    path = "/accounts/{account_id}",
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Retrieved", body = MerchantAccountResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Retrieve a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn retrieve_merchant_account() {}

/// Merchant Account - Update
///
/// To update an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc
#[utoipa::path(
    post,
    path = "/accounts/{account_id}",
    request_body = MerchantAccountUpdate,
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Updated", body = MerchantAccountResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Update a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn update_merchant_account() {}

/// Merchant Account - Delete
///
/// To delete a merchant account
#[utoipa::path(
    delete,
    path = "/accounts/{account_id}",
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Deleted", body = MerchantAccountDeleteResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Delete a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn delete_merchant_account() {}
