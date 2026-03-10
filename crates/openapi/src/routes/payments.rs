/// Payments - Create
///
/// Creates a payment resource, which represents a customer's intent to pay.
/// This endpoint is the starting point for various payment flows:
///
#[utoipa::path(
    post,
    path = "/payments",
    request_body(
        content = PaymentsCreateRequest,
        examples(
            (
                "01. Create a payment with minimal fields" = (
                    value = json!({"amount": 6540,"currency": "USD"})
                )
            ),
            (
                "02. Create a payment with customer details and metadata" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "payment_id": "abcdefghijklmnopqrstuvwxyz",
                    "customer": {
                      "id": "cus_abcdefgh",
                      "name": "John Dough",
                      "phone": "9123456789",
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
                "03. Create a 3DS payment" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "authentication_type": "three_ds"
                  })
                )
            ),
            (
                "04. Create a manual capture payment (basic)" = (
                    value = json!({
                    "amount": 6540,
                    "currency": "USD",
                    "capture_method": "manual"
                  })
                )
            ),
            (
                "05. Create a setup mandate payment" = (
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
                        "acceptance_type": "online",
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
                    },
                    "customer_acceptance": {
                      "acceptance_type": "online",
                      "accepted_at": "1963-05-03T04:07:52.723Z",
                      "online": {
                        "ip_address": "127.0.0.1",
                        "user_agent": "amet irure esse"
                      }
                    }
                  })
                )
            ),
            (
                "06. Create a recurring payment with mandate_id" = (
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
                "07. Create a payment and save the card" = (
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
                    "customer_acceptance": {
                      "acceptance_type": "online",
                      "accepted_at": "1963-05-03T04:07:52.723Z",
                      "online": {
                        "ip_address": "127.0.0.1",
                        "user_agent": "amet irure esse"
                      }
                    },
                    "setup_future_usage": "off_session"
                  })
                )
            ),
            (
                "08. Create a payment using an already saved card's token" = (
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
                "09. Create a payment with billing details" = (
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
                        "number": "9123456789",
                        "country_code": "+91"
                      }
                    }
                })
            )
            ),
            (
              "10. Create a Stripe Split Payments CIT call" = (
                value = json!({
                  "amount": 200,
                  "currency": "USD",
                  "profile_id": "pro_abcdefghijklmnop",
                  "confirm": true,
                  "capture_method": "automatic",
                  "amount_to_capture": 200,
                  "customer_id": "StripeCustomer123",
                  "setup_future_usage": "off_session",
                  "customer_acceptance": {
                      "acceptance_type": "offline",
                      "accepted_at": "1963-05-03T04:07:52.723Z",
                      "online": {
                          "ip_address": "125.0.0.1",
                          "user_agent": "amet irure esse"
                      }
                  },
                  "authentication_type": "no_three_ds",
                  "return_url": "https://hyperswitch.io",
                  "name": "John Doe",
                  "phone": "999999999",
                  "phone_country_code": "+65",
                  "description": "Its my first payment request",
                  "payment_method": "card",
                  "payment_method_type": "debit",
                  "payment_method_data": {
                      "card": {
                          "card_number": "4242424242424242",
                          "card_exp_month": "09",
                          "card_exp_year": "25",
                          "card_holder_name": "joseph Doe",
                          "card_cvc": "123"
                      }
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
                          "number": "9999999999",
                          "country_code": "+91"
                      }
                  },
                  "split_payments": {
                      "stripe_split_payment": {
                          "charge_type": "direct",
                          "application_fees": 100,
                          "transfer_account_id": "acct_123456789"
                      }
                  }
              })
              )
            ),
            (
              "11. Create a Stripe Split Payments MIT call" = (
                value = json!({
                  "amount": 200,
                  "currency": "USD",
                  "profile_id": "pro_abcdefghijklmnop",
                  "customer_id": "StripeCustomer123",
                  "description": "Subsequent Mandate Test Payment (MIT from New CIT Demo)",
                  "confirm": true,
                  "off_session": true,
                  "recurring_details": {
                      "type": "payment_method_id",
                      "data": "pm_123456789" 
                  },
                  "split_payments": {
                      "stripe_split_payment": {
                          "charge_type": "direct",
                          "application_fees": 11,
                          "transfer_account_id": "acct_123456789"
                      }
                  }
              })
              )
            ),
        ),
    ),
    responses(
        (status = 200, description = "Payment created", body = PaymentsCreateResponseOpenApi,
            examples(
                ("01. Response for minimal payment creation (requires payment method)" = (
                    value = json!({
                        "payment_id": "pay_syxxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "requires_payment_method",
                        "amount": 6540,
                        "currency": "USD",
                        "client_secret": "pay_syxxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:00:00Z",
                        "amount_capturable": 6540,
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "expires_on": "2023-10-26T10:15:00Z"
                    })
                )),
                ("02. Response for payment with customer details (requires payment method)" = (
                    value = json!({
                        "payment_id": "pay_custmeta_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "requires_payment_method",
                        "amount": 6540,
                        "currency": "USD",
                        "customer_id": "cus_abcdefgh",
                        "customer": {
                            "id": "cus_abcdefgh",
                            "name": "John Dough",
                            "email": "john@example.com",
                            "phone": "9123456789"
                        },
                        "description": "Its my first payment request",
                        "statement_descriptor_name": "joseph",
                        "statement_descriptor_suffix": "JS",
                        "metadata": {
                            "udf1": "some-value",
                            "udf2": "some-value"
                        },
                        "client_secret": "pay_custmeta_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:05:00Z",
                        "ephemeral_key": {
                            "customer_id": "cus_abcdefgh",
                            "secret": "epk_ephemeralxxxxxxxxxxxx"
                        },
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "expires_on": "2023-10-26T10:20:00Z"
                    })
                )),
                ("03. Response for 3DS payment creation (requires payment method)" = (
                    value = json!({
                        "payment_id": "pay_3ds_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "requires_payment_method",
                        "amount": 6540,
                        "currency": "USD",
                        "authentication_type": "three_ds",
                        "client_secret": "pay_3ds_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:10:00Z",
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "expires_on": "2023-10-26T10:25:00Z"
                    })
                )),
                ("04. Response for basic manual capture payment (requires payment method)" = (
                    value = json!({
                        "payment_id": "pay_manualcap_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "requires_payment_method",
                        "amount": 6540,
                        "currency": "USD",
                        "capture_method": "manual",
                        "client_secret": "pay_manualcap_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:15:00Z",
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "expires_on": "2023-10-26T10:30:00Z"
                    })
                )),
                ("05. Response for successful setup mandate payment" = (
                    value = json!({
                        "payment_id": "pay_mandatesetup_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "succeeded",
                        "amount": 6540,
                        "currency": "USD",
                        "amount_capturable": 0,
                        "amount_received": 6540,
                        "connector": "fauxpay",
                        "customer_id": "StripeCustomer123",
                        "mandate_id": "man_xxxxxxxxxxxx",
                        "mandate_data": {
                            "customer_acceptance": {
                                "acceptance_type": "online",
                                "accepted_at": "1963-05-03T04:07:52.723Z",
                                "online": { "ip_address": "127.0.0.1", "user_agent": "amet irure esse" }
                            },
                            "mandate_type": { "single_use": { "amount": 6540, "currency": "USD" } }
                        },
                        "setup_future_usage": "on_session",
                        "payment_method": "card",
                        "payment_method_data": {
                            "card": { "last4": "4242", "card_exp_month": "10", "card_exp_year": "25", "card_holder_name": "joseph Doe" }
                        },
                        "authentication_type": "no_three_ds",
                        "client_secret": "pay_mandatesetup_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:20:00Z",
                        "ephemeral_key": { "customer_id": "StripeCustomer123", "secret": "epk_ephemeralxxxxxxxxxxxx" },
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "merchant_connector_id": "mca_mcaconnectorxxxx",
                        "connector_transaction_id": "txn_connectortransidxxxx"
                    })
                )),
                ("06. Response for successful recurring payment with mandate_id" = (
                    value = json!({
                        "payment_id": "pay_recurring_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "succeeded",
                        "amount": 6540,
                        "currency": "USD",
                        "amount_capturable": 0,
                        "amount_received": 6540,
                        "connector": "fauxpay",
                        "customer_id": "StripeCustomer",
                        "mandate_id": "{{mandate_id}}",
                        "off_session": true,
                        "payment_method": "card",
                        "authentication_type": "no_three_ds",
                        "client_secret": "pay_recurring_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:22:00Z",
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "merchant_connector_id": "mca_mcaconnectorxxxx",
                        "connector_transaction_id": "txn_connectortransidxxxx"
                    })
                )),
                ("07. Response for successful payment with card saved" = (
                    value = json!({
                        "payment_id": "pay_savecard_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "succeeded",
                        "amount": 6540,
                        "currency": "USD",
                        "amount_capturable": 0,
                        "amount_received": 6540,
                        "connector": "fauxpay",
                        "customer_id": "StripeCustomer123",
                        "setup_future_usage": "on_session",
                        "payment_method": "card",
                        "payment_method_data": {
                            "card": { "last4": "4242", "card_exp_month": "10", "card_exp_year": "25", "card_holder_name": "joseph Doe" }
                        },
                        "authentication_type": "no_three_ds",
                        "client_secret": "pay_savecard_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:25:00Z",
                        "ephemeral_key": { "customer_id": "StripeCustomer123", "secret": "epk_ephemeralxxxxxxxxxxxx" },
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "merchant_connector_id": "mca_mcaconnectorxxxx",
                        "connector_transaction_id": "txn_connectortransidxxxx",
                        "payment_token": null // Assuming payment_token is for subsequent use, not in this response.
                    })
                )),
                ("08. Response for successful payment using saved card token" = (
                    value = json!({
                        "payment_id": "pay_token_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "succeeded",
                        "amount": 6540,
                        "currency": "USD",
                        "amount_capturable": 0,
                        "amount_received": 6540,
                        "connector": "fauxpay",
                        "payment_method": "card",
                        "payment_token": "{{payment_token}}",
                        "client_secret": "pay_token_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:27:00Z",
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "merchant_connector_id": "mca_mcaconnectorxxxx",
                        "connector_transaction_id": "txn_connectortransidxxxx"
                    })
                )),
                ("09. Response for payment with billing details (requires payment method)" = (
                    value = json!({
                        "payment_id": "pay_manualbill_xxxxxxxxxxxx",
                        "merchant_id": "merchant_myyyyyyyyyyyy",
                        "status": "requires_payment_method",
                        "amount": 6540,
                        "currency": "USD",
                        "customer_id": "cus_abcdefgh",
                        "customer": {
                            "id": "cus_abcdefgh",
                            "name": "John Dough", 
                            "email": "john@example.com", 
                            "phone": "9123456789"
                        },
                        "billing": {
                            "address": {
                                "line1": "1467", "line2": "Harrison Street", "city": "San Fransico",
                                "state": "California", "zip": "94122", "country": "US",
                                "first_name": "joseph", "last_name": "Doe"
                            },
                            "phone": { "number": "9123456789", "country_code": "+91" }
                        },
                        "client_secret": "pay_manualbill_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                        "created": "2023-10-26T10:30:00Z",
                        "ephemeral_key": { "customer_id": "cus_abcdefgh", "secret": "epk_ephemeralxxxxxxxxxxxx" },
                        "profile_id": "pro_pzzzzzzzzzzz",
                        "attempt_count": 1,
                        "expires_on": "2023-10-26T10:45:00Z"
                    })
                )),

                ("10. Response for the CIT call for Stripe Split Payments" = (
                  value = json!({
                      "payment_id": "pay_manualbill_xxxxxxxxxxxx",
                      "merchant_id": "merchant_myyyyyyyyyyyy",
                      "status": "succeeded",
                      "amount": 200,
                      "currency": "USD",
                      "customer_id": "cus_abcdefgh",
                      "payment_method_id": "pm_123456789",
                      "connector_mandate_id": "pm_abcdefgh",
                      "customer": {
                          "id": "cus_abcdefgh",
                          "name": "John Dough", 
                          "email": "john@example.com", 
                          "phone": "9123456789"
                      },
                      "billing": {
                          "address": {
                              "line1": "1467", "line2": "Harrison Street", "city": "San Fransico",
                              "state": "California", "zip": "94122", "country": "US",
                              "first_name": "joseph", "last_name": "Doe"
                          },
                          "phone": { "number": "9123456789", "country_code": "+91" }
                      },
                      "client_secret": "pay_manualbill_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                      "created": "2023-10-26T10:30:00Z",
                      "ephemeral_key": { "customer_id": "cus_abcdefgh", "secret": "epk_ephemeralxxxxxxxxxxxx" },
                      "profile_id": "pro_pzzzzzzzzzzz",
                      "attempt_count": 1,
                      "expires_on": "2023-10-26T10:45:00Z"
                  })
              )),

              ("11. Response for the MIT call for Stripe Split Payments" = (
                value = json!({
                    "payment_id": "pay_manualbill_xxxxxxxxxxxx",
                    "merchant_id": "merchant_myyyyyyyyyyyy",
                    "status": "succeeded",
                    "amount": 200,
                    "currency": "USD",
                    "customer_id": "cus_abcdefgh",
                    "payment_method_id": "pm_123456789",
                    "connector_mandate_id": "pm_abcdefgh",
                    "customer": {
                        "id": "cus_abcdefgh",
                        "name": "John Dough", 
                        "email": "john@example.com", 
                        "phone": "9123456789"
                    },
                    "billing": {
                        "address": {
                            "line1": "1467", "line2": "Harrison Street", "city": "San Fransico",
                            "state": "California", "zip": "94122", "country": "US",
                            "first_name": "joseph", "last_name": "Doe"
                        },
                        "phone": { "number": "9123456789", "country_code": "+91" }
                    },
                    "client_secret": "pay_manualbill_xxxxxxxxxxxx_secret_szzzzzzzzzzz",
                    "created": "2023-10-26T10:30:00Z",
                    "ephemeral_key": { "customer_id": "cus_abcdefgh", "secret": "epk_ephemeralxxxxxxxxxxxx" },
                    "profile_id": "pro_pzzzzzzzzzzz",
                    "attempt_count": 1,
                    "expires_on": "2023-10-26T10:45:00Z"
                })
            ))
            )
        ),
        (status = 400, description = "Missing Mandatory fields", body = GenericErrorResponseOpenApi),
    ),
    tag = "Payments",
    operation_id = "Create a Payment",
    security(("api_key" = [])),
)]
pub fn payments_create() {}

