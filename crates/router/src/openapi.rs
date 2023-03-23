#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "Hyperswitch - API Documentation",
        contact(
            name = "Hyperswitch Support",
            url = "https://hyperswitch.io",
            email = "hyperswitch@juspay.in"
        ),
        // terms_of_service = "https://www.juspay.io/terms",
        description = r#"
## Get started

Hyperswitch provides a collection of APIs that enable you to process and manage payments.
Our APIs accept and return JSON in the HTTP body, and return standard HTTP response codes.

You can consume the APIs directly using your favorite HTTP/REST library.

We have a testing environment referred to "sandbox", which you can setup to test API calls without
affecting production data.
Currently, our sandbox environment is live while our production environment is under development
and will be available soon.
You can sign up on our Dashboard to get API keys to access Hyperswitch API.

### Environment

Use the following base URLs when making requests to the APIs:

| Environment   |  Base URL                          |
|---------------|------------------------------------|
| Sandbox       | <https://sandbox.hyperswitch.io>   |
| Production    | Coming Soon!                       |

## Authentication

When you sign up on our [dashboard](https://app.hyperswitch.io) and create a merchant
account, you are given a secret key (also referred as api-key) and a publishable key.
You may authenticate all API requests with Hyperswitch server by providing the appropriate key in
the request Authorization header.

| Key           |  Description                                                                                  |
|---------------|-----------------------------------------------------------------------------------------------|
| Sandbox       | Private key. Used to authenticate all API requests from your merchant server                  |
| Production    | Unique identifier for your account. Used to authenticate API requests from your app's client  |

Never share your secret api keys. Keep them guarded and secure.
"#,
    ),
    servers(
        (url = "https://sandbox.hyperswitch.io", description = "Sandbox Environment")
    ),
    tags(
        (name = "Merchant Account", description = "Create and manage merchant accounts"),
        (name = "Merchant Connector Account", description = "Create and manage merchant connector accounts"),
        (name = "Payments", description = "Create and manage one-time payments, recurring payments and mandates"),
        (name = "Refunds", description = "Create and manage refunds for successful payments"),
        (name = "Mandates", description = "Manage mandates"),
        (name = "Customers", description = "Create and manage customers"),
        (name = "Payment Methods", description = "Create and manage payment methods of customers"),
        // (name = "API Key", description = "Create and manage API Keys"),
    ),
    paths(
        crate::routes::refunds::refunds_create,
        crate::routes::refunds::refunds_retrieve,
        crate::routes::refunds::refunds_update,
        crate::routes::refunds::refunds_list,
        crate::routes::admin::merchant_account_create,
        crate::routes::admin::retrieve_merchant_account,
        crate::routes::admin::update_merchant_account,
        crate::routes::admin::delete_merchant_account,
        crate::routes::admin::payment_connector_create,
        crate::routes::admin::payment_connector_retrieve,
        crate::routes::admin::payment_connector_list,
        crate::routes::admin::payment_connector_update,
        crate::routes::admin::payment_connector_delete,
        crate::routes::mandates::get_mandate,
        crate::routes::mandates::revoke_mandate,
        crate::routes::payments::payments_create,
       // crate::routes::payments::payments_start,
        crate::routes::payments::payments_retrieve,
        crate::routes::payments::payments_update,
        crate::routes::payments::payments_confirm,
        crate::routes::payments::payments_capture,
        crate::routes::payments::payments_connector_session,
       // crate::routes::payments::payments_redirect_response,
        crate::routes::payments::payments_cancel,
        crate::routes::payments::payments_list,
        crate::routes::payment_methods::create_payment_method_api,
        crate::routes::payment_methods::list_payment_method_api,
        crate::routes::payment_methods::list_customer_payment_method_api,
        crate::routes::payment_methods::payment_method_retrieve_api,
        crate::routes::payment_methods::payment_method_update_api,
        crate::routes::payment_methods::payment_method_delete_api,
        crate::routes::customers::customers_create,
        crate::routes::customers::customers_retrieve,
        crate::routes::customers::customers_update,
        crate::routes::customers::customers_delete,
        // crate::routes::api_keys::api_key_create,
        // crate::routes::api_keys::api_key_retrieve,
        // crate::routes::api_keys::api_key_update,
        // crate::routes::api_keys::api_key_revoke,
        // crate::routes::api_keys::api_key_list,
    ),
    components(schemas(
        crate::types::api::refunds::RefundRequest,
        crate::types::api::refunds::RefundType,
        crate::types::api::refunds::RefundResponse,
        crate::types::api::refunds::RefundStatus,
        crate::types::api::refunds::RefundUpdateRequest,
        crate::types::api::admin::MerchantAccountCreate,
        crate::types::api::admin::MerchantAccountUpdate,
        crate::types::api::admin::MerchantAccountDeleteResponse,
        crate::types::api::admin::MerchantConnectorDeleteResponse,
        crate::types::api::customers::CustomerRequest,
        crate::types::api::customers::CustomerDeleteResponse,
        crate::types::api::payment_methods::PaymentMethodCreate,
        crate::types::api::payment_methods::PaymentMethodResponse,
        crate::types::api::payment_methods::PaymentMethodList,
        crate::types::api::payment_methods::CustomerPaymentMethod,
        crate::types::api::payment_methods::PaymentMethodListResponse,
        crate::types::api::payment_methods::CustomerPaymentMethodsListResponse,
        crate::types::api::payment_methods::PaymentMethodDeleteResponse,
        crate::types::api::payment_methods::PaymentMethodUpdate,
        crate::types::api::payment_methods::CardDetailFromLocker,
        crate::types::api::payment_methods::CardDetail,
        api_models::customers::CustomerResponse,
        api_models::admin::AcceptedCountries,
        api_models::admin::AcceptedCurrencies,
        api_models::enums::RoutingAlgorithm,
        api_models::enums::PaymentMethod,
        api_models::enums::PaymentMethodType,
        api_models::enums::ConnectorType,
        api_models::enums::Currency,
        api_models::enums::IntentStatus,
        api_models::enums::CaptureMethod,
        api_models::enums::FutureUsage,
        api_models::enums::AuthenticationType,
        api_models::enums::Connector,
        api_models::enums::PaymentMethod,
        api_models::enums::SupportedWallets,
        api_models::enums::PaymentMethodIssuerCode,
        api_models::enums::MandateStatus,
        api_models::enums::PaymentExperience,
        api_models::enums::BankNames,
        api_models::enums::CardNetwork,
        api_models::admin::MerchantConnector,
        api_models::admin::PaymentMethodsEnabled,
        api_models::payments::AddressDetails,
        api_models::payments::Address,
        api_models::payments::BankRedirectData,
        api_models::payments::BankRedirectBilling,
        api_models::payments::OrderDetails,
        api_models::payments::NextActionType,
        api_models::payments::Metadata,
        api_models::payments::WalletData,
        api_models::payments::NextAction,
        api_models::payments::PayLaterData,
        api_models::payments::MandateData,
        api_models::payments::PhoneDetails,
        api_models::payments::PaymentMethodData,
        api_models::payments::MandateType,
        api_models::payments::AcceptanceType,
        api_models::payments::MandateAmountData,
        api_models::payments::OnlineMandate,
        api_models::payments::Card,
        api_models::payments::CustomerAcceptance,
        api_models::payments::PaymentsRequest,
        api_models::payments::PaymentsResponse,
        api_models::payments::PaymentsStartRequest,
        api_models::payments::PaymentRetrieveBody,
        api_models::payments::PaymentsRetrieveRequest,
        api_models::payments::PaymentIdType,
        api_models::payments::PaymentsCaptureRequest,
        api_models::payments::PaymentsSessionRequest,
        api_models::payments::PaymentsSessionResponse,
        api_models::payments::SessionToken,
        api_models::payments::ApplePaySessionResponse,
        api_models::payments::ApplePayPaymentRequest,
        api_models::payments::AmountInfo,
        api_models::payments::GooglePayWalletData,
        api_models::payments::PayPalWalletData,
        api_models::payments::PaypalRedirection,
        api_models::payments::GpayMerchantInfo,
        api_models::payments::GpayAllowedPaymentMethods,
        api_models::payments::GpayAllowedMethodsParameters,
        api_models::payments::GpayTokenizationSpecification,
        api_models::payments::GpayTokenParameters,
        api_models::payments::GpayTransactionInfo,
        api_models::payments::GpaySessionTokenResponse,
        api_models::payments::KlarnaSessionTokenResponse,
        api_models::payments::PaypalSessionTokenResponse,
        api_models::payments::ApplepaySessionTokenResponse,
        api_models::payments::GpayTokenizationData,
        api_models::payments::GooglePayPaymentMethodInfo,
        api_models::payments::ApplePayWalletData,
        api_models::payments::ApplepayPaymentMethod,
        api_models::payments::PaymentsCancelRequest,
        api_models::payments::PaymentListConstraints,
        api_models::payments::PaymentListResponse,
        api_models::refunds::RefundListRequest,
        api_models::refunds::RefundListResponse,
        api_models::mandates::MandateRevokedResponse,
        api_models::mandates::MandateResponse,
        api_models::mandates::MandateCardDetails,
        crate::types::api::admin::MerchantAccountResponse,
        crate::types::api::admin::MerchantConnectorId,
        crate::types::api::admin::MerchantDetails,
        crate::types::api::admin::WebhookDetails,
        crate::types::api::api_keys::ApiKeyExpiration,
        crate::types::api::api_keys::CreateApiKeyRequest,
        crate::types::api::api_keys::CreateApiKeyResponse,
        crate::types::api::api_keys::RetrieveApiKeyResponse,
        crate::types::api::api_keys::RevokeApiKeyResponse,
        crate::types::api::api_keys::UpdateApiKeyRequest
    )),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

        if let Some(components) = openapi.components.as_mut() {
            components.add_security_schemes_from_iter([
                (
                    "api_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                        "api-key",
                        "API keys are the most common method of authentication and can be obtained \
                         from the HyperSwitch dashboard."
                    ))),
                ),
                (
                    "admin_api_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                        "api-key",
                        "Admin API keys allow you to perform some privileged actions such as \
                         creating a merchant account and Merchant Connector account."
                    ))),
                ),
                (
                    "publishable_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                        "api-key",
                        "Publishable keys are a type of keys that can be public and have limited \
                         scope of usage."
                    ))),
                ),
                (
                    "ephemeral_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                        "api-key",
                        "Ephemeral keys provide temporary access to singular data, such as access \
                         to a single customer object for a short period of time."
                    ))),
                ),
            ]);
        }
    }
}
