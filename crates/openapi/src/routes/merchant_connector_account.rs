/// Merchant Connector - Create
///
/// Creates a new Merchant Connector for the merchant account. The connector could be a payment processor/facilitator/acquirer or a provider of specialized services like Fraud/Accounting etc.
#[cfg(feature = "v1")]
#[utoipa::path(
    post,
    path = "/account/{account_id}/connectors",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account")
    ),
    request_body(
        content = MerchantConnectorCreate,
        examples(
            (
                "Create a merchant connector account with minimal fields" = (
                    value = json!({
                        "connector_type": "payment_processor",
                        "connector_name": "adyen",
                        "connector_account_details": {
                          "auth_type": "BodyKey",
                          "api_key": "{{adyen-api-key}}",
                          "key1": "{{adyen_merchant_account}}"
                        }
                      })
                )
            ),
            (
                "Create a merchant connector account under a specific profile" = (
                    value = json!({
                        "connector_type": "payment_processor",
                        "connector_name": "adyen",
                        "connector_account_details": {
                          "auth_type": "BodyKey",
                          "api_key": "{{adyen-api-key}}",
                          "key1": "{{adyen_merchant_account}}"
                        },
                        "profile_id": "{{profile_id}}"
                      })
                )
            ),
            (
                "Create a merchant account with custom connector label" = (
                    value = json!({
                        "connector_type": "payment_processor",
                        "connector_name": "adyen",
                        "connector_label": "EU_adyen",
                        "connector_account_details": {
                          "auth_type": "BodyKey",
                          "api_key": "{{adyen-api-key}}",
                          "key1": "{{adyen_merchant_account}}"
                        }
                      })
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Merchant Connector Created", body = MerchantConnectorResponse),
        (status = 400, description = "Missing Mandatory fields"),
    ),
    tag = "Merchant Connector Account",
    operation_id = "Create a Merchant Connector",
    security(("api_key" = []))
)]
pub async fn connector_create() {}

/// Connector Account - Create
///
/// Creates a new Connector Account for the merchant account. The connector could be a payment processor/facilitator/acquirer or a provider of specialized services like Fraud/Accounting etc.
#[cfg(feature = "v2")]
#[utoipa::path(
    post,
    path = "/v2/connector-accounts",
    request_body(
        content = MerchantConnectorCreate,
        examples(
            (
                "Create a merchant connector account with minimal fields" = (
                    value = json!({
                        "connector_type": "payment_processor",
                        "connector_name": "adyen",
                        "connector_account_details": {
                          "auth_type": "BodyKey",
                          "api_key": "{{adyen-api-key}}",
                          "key1": "{{adyen_merchant_account}}"
                        }
                      })
                )
            ),
            (
                "Create a merchant connector account under a specific profile" = (
                    value = json!({
                        "connector_type": "payment_processor",
                        "connector_name": "adyen",
                        "connector_account_details": {
                          "auth_type": "BodyKey",
                          "api_key": "{{adyen-api-key}}",
                          "key1": "{{adyen_merchant_account}}"
                        },
                        "profile_id": "{{profile_id}}"
                      })
                )
            ),
            (
                "Create a merchant account with custom connector label" = (
                    value = json!({
                        "connector_type": "payment_processor",
                        "connector_name": "adyen",
                        "connector_label": "EU_adyen",
                        "connector_account_details": {
                          "auth_type": "BodyKey",
                          "api_key": "{{adyen-api-key}}",
                          "key1": "{{adyen_merchant_account}}"
                        }
                      })
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Merchant Connector Created", body = MerchantConnectorResponse),
        (status = 400, description = "Missing Mandatory fields"),
    ),
    tag = "Merchant Connector Account",
    operation_id = "Create a Merchant Connector",
    security(("admin_api_key" = []))
)]
pub async fn connector_create() {}

/// Merchant Connector - Retrieve
///
/// Retrieves details of a Connector account
#[cfg(feature = "v1")]
#[utoipa::path(
    get,
    path = "/account/{account_id}/connectors/{merchant_connector_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("merchant_connector_id" = String, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector retrieved successfully", body = MerchantConnectorResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Retrieve a Merchant Connector",
    security(("api_key" = []))
)]
pub async fn connector_retrieve() {}