/// Payments - Retrieve
///
/// Retrieves a Payment. This API can also be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/payments/{payment_id}",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment"),
        ("force_sync" = Option<bool>, Query, description = "Decider to enable or disable the connector call for retrieve request"),
        ("client_secret" = Option<String>, Query, description = "This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK"),
        ("expand_attempts" = Option<bool>, Query, description = "If enabled provides list of attempts linked to payment intent"),
        ("expand_captures" = Option<bool>, Query, description = "If enabled provides list of captures linked to latest attempt"),
    ),
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
                    "number": "9123456789",
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
        (status = 200, description = "Payment updated", body = PaymentsCreateResponseOpenApi),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Update a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub fn payments_update() {}

/// Payments - Confirm
///
/// Confirms a payment intent that was previously created with `confirm: false`. This action attempts to authorize the payment with the payment processor.
///
/// Expected status transitions after confirmation:
/// - `succeeded`: If authorization is successful and `capture_method` is `automatic`.
/// - `requires_capture`: If authorization is successful and `capture_method` is `manual`.
/// - `failed`: If authorization fails.
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
              },
              "customer_acceptance": {
                "acceptance_type": "online",
                "accepted_at": "1963-05-03T04:07:52.723Z",
                "online": {
                  "ip_address": "127.0.0.1",
                  "user_agent": "amet irure esse"
                }
              }
            }
          )
        )
      )
     )
    ),
    responses(
        (status = 200, description = "Payment confirmed", body = PaymentsCreateResponseOpenApi),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Confirm a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
pub fn payments_confirm() {}

/// Payments - Capture
///
/// Captures the funds for a previously authorized payment intent where `capture_method` was set to `manual` and the payment is in a `requires_capture` state.
///
/// Upon successful capture, the payment status usually transitions to `succeeded`.
/// The `amount_to_capture` can be specified in the request body; it must be less than or equal to the payment's `amount_capturable`. If omitted, the full capturable amount is captured.
///
/// A payment must be in a capturable state (e.g., `requires_capture`). Attempting to capture an already `succeeded` (and fully captured) payment or one in an invalid state will lead to an error.
///
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
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Capture a Payment",
    security(("api_key" = []))
)]
pub fn payments_capture() {}

