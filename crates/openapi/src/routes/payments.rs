/// Payments - Create
///
/// **Creates a payment object when amount and currency are passed.** This API is also used to create a mandate by passing the `mandate_object`.
///
/// To completely process a payment you will have to create a payment, attach a payment method, confirm and capture funds.
///
/// Depending on the user journey you wish to achieve, you may opt to complete all the steps in a single request by attaching a payment method, setting `confirm=true` and `capture_method = automatic` in the *Payments/Create API* request or you could use the following sequence of API requests to achieve the same:
///
/// 1. Payments - Create
///
/// 2. Payments - Update
///
/// 3. Payments - Confirm
///
/// 4. Payments - Capture.
///
/// Use the client secret returned in this API along with your publishable key to make subsequent API calls from your client
#[utoipa::path(
    post,
    path = "/payments",
    request_body(
        content = PaymentsCreateRequest,
        examples(
            (
                "Create a payment with minimal fields" = (
                    value = json!({"amount": 6540,"currency": "USD"})
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Payment created", body = PaymentsResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Create a Payment",
    security(("api_key" = [])),
)]
pub fn payments_create() {}

/// Payments - Retrieve
///
/// To retrieve the properties of a Payment. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/payments/{payment_id}",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentRetrieveBody,
    responses(
        (status = 200, description = "Gets the payment with final status", body = PaymentsResponse),
        (status = 404, description = "No payment found")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub fn payments_retrieve() {}

/// Payments - Update
///
/// To update the properties of a PaymentIntent object. This may include attaching a payment method, or attaching customer object or metadata fields after the Payment is created
#[utoipa::path(
    post,
    path = "/payments/{payment_id}",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsRequest,
    responses(
        (status = 200, description = "Payment updated", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Update a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub fn payments_update() {}

/// Payments - Confirm
///
/// This API is to confirm the payment request and forward payment to the payment processor. This API provides more granular control upon when the API is forwarded to the payment processor. Alternatively you can confirm the payment within the Payments Create API
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/confirm",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsRequest,
    responses(
        (status = 200, description = "Payment confirmed", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Confirm a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub fn payments_confirm() {}

/// Payments - Capture
///
/// To capture the funds for an uncaptured payment
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/capture",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsCaptureRequest,
    responses(
        (status = 200, description = "Payment captured", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Capture a Payment",
    security(("api_key" = []))
)]
pub fn payments_capture() {}

/// Payments - Session token
///
/// To create the session object or to get session token for wallets
#[utoipa::path(
    post,
    path = "/payments/session_tokens",
    request_body=PaymentsSessionRequest,
    responses(
        (status = 200, description = "Payment session object created or session token was retrieved from wallets", body = PaymentsSessionResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Create Session tokens for a Payment",
    security(("publishable_key" = []))
)]
pub fn payments_connector_session() {}

/// Payments - Cancel
///
/// A Payment could can be cancelled when it is in one of these statuses: requires_payment_method, requires_capture, requires_confirmation, requires_customer_action
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/cancel",
    request_body=PaymentsCancelRequest,
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    responses(
        (status = 200, description = "Payment canceled"),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Cancel a Payment",
    security(("api_key" = []))
)]
pub fn payments_cancel() {}

/// Payments - List
///
/// To list the payments
#[utoipa::path(
    get,
    path = "/payments/list",
    params(
        ("customer_id" = String, Query, description = "The identifier for the customer"),
        ("starting_after" = String, Query, description = "A cursor for use in pagination, fetch the next list after some object"),
        ("ending_before" = String, Query, description = "A cursor for use in pagination, fetch the previous list before some object"),
        ("limit" = i64, Query, description = "Limit on the number of objects to return"),
        ("created" = PrimitiveDateTime, Query, description = "The time at which payment is created"),
        ("created_lt" = PrimitiveDateTime, Query, description = "Time less than the payment created time"),
        ("created_gt" = PrimitiveDateTime, Query, description = "Time greater than the payment created time"),
        ("created_lte" = PrimitiveDateTime, Query, description = "Time less than or equals to the payment created time"),
        ("created_gte" = PrimitiveDateTime, Query, description = "Time greater than or equals to the payment created time")
    ),
    responses(
        (status = 200, description = "Received payment list"),
        (status = 404, description = "No payments found")
    ),
    tag = "Payments",
    operation_id = "List all Payments",
    security(("api_key" = []))
)]
pub fn payments_list() {}

/// Payments - Incremental Authorization
///
/// Authorized amount for a payment can be incremented if it is in status: requires_capture
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/incremental_authorization",
    request_body=PaymentsIncrementalAuthorizationRequest,
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    responses(
        (status = 200, description = "Payment authorized amount incremented"),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Increment authorized amount for a Payment",
    security(("api_key" = []))
)]
pub fn payments_incremental_authorization() {}
