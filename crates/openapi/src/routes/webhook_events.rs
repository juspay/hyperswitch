/// Events - List
///
/// List all Events associated with a Merchant Account or Profile.
#[utoipa::path(
    post,
    path = "/events/{merchant_id}",
    params(
        (
            "merchant_id" = String,
            Path,
            description = "The unique identifier for the Merchant Account."
        ),
    ),
    request_body(
        content = EventListConstraints,
        description = "The constraints that can be applied when listing Events.",
        examples (
            ("example" = (
                value = json!({
                    "created_after": "2023-01-01T00:00:00",
                    "created_before": "2023-01-31T23:59:59",
                    "limit": 5,
                    "offset": 0,
                    "object_id": "{{object_id}}",
                    "profile_id": "{{profile_id}}",
                    "event_classes": ["payments", "refunds"],
                    "event_types": ["payment_succeeded"],
                    "is_delivered": true
                })
            )),
        )
    ),
    responses(
        (status = 200, description = "List of Events retrieved successfully", body = TotalEventsResponse),
    ),
    tag = "Event",
    operation_id = "List all Events associated with a Merchant Account or Profile",
    security(("admin_api_key" = []))
)]
pub fn list_initial_webhook_delivery_attempts() {}

/// Events - List
///
/// List all Events associated with a Profile.
#[utoipa::path(
    post,
    path = "/events/profile/list",
    request_body(
        content = EventListConstraints,
        description = "The constraints that can be applied when listing Events.",
        examples (
            ("example" = (
                value = json!({
                    "created_after": "2023-01-01T00:00:00",
                    "created_before": "2023-01-31T23:59:59",
                    "limit": 5,
                    "offset": 0,
                    "object_id": "{{object_id}}",
                    "profile_id": "{{profile_id}}",
                    "event_classes": ["payments", "refunds"],
                    "event_types": ["payment_succeeded"],
                    "is_delivered": true
                })
            )),
        )
    ),
    responses(
        (status = 200, description = "List of Events retrieved successfully", body = TotalEventsResponse),
    ),
    tag = "Event",
    operation_id = "List all Events associated with a Profile",
    security(("jwt_key" = []))
)]
pub fn list_initial_webhook_delivery_attempts_with_jwtauth() {}

/// Events - Delivery Attempt List
///
/// List all delivery attempts for the specified Event.
#[utoipa::path(
    get,
    path = "/events/{merchant_id}/{event_id}/attempts",
    params(
        ("merchant_id" = String, Path, description = "The unique identifier for the Merchant Account."),
        ("event_id" = String, Path, description = "The unique identifier for the Event"),
    ),
    responses(
        (status = 200, description = "List of delivery attempts retrieved successfully", body = Vec<EventRetrieveResponse>),
    ),
    tag = "Event",
    operation_id = "List all delivery attempts for an Event",
    security(("admin_api_key" = []))
)]
pub fn list_webhook_delivery_attempts() {}

/// Events - Manual Retry
///
/// Manually retry the delivery of the specified Event.
#[utoipa::path(
    post,
    path = "/events/{merchant_id}/{event_id}/retry",
    params(
        ("merchant_id" = String, Path, description = "The unique identifier for the Merchant Account."),
        ("event_id" = String, Path, description = "The unique identifier for the Event"),
    ),
    responses(
        (
            status = 200,
            description = "The delivery of the Event was attempted. \
                           Check the `response` field in the response payload to identify the status of the delivery attempt.",
            body = EventRetrieveResponse
        ),
    ),
    tag = "Event",
    operation_id = "Manually retry the delivery of an Event",
    security(("admin_api_key" = []))
)]
pub fn retry_webhook_delivery_attempt() {}