#[cfg(feature = "v1")]
/// Payments - Session token
///
/// Creates a session object or a session token for wallets like Apple Pay, Google Pay, etc. These tokens are used by Hyperswitch's SDK to initiate these wallets' SDK.
#[utoipa::path(
  post,
  path = "/payments/session_tokens",
  request_body=PaymentsSessionRequest,
  responses(
      (status = 200, description = "Payment session object created or session token was retrieved from wallets", body = PaymentsSessionResponse),
      (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Create Session tokens for a Payment",
  security(("publishable_key" = []))
)]
pub fn payments_connector_session() {}

#[cfg(feature = "v2")]
/// Payments - Session token
///
/// Creates a session object or a session token for wallets like Apple Pay, Google Pay, etc. These tokens are used by Hyperswitch's SDK to initiate these wallets' SDK.
#[utoipa::path(
    post,
    path = "/v2/payments/{payment_id}/create-external-sdk-tokens",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsSessionRequest,
    responses(
        (status = 200, description = "Payment session object created or session token was retrieved from wallets", body = PaymentsSessionResponse),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Create V2 Session tokens for a Payment",
    security(("publishable_key" = []))
)]
pub fn payments_connector_session() {}

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
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Cancel a Payment",
    security(("api_key" = []))
)]
pub fn payments_cancel() {}

