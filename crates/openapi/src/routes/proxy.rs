#[cfg(feature = "v2")]
///Proxy
///
/// Create a proxy request
#[utoipa::path(
    post,
    path = "/proxy",
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

pub async fn proxy_core() {}
