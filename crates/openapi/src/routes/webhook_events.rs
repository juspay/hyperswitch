/// Events - List
///
/// List all Events associated with a Merchant Account or Business Profile.
#[utoipa::path(
    get,
    path = "/events/{merchant_id_or_profile_id}",
    params(
        (
            "merchant_id_or_profile_id" = String,
            Path,
            description = "The unique identifier for the Merchant Account or Business Profile"
        ),
        (
            "created_after" = Option<PrimitiveDateTime>,
            Query,
            description = "Only include Events created after the specified time. \
                           Either only `object_id` must be specified, or one or more of `created_after`, `created_before`, `limit` and `offset` must be specified."
        ),
        (
            "created_before" = Option<PrimitiveDateTime>,
            Query,
            description = "Only include Events created before the specified time. \
                           Either only `object_id` must be specified, or one or more of `created_after`, `created_before`, `limit` and `offset` must be specified."
        ),
        (
            "limit" = Option<i64>,
            Query,
            description = "The maximum number of Events to include in the response. \
                           Either only `object_id` must be specified, or one or more of `created_after`, `created_before`, `limit` and `offset` must be specified."
        ),
        (
            "offset" = Option<i64>,
            Query,
            description = "The number of Events to skip when retrieving the list of Events.
                           Either only `object_id` must be specified, or one or more of `created_after`, `created_before`, `limit` and `offset` must be specified."
        ),
        (
            "object_id" = Option<String>,
            Query,
            description = "Only include Events associated with the specified object (Payment Intent ID, Refund ID, etc.). \
                           Either only `object_id` must be specified, or one or more of `created_after`, `created_before`, `limit` and `offset` must be specified."
        ),
    ),
    responses(
        (status = 200, description = "List of Events retrieved successfully", body = Vec<EventListItemResponse>),
    ),
    tag = "Event",
    operation_id = "List all Events associated with a Merchant Account or Business Profile",
    security(("admin_api_key" = []))
)]
pub fn list_initial_webhook_delivery_attempts() {}

/// Events - Delivery Attempt List
///
/// List all delivery attempts for the specified Event.
#[utoipa::path(
    get,
    path = "/events/{merchant_id_or_profile_id}/{event_id}/attempts",
    params(
        ("merchant_id_or_profile_id" = String, Path, description = "The unique identifier for the Merchant Account or Business Profile"),
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
