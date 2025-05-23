/// Relay - Create
///
/// Creates a relay request.
#[utoipa::path(
    post,
    path = "/relay",
    request_body(
        content = RelayRequest,
        examples((
            "Create a relay request" = (
                value = json!({
                    "connector_resource_id": "7256228702616471803954",
                    "connector_id": "mca_5apGeP94tMts6rg3U3kR",
                    "type": "refund",
                    "data": {
                        "refund": {
                            "amount": 6540,
                            "currency": "USD"
                        }
                    }
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Relay request", body = RelayResponse),
        (status = 400, description = "Invalid data")
    ),
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication"),
        ("X-Idempotency-Key" = String, Header, description = "Idempotency Key for relay request")
    ),
    tag = "Relay",
    operation_id = "Relay Request",
    security(("api_key" = []))
)]

pub async fn relay() {}

/// Relay - Retrieve
///
/// Retrieves a relay details.
#[utoipa::path(
    get,
    path = "/relay/{relay_id}",
    params (("relay_id" = String, Path, description = "The unique identifier for the Relay"), ("X-Profile-Id" = String, Header, description = "Profile ID for authentication")),
    responses(
        (status = 200, description = "Relay Retrieved", body = RelayResponse),
        (status = 404, description = "Relay details was not found")
    ),
    tag = "Relay",
    operation_id = "Retrieve a Relay details",
    security(("api_key" = []), ("ephemeral_key" = []))
)]

pub async fn relay_retrieve() {}