/// Payments - Cancel Post Capture
///
/// A Payment could can be cancelled when it is in one of these statuses: `succeeded`, `partially_captured`, `partially_captured_and_capturable`.
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/cancel_post_capture",
    request_body (
        content = PaymentsCancelPostCaptureRequest,
        examples(
            (
                "Cancel the payment post capture with minimal fields" = (
                    value = json!({})
                )
            ),
            (
                "Cancel the payment post capture with cancellation reason" = (
                    value = json!({"cancellation_reason": "requested_by_customer"})
                )
            ),
        )
    ),
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    responses(
        (status = 200, description = "Payment canceled post capture"),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Cancel a Payment Post Capture",
    security(("api_key" = []))
)]
pub fn payments_cancel_post_capture() {}

/// Payments - List
///
/// To list the *payments*
#[cfg(feature = "v1")]
#[utoipa::path(
    get,
    path = "/payments/list",
    params(
        ("customer_id" = Option<String>, Query, description = "The identifier for the customer"),
        ("starting_after" = Option<String>, Query, description = "A cursor for use in pagination, fetch the next list after some object"),
        ("ending_before" = Option<String>, Query, description = "A cursor for use in pagination, fetch the previous list before some object"),
        ("limit" = Option<i64>, Query, description = "Limit on the number of objects to return"),
        ("created" = Option<PrimitiveDateTime>, Query, description = "The time at which payment is created"),
        ("created_lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the payment created time"),
        ("created_gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the payment created time"),
        ("created_lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the payment created time"),
        ("created_gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the payment created time")
    ),
    responses(
        (status = 200, description = "Successfully retrieved a payment list", body = Vec<PaymentListResponse>),
        (status = 404, description = "No payments found")
    ),
    tag = "Payments",
    operation_id = "List all Payments",
    security(("api_key" = []))
)]
pub fn payments_list() {}