/// Connector Account - Retrieve
///
/// Retrieves details of a Connector account
#[cfg(feature = "v2")]
#[utoipa::path(
    get,
    path = "/v2/connector-accounts/{id}",
    params(
        ("id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector retrieved successfully", body = MerchantConnectorResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Retrieve a Merchant Connector",
    security(("admin_api_key" = []))
)]
pub async fn connector_retrieve() {}

/// Merchant Connector - List
///
/// List Merchant Connector Details for the merchant
#[utoipa::path(
    get,
    path = "/account/{account_id}/connectors",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorListResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "List all Merchant Connectors",
    security(("api_key" = []))
)]
pub async fn connector_list() {}

/// Merchant Connector - Update
///
/// To update an existing Merchant Connector account. Helpful in enabling/disabling different payment methods and other settings for the connector
#[cfg(feature = "v1")]
#[utoipa::path(
    post,
    path = "/account/{account_id}/connectors/{merchant_connector_id}",
    request_body(
        content = MerchantConnectorUpdate,
        examples(
            (
                "Enable card payment method" = (
                    value = json! ({
                        "connector_type": "payment_processor",
                        "payment_methods_enabled": [
                          {
                            "payment_method": "card"
                          }
                        ]
                })
                )
            ),
            (
                "Update webhook secret" = (
                    value = json! ({
                        "connector_webhook_details": {
                            "merchant_secret": "{{webhook_secret}}"
                          }
                    })
                )
            )
        ),
    ),
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("merchant_connector_id" = String, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Updated", body = MerchantConnectorResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
   tag = "Merchant Connector Account",
   operation_id = "Update a Merchant Connector",
   security(("api_key" = []))
)]
pub async fn connector_update() {}

/// Connector Account - Update
///
/// To update an existing Connector account. Helpful in enabling/disabling different payment methods and other settings for the connector
#[cfg(feature = "v2")]
#[utoipa::path(
    put,
    path = "/v2/connector-accounts/{id}",
    request_body(
        content = MerchantConnectorUpdate,
        examples(
            (
                "Enable card payment method" = (
                    value = json! ({
                        "connector_type": "payment_processor",
                        "payment_methods_enabled": [
                          {
                            "payment_method": "card"
                          }
                        ]
                })
                )
            ),
            (
                "Update webhook secret" = (
                    value = json! ({
                        "connector_webhook_details": {
                            "merchant_secret": "{{webhook_secret}}"
                          }
                    })
                )
            )
        ),
    ),
    params(
        ("id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Updated", body = MerchantConnectorResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
   tag = "Merchant Connector Account",
   operation_id = "Update a Merchant Connector",
   security(("admin_api_key" = []))
)]
pub async fn connector_update() {}

/// Merchant Connector - Delete
///
/// Delete or Detach a Merchant Connector from Merchant Account
#[cfg(feature = "v1")]
#[utoipa::path(
    delete,
    path = "/account/{account_id}/connectors/{merchant_connector_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("merchant_connector_id" = String, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Deleted", body = MerchantConnectorDeleteResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Delete a Merchant Connector",
    security(("admin_api_key" = []))
)]
pub async fn connector_delete() {}

/// Merchant Connector - Delete
///
/// Delete or Detach a Merchant Connector from Merchant Account
#[cfg(feature = "v2")]
#[utoipa::path(
    delete,
    path = "/v2/connector-accounts/{id}",
    params(
        ("id" = i32, Path, description = "The unique identifier for the Merchant Connector")
    ),
    responses(
        (status = 200, description = "Merchant Connector Deleted", body = MerchantConnectorDeleteResponse),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "Delete a Merchant Connector",
    security(("admin_api_key" = []))
)]
pub async fn connector_delete() {}
