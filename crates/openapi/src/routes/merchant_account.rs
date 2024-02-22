/// Merchant Account - Create
///
/// Create a new account for a *merchant* and the *merchant* could be a seller or retailer or client who likes to receive and send payments.
#[utoipa::path(
    post,
    path = "/accounts",
    request_body(
        content = MerchantAccountCreate,
        examples(
            (
                "Create a merchant account with minimal fields" = (
                    value = json!({"merchant_id": "merchant_abc"})
                )
            ),
            (
                "Create a merchant account with webhook url" = (
                    value = json!({
                        "merchant_id": "merchant_abc",
                        "webhook_details" : {
                            "webhook_url": "https://webhook.site/a5c54f75-1f7e-4545-b781-af525b7e37a0"
                        }
                    })
                )
            ),
            (
                "Create a merchant account with return url" = (
                    value = json!({"merchant_id": "merchant_abc",
                "return_url": "https://example.com"})
                )
            )
        )

    ),
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
/// Retrieve a *merchant* account details.
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
/// Updates details of an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc
#[utoipa::path(
    post,
    path = "/accounts/{account_id}",
    request_body (
        content = MerchantAccountUpdate,
        examples(
            (
            "Update merchant name" = (
                value = json!({
                    "merchant_id": "merchant_abc",
                    "merchant_name": "merchant_name"
                })
            )
            ),
            ("Update merchant name" = (
                value = json!({
                    "merchant_id": "merchant_abc",
                    "merchant_name": "merchant_name"
                })
            )),
            ("Update webhook url" = (
                    value = json!({
                        "merchant_id": "merchant_abc",
                        "webhook_details": {
                            "webhook_url": "https://webhook.site/a5c54f75-1f7e-4545-b781-af525b7e37a0"
                        }
                    })
                )
            ),
            ("Update return url" = (
                value = json!({
                    "merchant_id": "merchant_abc",
                    "return_url": "https://example.com"
                })
            )))),
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
/// Delete a *merchant* account
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

/// Merchant Account - KV Status
///
/// Toggle KV mode for the Merchant Account
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/kv",
    request_body (
        content = ToggleKVRequest,
        examples (
            ("Enable KV for Merchant" = (
                value = json!({"merchant_id": "merchant_abc",
                "kv_enabled": "true"
                })
        )),
        ("Disable KV for Merchant" = (
                value = json!({"merchant_id": "merchant_abc",
                "kv_enabled": "false"
                })
        )))
    ),
    params (("account_id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "KV mode is enabled/disabled for Merchant Account", body = ToggleKVResponse),
        (status = 400, description = "Invalid data"),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Enable/Disable KV for a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn merchant_account_kv_status() {}