/// Profile level Payments - List
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
  operation_id = "List all Payments for the Profile",
  security(("api_key" = []))
)]
pub async fn profile_payments_list() {}

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
      (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Increment authorized amount for a Payment",
  security(("api_key" = []))
)]
pub fn payments_incremental_authorization() {}

/// Payments - Extended Authorization
///
/// Extended authorization is available for payments currently in the `requires_capture` status
/// Call this endpoint to increase the authorization validity period
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/extend_authorization",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    responses(
        (status = 200, description = "Extended authorization for the payment"),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Extend authorization period for a Payment",
    security(("api_key" = []))
)]
pub fn payments_extend_authorization() {}

/// Payments - External 3DS Authentication
///
/// External 3DS Authentication is performed and returns the AuthenticationResponse
#[utoipa::path(
  post,
  path = "/payments/{payment_id}/3ds/authentication",
  request_body=PaymentsExternalAuthenticationRequest,
  params(
      ("payment_id" = String, Path, description = "The identifier for payment")
  ),
  responses(
      (status = 200, description = "Authentication created", body = PaymentsExternalAuthenticationResponse),
      (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Initiate external authentication for a Payment",
  security(("publishable_key" = []))
)]
pub fn payments_external_authentication() {}

/// Payments - Complete Authorize
#[utoipa::path(
  post,
  path = "/payments/{payment_id}/complete_authorize",
  request_body=PaymentsCompleteAuthorizeRequest,
  params(
    ("payment_id" =String, Path, description =  "The identifier for payment")
  ),
 responses(
      (status = 200, description = "Payments Complete Authorize Success", body = PaymentsResponse),
      (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Complete Authorize a Payment",
  security(("publishable_key" = []))
)]
pub fn payments_complete_authorize() {}

