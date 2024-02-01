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
            (
                "Create a payment with customer details and metadata" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "payment_id": "abcdefghijklmnopqrstuvwxyz",
                    "customer": {
                      "id": "cus_abcdefgh",
                      "name": "John Dough",
                      "phone": "9999999999",
                      "email": "john@example.com"
                    },
                    "description": "Its my first payment request",
                    "statement_descriptor_name": "joseph",
                    "statement_descriptor_suffix": "JS",
                    "metadata": {
                      "udf1": "some-value",
                      "udf2": "some-value"
                    }
                  })
                )
            ),
            (
                "Create a 3DS payment" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "authentication_type": "three_ds"
                  })
                )
            ),
            (
                "Create a manual capture payment" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "capture_method": "manual"
                  })
                )
            ),
            (
                "Create a setup mandate payment" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "confirm": true,
                    "customer_id": "StripeCustomer123",
                    "authentication_type": "no_three_ds",
                    "payment_method": "card",
                    "payment_method_data": {
                      "card": {
                        "card_number": "4242424242424242",
                        "card_exp_month": "10",
                        "card_exp_year": "25",
                        "card_holder_name": "joseph Doe",
                        "card_cvc": "123"
                      }
                    },
                    "setup_future_usage": "off_session",
                    "mandate_data": {
                      "customer_acceptance": {
                        "acceptance_type": "offline",
                        "accepted_at": "1963-05-03T04:07:52.723Z",
                        "online": {
                          "ip_address": "127.0.0.1",
                          "user_agent": "amet irure esse"
                        }
                      },
                      "mandate_type": {
                        "single_use": {
                          "amount": 6540,
                          "currency": "USD"
                        }
                      }
                    }
                  })
                )
            ),
            (
                "Create a recurring payment with mandate_id" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "confirm": true,
                    "customer_id": "StripeCustomer",
                    "authentication_type": "no_three_ds",
                    "mandate_id": "{{mandate_id}}",
                    "off_session": true
                  })
                )
            ),
            (
                "Create a payment and save the card" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "confirm": true,
                    "customer_id": "StripeCustomer123",
                    "authentication_type": "no_three_ds",
                    "payment_method": "card",
                    "payment_method_data": {
                      "card": {
                        "card_number": "4242424242424242",
                        "card_exp_month": "10",
                        "card_exp_year": "25",
                        "card_holder_name": "joseph Doe",
                        "card_cvc": "123"
                      }
                    },
                    "setup_future_usage": "off_session"
                  })
                )
            ),
            (
                "Create a payment using an already saved card's token" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "confirm": true,
                    "client_secret": "{{client_secret}}",
                    "payment_method": "card",
                    "payment_token": "{{payment_token}}",
                    "card_cvc": "123"
                  })
                )
            ),
            (
                "Create a manual capture payment" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "customer": {
                      "id": "cus_abcdefgh"
                    },
                    "billing": {
                      "address": {
                        "line1": "1467",
                        "line2": "Harrison Street",
                        "line3": "Harrison Street",
                        "city": "San Fransico",
                        "state": "California",
                        "zip": "94122",
                        "country": "US",
                        "first_name": "joseph",
                        "last_name": "Doe"
                      },
                      "phone": {
                        "number": "8056594427",
                        "country_code": "+91"
                      }
                    }
                })
            )
            )
        ),
    ),
    responses(
        (status = 200, description = "Payment created", body = PaymentsResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Create a Payment",
    security(("api_key" = [])),
)]
/// Creates a new payment record in the database.
pub fn payments_create() {
    // implementation goes here
}

/// Payments - Retrieve
///
/// Retrieves a Payment. This API can also be used to get the status of a previously initiated payment or next action for an ongoing payment
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
/// This method retrieves the payments from the database and returns them.
pub fn payments_retrieve() {}

