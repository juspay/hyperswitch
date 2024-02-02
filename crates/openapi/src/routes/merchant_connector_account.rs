/// Merchant Connector - Create
///
/// Creates a new Merchant Connector for the merchant account. The connector could be a payment processor/facilitator/acquirer or a provider of specialized services like Fraud/Accounting etc.
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/connectors",
    request_body(
        content = MerchantConnectorCreate,
        examples(
            (
                "Create a merchant connector account with minimal fields" = (
                    value = json!({
                        "connector_type": "fiz_operations",
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
                "Create a merchant connector account under a specific business profile" = (
                    value = json!({
                        "connector_type": "fiz_operations",
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
                        "connector_type": "fiz_operations",
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
/// Asynchronously creates a payment connector. This method is responsible for establishing a connection to the payment system and setting up the necessary configurations and authentication. 
pub async fn payment_connector_create() {
    // implementation goes here
}

/// Merchant Connector - Retrieve
///
/// Retrieves details of a Connector account
#[utoipa::path(
    get,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
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
/// Asynchronously retrieves the payment connector information.
pub async fn payment_connector_retrieve() {}

/// Merchant Connector - List
///
/// List Merchant Connector Details for the merchant
#[utoipa::path(
    get,
    path = "/accounts/{account_id}/connectors",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
    ),
    responses(
        (status = 200, description = "Merchant Connector list retrieved successfully", body = Vec<MerchantConnectorResponse>),
        (status = 404, description = "Merchant Connector does not exist in records"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Merchant Connector Account",
    operation_id = "List all Merchant Connectors",
    security(("admin_api_key" = []))
)]
/// Asynchronously retrieves a list of payment connectors.
pub async fn payment_connector_list() {
    // method implementation goes here
}

/// Merchant Connector - Update
///
/// To update an existing Merchant Connector account. Helpful in enabling/disabling different payment methods and other settings for the connector
#[utoipa::path(
    post,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    request_body(
        content = MerchantConnectorUpdate,
        examples(
            (
                "Enable card payment method" = (
                    value = json! ({
                        "connector_type": "fiz_operations",
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
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
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
/// Asynchronously updates the payment connector with the latest information.
pub async fn payment_connector_update() {}

/// Merchant Connector - Delete
///
/// Delete or Detach a Merchant Connector from Merchant Account
#[utoipa::path(
    delete,
    path = "/accounts/{account_id}/connectors/{connector_id}",
    params(
        ("account_id" = String, Path, description = "The unique identifier for the merchant account"),
        ("connector_id" = i32, Path, description = "The unique identifier for the Merchant Connector")
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
/// Asynchronously deletes a payment connector from the system.
pub async fn payment_connector_delete() {}
