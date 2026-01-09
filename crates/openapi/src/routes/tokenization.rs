use serde_json::json;
use utoipa::OpenApi;

/// Tokenization - Create
///
/// Create a token with customer_id
#[cfg(feature = "v2")]
#[utoipa::path(
    post,
    path = "/v2/tokenize",
    request_body(
        content = GenericTokenizationRequest,
        examples(("Create a token with customer_id" = (
            value = json!({
                "customer_id": "12345_cus_0196d94b9c207333a297cbcf31f2e8c8",
                "token_request": {
                    "payment_method_data": {
                        "card": {
                            "card_holder_name": "test name"
                        }
                    }
                }
            })
        )))
    ),
    responses(
        (status = 200, description = "Token created successfully", body = GenericTokenizationResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Tokenization",
    operation_id = "create_token_vault_api",
    security(("ephemeral_key" = []),("api_key" = []))
)]
pub async fn create_token_vault_api() {}

/// Tokenization - Delete
///
/// Delete a token entry with customer_id and session_id
#[cfg(feature = "v2")]
#[utoipa::path(
    delete,
    path = "/v2/tokenize/{id}",
    request_body(
        content = DeleteTokenDataRequest,
        examples(("Delete a token entry with customer_id and session_id" = (
            value = json!({
                "customer_id": "12345_cus_0196d94b9c207333a297cbcf31f2e8c8",
                "session_id": "12345_pms_01926c58bc6e77c09e809964e72af8c8",
            })
        )))
    ),
    responses(
        (status = 200, description = "Token deleted successfully", body = DeleteTokenDataResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Tokenization",
    operation_id = "delete_tokenized_data_api",
    security(("ephemeral_key" = []),("api_key" = []))
)]

pub async fn delete_tokenized_data_api() {}