/// Dynamic Tax Calculation
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/calculate_tax",
    request_body=PaymentsDynamicTaxCalculationRequest,
    responses(
        (status = 200, description = "Tax Calculation is done", body = PaymentsDynamicTaxCalculationResponse),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Create Tax Calculation for a Payment",
    security(("publishable_key" = []))
)]

pub fn payments_dynamic_tax_calculation() {}

/// Payments - Post Session Tokens
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/post_session_tokens",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsPostSessionTokensRequest,
    responses(
        (status = 200, description = "Post Session Token is done", body = PaymentsPostSessionTokensResponse),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Create Post Session Tokens for a Payment",
    security(("publishable_key" = []))
)]

pub fn payments_post_session_tokens() {}

/// Payments - Update Metadata
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/update_metadata",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsUpdateMetadataRequest,
    responses(
        (status = 200, description = "Metadata updated successfully", body = PaymentsUpdateMetadataResponse),
        (status = 400, description = "Missing mandatory fields", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Update Metadata for a Payment",
    security(("api_key" = []))
)]
pub fn payments_update_metadata() {}

/// Payments - Submit Eligibility Data
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/eligibility",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsEligibilityRequest,
    responses(
        (status = 200, description = "Eligbility submit is successful", body = PaymentsEligibilityResponse),
        (status = 400, description = "Bad Request", body = GenericErrorResponseOpenApi)
    ),
    tag = "Payments",
    operation_id = "Submit Eligibility data for a Payment",
    security(("publishable_key" = []))
)]
pub fn payments_submit_eligibility() {}

/// Payments - Create Intent
///
/// **Creates a payment intent object when amount_details are passed.**
///
/// You will require the 'API - Key' from the Hyperswitch dashboard to make the first call, and use the 'client secret' returned in this API along with your 'publishable key' to make subsequent API calls from your client.
#[utoipa::path(
  post,
  path = "/v2/payments/create-intent",
  request_body(
      content = PaymentsCreateIntentRequest,
      examples(
          (
              "Create a payment intent with minimal fields" = (
                  value = json!({"amount_details": {"order_amount": 6540, "currency": "USD"}})
              )
          ),
      ),
  ),
  responses(
      (status = 200, description = "Payment created", body = PaymentsIntentResponse),
      (status = 400, description = "Missing Mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Create a Payment Intent",
  security(("api_key" = [])),
)]
#[cfg(feature = "v2")]
pub fn payments_create_intent() {}

