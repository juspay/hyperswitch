#[cfg(feature = "v1")]
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

#[cfg(feature = "v2")]
/// Merchant Account - Create
///
/// Create a new account for a *merchant* and the *merchant* could be a seller or retailer or client who likes to receive and send payments.
///
/// Before creating the merchant account, it is mandatory to create an organization.
#[utoipa::path(
    post,
    path = "/v2/merchant-accounts",
    params(
      (
        "X-Organization-Id" = String, Header,
        description = "Organization ID for which the merchant account has to be created.",
        example = json!({"X-Organization-Id": "org_abcdefghijklmnop"})
      ),
    ),
    request_body(
        content = MerchantAccountCreate,
        examples(
            (
                "Create a merchant account with minimal fields" = (
                    value = json!({
                        "merchant_name": "Cloth Store",
                    })
                )
            ),
            (
                "Create a merchant account with merchant details" = (
                    value = json!({
                        "merchant_name": "Cloth Store",
                        "merchant_details": {
                                "primary_contact_person": "John Doe",
                                "primary_email": "example@company.com"
                        }
                    })
                )
            ),
            (
                "Create a merchant account with metadata" = (
                    value = json!({
                        "merchant_name": "Cloth Store",
                        "metadata": {
                                "key_1": "John Doe",
                                "key_2": "Trends"
                        }
                    })
                )
            ),

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

#[cfg(feature = "v1")]
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

#[cfg(feature = "v2")]
/// Merchant Account - Retrieve
///
/// Retrieve a *merchant* account details.
#[utoipa::path(
    get,
    path = "/v2/merchant-accounts/{id}",
    params (("id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Retrieved", body = MerchantAccountResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Retrieve a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn merchant_account_retrieve() {}

#[cfg(feature = "v1")]
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

#[cfg(feature = "v2")]
/// Merchant Account - Update
///
/// Updates details of an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc
#[utoipa::path(
    put,
    path = "/v2/merchant-accounts/{id}",
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
            ("Update Merchant Details" = (
                    value = json!({
                        "merchant_details": {
                                "primary_contact_person": "John Doe",
                                "primary_email": "example@company.com"
                        }
                    })
                )
            ),
            )),
    params (("id" = String, Path, description = "The unique identifier for the merchant account")),
    responses(
        (status = 200, description = "Merchant Account Updated", body = MerchantAccountResponse),
        (status = 404, description = "Merchant account not found")
    ),
    tag = "Merchant Account",
    operation_id = "Update a Merchant Account",
    security(("admin_api_key" = []))
)]
pub async fn merchant_account_update() {}

#[cfg(feature = "v1")]
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

#[cfg(feature = "v1")]
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
                value = json!({
                "kv_enabled": "true"
                })
        )),
        ("Disable KV for Merchant" = (
                value = json!({
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

/// Merchant Connector - List
///
/// List Merchant Connector Details for the merchant
#[utoipa::path(
    get,
    path = "/accounts/{account_id}/profile/connectors",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "List all Merchant Connectors for The given Profile",
    security(("admin_api_key" = []))
)]
pub async fn payment_connector_list_profile() {}

#[cfg(feature = "v2")]
/// Merchant Account - Profile List
///
/// List profiles for an Merchant
#[utoipa::path(
    get,
    path = "/v2/merchant-accounts/{id}/profiles",
    params (("id" = String, Path, description = "The unique identifier for the Merchant")),
    responses(
        (status = 200, description = "profile list retrieved successfully", body = Vec<ProfileResponse>),
        (status = 400, description = "Invalid data")
    ),
    tag = "Merchant Account",
    operation_id = "List Profiles",
    security(("admin_api_key" = []))
)]
pub async fn profiles_list() {}
