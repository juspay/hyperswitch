#[cfg(feature = "v1")]
/// Telephone Order - Charge
///
/// Charges a card that an IVR provider collected over the phone, or stores one without charging it. Only `twilio` is supported as `{provider}`.
#[utoipa::path(
    post,
    path = "/telephone_order/{merchant_id}/{provider}",
    params(
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account the card is charged under", example = "merchant_1791294465"),
        ("provider" = String, Path, description = "The telephone order provider posting the request. Currently only `twilio`.", example = "twilio"),
    ),
    request_body(
        content = TwilioGenericPayRequest,
        description = "The provider's request. Shown here in Twilio's Generic Pay Connector shape. `method` follows the `<Pay>` verb's `chargeAmount`: `0` or absent makes it a `tokenize`. Storing a card — a `tokenize`, or a charge carrying `parameters.setup_future_usage` — additionally needs `parameters.customer_id` and `parameters.customer_acceptance`.",
        examples(
            ("01. Charge a card collected over the phone" = (
                value = json!({
                    "method": "charge",
                    "transaction_id": "IP1234",
                    "cardnumber": "4242424242424242",
                    "cvv": "123",
                    "expiry_month": "12",
                    "expiry_year": "28",
                    "amount": "1.00",
                    "currency_code": "USD",
                    "postal_code": "94122",
                    "description": "Order placed over the phone",
                    "parameters": {
                        "payment_id": "pay_1234",
                        "profile_id": "pro_1234"
                    }
                })
            )),
            ("02. Charge the card and keep it for later" = (
                value = json!({
                    "method": "charge",
                    "transaction_id": "IP1234",
                    "cardnumber": "4242424242424242",
                    "cvv": "123",
                    "expiry_month": "12",
                    "expiry_year": "28",
                    "amount": "1.00",
                    "currency_code": "USD",
                    "description": "Order placed over the phone",
                    "parameters": {
                        "customer_id": "cus_1234",
                        "setup_future_usage": "off_session",
                        "customer_acceptance": "offline",
                        "acceptance_timestamp": "2026-11-26T10:11:12Z"
                    }
                })
            )),
            ("03. Store a card without charging it" = (
                value = json!({
                    "method": "tokenize",
                    "transaction_id": "IP1234",
                    "cardnumber": "4242424242424242",
                    "cvv": "123",
                    "expiry_month": "12",
                    "expiry_year": "28",
                    "postal_code": "94122",
                    "parameters": {
                        "customer_id": "cus_1234",
                        "currency": "USD",
                        "customer_acceptance": "offline"
                    }
                })
            ))
        )
    ),
    responses(
        (status = 200, description = "The outcome of the operation, in the provider's response shape.", body = TwilioGenericPayResponse,
            examples(
                ("01. The card was charged" = (
                    value = json!({
                        "charge_id": "pay_1234",
                        "error_code": null,
                        "error_message": null,
                        "parameters": {
                            "payment_id": "pay_1234",
                            "status": "succeeded",
                            "amount_capturable": "0",
                            "amount_received": "100",
                            "connector": "stripe",
                            "connector_transaction_id": "pi_1234",
                            "merchant_order_reference_id": "IP1234"
                        }
                    })
                )),
                ("02. The card was charged and kept" = (
                    value = json!({
                        "charge_id": "pay_1234",
                        "error_code": null,
                        "error_message": null,
                        "parameters": {
                            "payment_id": "pay_1234",
                            "status": "succeeded",
                            "amount_capturable": "0",
                            "amount_received": "100",
                            "connector": "stripe",
                            "connector_transaction_id": "pi_1234",
                            "payment_method_id": "pm_1234",
                            "merchant_order_reference_id": "IP1234"
                        }
                    })
                )),
                ("03. The card was stored without a charge" = (
                    value = json!({
                        "token_id": "pm_1234",
                        "error_code": null,
                        "error_message": null,
                        "parameters": {
                            "payment_id": "pay_1234",
                            "status": "succeeded",
                            "amount_capturable": "0",
                            "connector": "stripe",
                            "connector_transaction_id": "seti_1234",
                            "payment_method_id": "pm_1234",
                            "merchant_order_reference_id": "IP1234"
                        }
                    })
                )),
                ("04. The card was declined" = (
                    value = json!({
                        "error_code": "card_declined",
                        "error_message": "Your card was declined.",
                        "parameters": {
                            "payment_id": "pay_1234",
                            "status": "failed",
                            "amount_capturable": "0",
                            "connector": "stripe",
                            "unified_code": "UE_9000",
                            "merchant_order_reference_id": "IP1234"
                        }
                    })
                )),
                ("05. The charge has no final outcome" = (
                    value = json!({
                        "error_code": "PAYMENT_PENDING",
                        "error_message": "Payment is not yet complete",
                        "parameters": {
                            "payment_id": "pay_1234",
                            "status": "processing",
                            "amount_capturable": "0",
                            "connector": "stripe",
                            "merchant_order_reference_id": "IP1234"
                        }
                    })
                )),
                ("06. The IVR did not take the caller's consent" = (
                    value = json!({
                        "error_code": "ACCEPTANCE_REQUIRED",
                        "error_message": "customer_acceptance is required to store a card; ask the caller to agree and pass it as a <Parameter>",
                        "parameters": {}
                    })
                )),
                ("07. The card has no customer to be stored against" = (
                    value = json!({
                        "error_code": "CUSTOMER_REQUIRED",
                        "error_message": "customer_id is required to store a card; pass it as a <Parameter>",
                        "parameters": {}
                    })
                ))
            )
        ),
        (status = 400, description = "The request was rejected before a charge was attempted: the API key is missing or does not belong to this merchant, the provider in the path is unknown, or the body is not a valid charge request for that provider.", body = GenericErrorResponseOpenApi,
            example = json!({
                "type": "invalid_request",
                "message": "Missing required param: transaction_id",
                "code": "IR_04"
            })
        ),
    ),
    tag = "Telephone Order",
    operation_id = "Charge a telephone order",
    security(("basic_api_key" = []))
)]
pub async fn telephone_order_charge() {}
