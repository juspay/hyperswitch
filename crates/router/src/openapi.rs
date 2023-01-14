#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "Juspay Router - API Documentation",
        contact(
            name = "Juspay Support",
            url = "https://juspay.io",
            email = "support@juspay.in"
        ),
        // terms_of_service = "https://www.juspay.io/terms",
        description = r#"
## Get started

Juspay Router provides a collection of APIs that enable you to process and manage payments.
Our APIs accept and return JSON in the HTTP body, and return standard HTTP response codes.

You can consume the APIs directly using your favorite HTTP/REST library.

We have a testing environment referred to "sandbox", which you can setup to test API calls without
affecting production data.

### Base URLs

Use the following base URLs when making requests to the APIs:

| Environment   |  Base URL                                            |
|---------------|------------------------------------------------------|
| Sandbox       | <https://sandbox-router.juspay.io>                   |
| Production    | <https://router.juspay.io>                           |

## Authentication

When you sign up on our [dashboard](https://dashboard-hyperswitch.netlify.app) and create a merchant
account, you are given a secret key (also referred as api-key).
You may authenticate all API requests with Juspay server by providing the appropriate key in the
request Authorization header.

Never share your secret api keys. Keep them guarded and secure.
"#,
    ),
    servers(
        (url = "https://sandbox-router.juspay.io", description = "Sandbox Environment"),
        (url = "https://router.juspay.io", description = "Production Environment")
    ),
    paths(
        crate::routes::refunds::refunds_create,
        crate::routes::admin::merchant_account_create
    ),
    components(schemas(
        crate::types::api::refunds::RefundRequest,
        crate::types::api::refunds::RefundType,
        crate::types::api::refunds::RefundResponse,
        crate::types::api::refunds::RefundStatus,
        crate::types::api::admin::CreateMerchantAccount,
        crate::types::api::admin::CustomRoutingRules,
        api_models::enums::RoutingAlgorithm,
        api_models::enums::PaymentMethodType,
        api_models::enums::PaymentMethodSubType,
        api_models::enums::Currency,
        api_models::payments::AddressDetails,
        crate::types::api::admin::MerchantAccountResponse,
        crate::types::api::admin::MerchantConnectorId,
        crate::types::api::admin::MerchantDetails,
        crate::types::api::admin::WebhookDetails,
    ))
)]
pub struct ApiDoc;
