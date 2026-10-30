#[cfg(feature = "v1")]
/// Telephone Order - Charge
///
/// Charges a card that an IVR provider collected over the phone. Only `twilio` is supported as `{provider}`.
#[utoipa::path(
    post,
    path = "/telephone_order/{merchant_id}/{provider}",
    params(
        ("merchant_id" = String, Path, description = "The unique identifier for the merchant account the card is charged under", example = "merchant_1791294465"),
        ("provider" = String, Path, description = "The telephone order provider posting the request. Currently only `twilio`.", example = "twilio"),
    ),
    request_body(
        content = TwilioGenericPayRequest,
        description = "The provider's charge request. Shown here in Twilio's Generic Pay Connector shape.",
        examples((
            "Charge a card collected over the phone" = (
                value = json!({
                    "method": "charge",
                    "transaction_id": "IPe16430-39ba-4820-bedc-7aad0762a487",
                    "cardnumber": "4242424242424242",
                    "cvv": "123",
                    "expiry_month": "12",
                    "expiry_year": "28",
                    "amount": "1.00",
                    "currency_code": "USD",
                    "postal_code": "94122",
                    "description": "Order placed over the phone",
                    "parameters": {
                        "profile_id": "pro_GUMsmmzTU7oadT00pJeo"
                    }
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "The outcome of the charge, in the provider's response shape.", body = TwilioGenericPayResponse,
            examples(
                ("01. The card was charged" = (
                    value = json!({
                        "charge_id": "pay_8320b449b06630271e378214dab80ce5",
                        "error_code": null,
                        "error_message": null,
                        "parameters": {
                            "payment_id": "pay_8320b449b06630271e378214dab80ce5",
                            "status": "succeeded",
                            "amount_capturable": "0",
                            "amount_received": "100",
                            "connector": "stripe",
                            "connector_transaction_id": "pi_3URyI8D5R7gDAGff17RhZpfW",
                            "merchant_order_reference_id": "IPe16430-39ba-4820-bedc-7aad0762a487"
                        }
                    })
                )),
                ("02. The card was declined" = (
                    value = json!({
                        "charge_id": null,
                        "error_code": "card_declined",
                        "error_message": "Your card was declined.",
                        "parameters": {
                            "payment_id": "pay_8320b449b06630271e378214dab80ce5",
                            "status": "failed",
                            "connector": "stripe",
                            "unified_code": "UE_9000",
                            "merchant_order_reference_id": "IPe16430-39ba-4820-bedc-7aad0762a487"
                        }
                    })
                )),
                ("03. The charge has no final outcome" = (
                    value = json!({
                        "charge_id": null,
                        "error_code": "PAYMENT_PENDING",
                        "error_message": "Payment is not yet complete",
                        "parameters": {
                            "payment_id": "pay_8320b449b06630271e378214dab80ce5",
                            "status": "processing",
                            "connector": "stripe",
                            "merchant_order_reference_id": "IPe16430-39ba-4820-bedc-7aad0762a487"
                        }
                    })
                ))
            )
        ),
        (status = 401, description = "The API key is missing, expired, or does not belong to this merchant"),
        (status = 404, description = "Unknown telephone order provider in the path"),
        (status = 422, description = "The request body could not be deserialized for this provider"),
    ),
    tag = "Telephone Order",
    operation_id = "Charge a telephone order",
    security(("basic_api_key" = []))
)]
pub async fn telephone_order_charge() {}