/// Payments - Update
///
/// To update the properties of a *PaymentIntent* object. This may include attaching a payment method, or attaching customer object or metadata fields after the Payment is created
#[utoipa::path(
    post,
    path = "/payments/{payment_id}",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
   request_body(
     content = PaymentsUpdateRequest,
     examples(
      (
        "Update the payment amount" = (
          value = json!({
              "amount": 7654,
            }
          )
        )
      ),
      (
        "Update the shipping address" = (
          value = json!(
            {
              "shipping": {
                "address": {
                    "line1": "1467",
                    "line2": "Harrison Street",
                    "line3": "Harrison Street",
                    "city": "San Fransico",
                    "state": "California",
                    "zip": "94122",
                    "country": "US",
                    "first_name": "joseph",
                    "last_name": "Doe"
                },
                "phone": {
                    "number": "8056594427",
                    "country_code": "+91"
                }
              },
            }
          )
        )
      )
     )
    ),
    responses(
        (status = 200, description = "Payment updated", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Update a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
/// Updates the payments in the system.
pub fn payments_update() {
    // implementation goes here
}

/// Payments - Confirm
///
/// **Use this API to confirm the payment and forward the payment to the payment processor.**
///
/// Alternatively you can confirm the payment within the *Payments/Create* API by setting `confirm=true`. After confirmation, the payment could either:
///
/// 1. fail with `failed` status or
///
/// 2. transition to a `requires_customer_action` status with a `next_action` block or
///
/// 3. succeed with either `succeeded` in case of automatic capture or `requires_capture` in case of manual capture
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/confirm",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body(
     content = PaymentsConfirmRequest,
     examples(
      (
        "Confirm a payment with payment method data" = (
          value = json!({
              "payment_method": "card",
              "payment_method_type": "credit",
              "payment_method_data": {
                "card": {
                  "card_number": "4242424242424242",
                  "card_exp_month": "10",
                  "card_exp_year": "25",
                  "card_holder_name": "joseph Doe",
                  "card_cvc": "123"
                }
              }
            }
          )
        )
      )
     )
    ),
    responses(
        (status = 200, description = "Payment confirmed", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Confirm a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
/// Confirms the payments for a transaction.
/// This method handles the confirmation of payments for a specific transaction. It may involve updating the transaction status, updating the payment records, and sending notifications to relevant parties.
pub fn payments_confirm() {
    // implementation goes here
}

/// Payments - Capture
///
/// To capture the funds for an uncaptured payment
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/capture",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body (
        content = PaymentsCaptureRequest,
        examples(
            (
                "Capture the full amount" = (
                    value = json!({})
                )
            ),
            (
                "Capture partial amount" = (
                    value = json!({"amount_to_capture": 654})
                )
            ),
        )
    ),
    responses(
        (status = 200, description = "Payment captured", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Capture a Payment",
    security(("api_key" = []))
)]
/// Captures payments that have been authorized but not yet captured.
pub fn payments_capture() {
    // implementation goes here
}

/// Payments - Session token
///
/// Creates a session object or a session token for wallets like Apple Pay, Google Pay, etc. These tokens are used by Hyperswitch's SDK to initiate these wallets' SDK.
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
/// Creates and manages a session for connecting to the payments connector.
pub fn payments_connector_session() {
    // implementation goes here
}

/// Payments - Cancel
///
/// A Payment could can be cancelled when it is in one of these statuses: `requires_payment_method`, `requires_capture`, `requires_confirmation`, `requires_customer_action`.
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/cancel",
    request_body (
        content = PaymentsCancelRequest,
        examples(
            (
                "Cancel the payment with minimal fields" = (
                    value = json!({})
                )
            ),
            (
                "Cancel the payment with cancellation reason" = (
                    value = json!({"cancellation_reason": "requested_by_customer"})
                )
            ),
        )
    ),
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
/// Cancels all pending payments.
pub fn payments_cancel() {
    // implementation details here
}

/// Payments - List
///
/// To list the *payments*
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
        (status = 200, description = "Successfully retrieved a payment list", body = Vec<PaymentListResponse>),
        (status = 404, description = "No payments found")
    ),
    tag = "Payments",
    operation_id = "List all Payments",
    security(("api_key" = []))
)]
/// Returns a list of payments.
pub fn payments_list() {
    // implementation goes here
}

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
      (status = 200, description = "Payment authorized amount incremented", body = PaymentsResponse),
      (status = 400, description = "Missing mandatory fields")
  ),
  tag = "Payments",
  operation_id = "Increment authorized amount for a Payment",
  security(("api_key" = []))
)]
/// This method is used to incrementally authorize payments. It allows for the authorization of additional funds beyond the original authorization amount for a payment transaction. 
pub fn payments_incremental_authorization() {
    // implementation goes here
}