/// Payments - Get Intent
///
/// **Get a payment intent object when id is passed in path**
///
/// You will require the 'API - Key' from the Hyperswitch dashboard to make the call.
#[utoipa::path(
  get,
  path = "/v2/payments/{id}/get-intent",
  params (("id" = String, Path, description = "The unique identifier for the Payment Intent")),
  responses(
      (status = 200, description = "Payment Intent", body = PaymentsIntentResponse),
      (status = 404, description = "Payment Intent not found")
  ),
  tag = "Payments",
  operation_id = "Get the Payment Intent details",
  security(("api_key" = [])),
)]
#[cfg(feature = "v2")]
pub fn payments_get_intent() {}

/// Payments - Update Intent
///
/// **Update a payment intent object**
///
/// You will require the 'API - Key' from the Hyperswitch dashboard to make the call.
#[utoipa::path(
  put,
  path = "/v2/payments/{id}/update-intent",
  params (("id" = String, Path, description = "The unique identifier for the Payment Intent"),
      (
        "X-Profile-Id" = String, Header,
        description = "Profile ID associated to the payment intent",
        example = "pro_abcdefghijklmnop"
      ),
    ),
  request_body(
      content = PaymentsUpdateIntentRequest,
      examples(
          (
              "Update a payment intent with minimal fields" = (
                  value = json!({"amount_details": {"order_amount": 6540, "currency": "USD"}})
              )
          ),
      ),
  ),
  responses(
      (status = 200, description = "Payment Intent Updated", body = PaymentsIntentResponse),
      (status = 404, description = "Payment Intent Not Found")
  ),
  tag = "Payments",
  operation_id = "Update a Payment Intent",
  security(("api_key" = [])),
)]
#[cfg(feature = "v2")]
pub fn payments_update_intent() {}

/// Payments - Confirm Intent
///
/// **Confirms a payment intent object with the payment method data**
///
/// .
#[utoipa::path(
  post,
  path = "/v2/payments/{id}/confirm-intent",
  params (("id" = String, Path, description = "The unique identifier for the Payment Intent"),
      (
        "X-Profile-Id" = String, Header,
        description = "Profile ID associated to the payment intent",
        example = "pro_abcdefghijklmnop"
      )
    ),
  request_body(
      content = PaymentsConfirmIntentRequest,
      examples(
          (
              "Confirm the payment intent with card details" = (
                  value = json!({
                    "payment_method_type": "card",
                    "payment_method_subtype": "credit",
                    "payment_method_data": {
                      "card": {
                        "card_number": "4242424242424242",
                        "card_exp_month": "10",
                        "card_exp_year": "25",
                        "card_holder_name": "joseph Doe",
                        "card_cvc": "123"
                      }
                    },
                  })
              )
          ),
      ),
  ),
  responses(
      (status = 200, description = "Payment created", body = PaymentsResponse),
      (status = 400, description = "Missing Mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Confirm Payment Intent",
  security(("publishable_key" = [])),
)]
#[cfg(feature = "v2")]
pub fn payments_confirm_intent() {}

/// Payments - Get
///
/// Retrieves a Payment. This API can also be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/v2/payments/{id}",
    params(
        ("id" = String, Path, description = "The global payment id"),
        ("force_sync" = ForceSync, Query, description = "A boolean to indicate whether to force sync the payment status. Value can be true or false")
    ),
    responses(
        (status = 200, description = "Gets the payment with final status", body = PaymentsResponse),
        (status = 404, description = "No payment found with the given id")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment",
    security(("api_key" = []))
)]
#[cfg(feature = "v2")]
pub fn payment_status() {}

