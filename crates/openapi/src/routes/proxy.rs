#[cfg(feature = "v2")]
///Proxy
///
/// Create a proxy request
#[utoipa::path(
    post,
    path = "/v2/proxy",
    request_body(
        content = ProxyRequest,
        examples((
            "Create a proxy request" = (
                value = json!({
                    "request_body": {
                        "source": {
                            "type": "card",
                            "number": "{{$card_number}}",
                            "expiry_month": "{{$card_exp_month}}",
                            "expiry_year": "{{$card_exp_year}}",
                            "billing_address": {
                                "address_line1": "123 High St.",
                                "city": "London",
                                "country": "GB"
                            }
                        },
                        "amount": 6540,
                        "currency": "USD",
                        "reference": "ORD-5023-4E89",
                        "capture": true
                    },
                    "destination_url": "https://api.example.com/payments",
                    "headers": {
                        "Content-Type": "application/json",
                        "Authorization": "Bearer sk_test_example"
                    },
                    "token": "pm_0196ea5a42a67583863d5b1253d62931",
                    "token_type": "PaymentMethodId",
                    "method": "POST"
                })
            )
        ), (
            "Create a proxy request that encrypts and signs" = (
                value = json!({
                    "request_body": {
                        "amount": { "total": 12.04, "currency": "USD" },
                        "source": {
                            "sourceType": "PaymentCard",
                            "encryptionData": {
                                "encryptionType": "RSA",
                                "encryptionTarget": "MANUAL",
                                "encryptionBlock": "{{$card_number}}{{$card_holder_name}}{{$card_exp_month}}{{$card_exp_year | exp_year_4}}{{$card_cvc}}",
                                "encryptionBlockFields": "card.cardData:{{$card_number | byte_length}},card.nameOnCard:{{$card_holder_name | byte_length}},card.expirationMonth:{{$card_exp_month | byte_length}},card.expirationYear:4,card.securityCode:{{$card_cvc | byte_length}}",
                                "keyId": "88000000022"
                            }
                        }
                    },
                    "destination_url": "https://api.example.com/payments",
                    "headers": {
                        "Content-Type": "application/json",
                        "Api-Key": "api_key_example",
                        "Client-Request-Id": "client_request_id_example",
                        "Timestamp": "1763900000000",
                        "Auth-Token-Type": "HMAC"
                    },
                    "token": "pm_example_payment_method_id",
                    "token_type": "payment_method_id",
                    "method": "POST",
                    "transformation": {
                        "steps": [
                            {
                                "operation": "encrypt",
                                "field": "/source/encryptionData/encryptionBlock",
                                "scheme": "rsa_oaep_sha256",
                                "key": "<base64 DER SubjectPublicKeyInfo public key>",
                                "encoding": "base64"
                            },
                            { "operation": "serialize", "format": "json" },
                            {
                                "operation": "sign",
                                "message": [
                                    { "header": "Api-Key" },
                                    { "header": "Client-Request-Id" },
                                    { "header": "Timestamp" },
                                    "body"
                                ],
                                "scheme": "hmac_sha256",
                                "key": "<signing secret>",
                                "encoding": "base64",
                                "header": "Authorization"
                            }
                        ]
                    }
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Proxy request", body = ProxyResponse),
        (status = 400, description = "Invalid data")
    ),
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication"),
    ),
    tag = "Proxy",
    operation_id = "Proxy Request",
    security(("api_key" = []))
)]

pub async fn proxy_core() {}
#[cfg(feature = "v2")]
///Proxy
///
/// Create a proxy request
#[utoipa::path(
    post,
    path = "/v1/proxy",
    request_body(
        content = ProxyRequest,
        examples((
            "Create a proxy request" = (
                value = json!({
                    "request_body": {
                        "source": {
                            "type": "card",
                            "number": "{{$card_number}}",
                            "expiry_month": "{{$card_exp_month}}",
                            "expiry_year": "{{$card_exp_year}}",
                            "billing_address": {
                                "address_line1": "123 High St.",
                                "city": "London",
                                "country": "GB"
                            }
                        },
                        "amount": 6540,
                        "currency": "USD",
                        "reference": "ORD-5023-4E89",
                        "capture": true
                    },
                    "destination_url": "https://api.example.com/payments",
                    "headers": {
                        "Content-Type": "application/json",
                        "Authorization": "Bearer sk_test_example"
                    },
                    "token": "pm_0196ea5a42a67583863d5b1253d62931",
                    "token_type": "PaymentMethodId",
                    "method": "POST"
                })
            )
        ))
    ),
    responses(
        (status = 200, description = "Proxy request", body = ProxyResponse),
        (status = 400, description = "Invalid data")
    ),
    params(
        ("X-Profile-Id" = String, Header, description = "Profile ID for authentication"),
    ),
    tag = "Proxy",
    operation_id = "Proxy Request",
    security(("api_key" = []))
)]

pub async fn proxy_core_v1() {}