/// Payments - Create and Confirm Intent
///
/// **Creates and confirms a payment intent object when the amount and payment method information are passed.**
///
/// You will require the 'API - Key' from the Hyperswitch dashboard to make the call.
#[utoipa::path(
  post,
  path = "/v2/payments",
  params (
      (
        "X-Profile-Id" = String, Header,
        description = "Profile ID associated to the payment intent",
        example = "pro_abcdefghijklmnop"
      )
    ),
  request_body(
      content = PaymentsRequest,
      examples(
          (
              "Create and confirm the payment intent with amount and card details" = (
                  value = json!({
                    "amount_details": {
                      "order_amount": 6540,
                      "currency": "USD"
                    },
                    "payment_method_type": "card",
                    "payment_method_subtype": "credit",
                    "payment_method_data": {
                      "card": {
                        "card_number": "4242424242424242",
                        "card_exp_month": "10",
                        "card_exp_year": "25",
                        "card_holder_name": "joseph Doe",
                        "card_cvc": "123"
                      }
                    },
                  })
              )
          ),
      ),
  ),
  responses(
      (status = 200, description = "Payment created", body = PaymentsResponse),
      (status = 400, description = "Missing Mandatory fields", body = GenericErrorResponseOpenApi)
  ),
  tag = "Payments",
  operation_id = "Create and Confirm Payment Intent",
  security(("api_key" = [])),
)]
pub fn payments_create_and_confirm_intent() {}

#[derive(utoipa::ToSchema)]
#[schema(rename_all = "lowercase")]
pub(crate) enum ForceSync {
    /// Force sync with the connector / processor to update the status
    True,
    /// Do not force sync with the connector / processor. Get the status which is available in the database
    False,
}

/// Payments - Payment Methods List
///
/// List the payment methods eligible for a payment. This endpoint also returns the saved payment methods for the customer when the customer_id is passed when creating the payment
#[cfg(feature = "v2")]
#[utoipa::path(
    get,
    path = "/v2/payments/{id}/payment-methods",
    params(
        ("id" = String, Path, description = "The global payment id"),
        (
          "X-Profile-Id" = String, Header,
          description = "Profile ID associated to the payment intent",
          example = "pro_abcdefghijklmnop"
        ),
    ),
    responses(
        (status = 200, description = "Get the payment methods", body = PaymentMethodListResponseForPayments),
        (status = 404, description = "No payment found with the given id")
    ),
    tag = "Payments",
    operation_id = "Retrieve Payment methods for a Payment",
    security(("publishable_key" = []))
)]
pub fn list_payment_methods() {}

/// Payments - List
///
/// To list the *payments*
#[cfg(feature = "v2")]
#[utoipa::path(
    get,
    path = "/v2/payments/list",
    params(api_models::payments::PaymentListConstraints),
    responses(
        (status = 200, description = "Successfully retrieved a payment list", body = PaymentListResponse),
        (status = 404, description = "No payments found")
    ),
    tag = "Payments",
    operation_id = "List all Payments",
    security(("api_key" = []), ("jwt_key" = []))
)]
pub fn payments_list() {}

/// Payments - Check Balance and Apply PM Data
///
/// Check the balance of the payment methods, apply the payment method data and recalculate remaining_amount and surcharge
#[cfg(feature = "v2")]
#[utoipa::path(
    post,
    path = "/v2/payments/{id}/eligibility/check-balance-and-apply-pm-data",
    params(
        ("id" = String, Path, description = "The global payment id"),
        (
          "X-Profile-Id" = String, Header,
          description = "Profile ID associated to the payment intent",
          example = "pro_abcdefghijklmnop"
        ),
    ),
    request_body(
      content = ApplyPaymentMethodDataRequest,
    ),
    responses(
        (status = 200, description = "Apply the Payment Method Data", body = CheckAndApplyPaymentMethodDataResponse),
    ),
    tag = "Payments",
    operation_id = "Apply Payment Method Data",
    security(("publishable_key" = []))
)]
pub fn payments_apply_pm_data() {}
