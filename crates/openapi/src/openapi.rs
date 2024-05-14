use crate::routes;

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
| Production    | <https://api.hyperswitch.io>       |

## Authentication

When you sign up on our [dashboard](https://app.hyperswitch.io) and create a merchant
account, you are given a secret key (also referred as api-key) and a publishable key.
You may authenticate all API requests with Hyperswitch server by providing the appropriate key in
the request Authorization header.

| Key             |  Description                                                                                  |
|-----------------|-----------------------------------------------------------------------------------------------|
| api-key         | Private key. Used to authenticate all API requests from your merchant server                  |
| publishable key | Unique identifier for your account. Used to authenticate API requests from your app's client  |

Never share your secret api keys. Keep them guarded and secure.
"#,
    ),
    servers(
        (url = "https://sandbox.hyperswitch.io", description = "Sandbox Environment")
    ),
    tags(
        (name = "Merchant Account", description = "Create and manage merchant accounts"),
        (name = "Business Profile", description = "Create and manage business profiles"),
        (name = "Merchant Connector Account", description = "Create and manage merchant connector accounts"),
        (name = "Payments", description = "Create and manage one-time payments, recurring payments and mandates"),
        (name = "Refunds", description = "Create and manage refunds for successful payments"),
        (name = "Mandates", description = "Manage mandates"),
        (name = "Customers", description = "Create and manage customers"),
        (name = "Payment Methods", description = "Create and manage payment methods of customers"),
        (name = "Disputes", description = "Manage disputes"),
        (name = "API Key", description = "Create and manage API Keys"),
        (name = "Payouts", description = "Create and manage payouts"),
        (name = "payment link", description = "Create payment link"),
        (name = "Routing", description = "Create and manage routing configurations"),
        (name = "Event", description = "Manage events"),
    ),
    // The paths will be displayed in the same order as they are registered here
    paths(
        // Routes for payments
        routes::payments::payments_create,
        routes::payments::payments_update,
        routes::payments::payments_confirm,
        routes::payments::payments_retrieve,
        routes::payments::payments_capture,
        routes::payments::payments_connector_session,
        routes::payments::payments_cancel,
        routes::payments::payments_list,
        routes::payments::payments_incremental_authorization,
        routes::payment_link::payment_link_retrieve,
        routes::payments::payments_external_authentication,

        // Routes for refunds
        routes::refunds::refunds_create,
        routes::refunds::refunds_retrieve,
        routes::refunds::refunds_update,
        routes::refunds::refunds_list,

        // Routes for merchant account
        routes::merchant_account::merchant_account_create,
        routes::merchant_account::retrieve_merchant_account,
        routes::merchant_account::update_merchant_account,
        routes::merchant_account::delete_merchant_account,
        routes::merchant_account::merchant_account_kv_status,

        // Routes for merchant connector account
        routes::merchant_connector_account::payment_connector_create,
        routes::merchant_connector_account::payment_connector_retrieve,
        routes::merchant_connector_account::payment_connector_list,
        routes::merchant_connector_account::payment_connector_update,
        routes::merchant_connector_account::payment_connector_delete,

        //Routes for gsm
        routes::gsm::create_gsm_rule,
        routes::gsm::get_gsm_rule,
        routes::gsm::update_gsm_rule,
        routes::gsm::delete_gsm_rule,

        // Routes for mandates
        routes::mandates::get_mandate,
        routes::mandates::revoke_mandate,
        routes::mandates::customers_mandates_list,

        //Routes for customers
        routes::customers::customers_create,
        routes::customers::customers_retrieve,
        routes::customers::customers_list,
        routes::customers::customers_update,
        routes::customers::customers_delete,

        //Routes for payment methods
        routes::payment_method::create_payment_method_api,
        routes::payment_method::list_payment_method_api,
        routes::payment_method::list_customer_payment_method_api,
        routes::payment_method::list_customer_payment_method_api_client,
        routes::payment_method::default_payment_method_set_api,
        routes::payment_method::payment_method_retrieve_api,
        routes::payment_method::payment_method_update_api,
        routes::payment_method::payment_method_delete_api,

        // Routes for Business Profile
        routes::business_profile::business_profile_create,
        routes::business_profile::business_profiles_list,
        routes::business_profile::business_profiles_retrieve,
        routes::business_profile::business_profiles_update,
        routes::business_profile::business_profiles_delete,

        // Routes for disputes
        routes::disputes::retrieve_dispute,
        routes::disputes::retrieve_disputes_list,

        // Routes for routing
        routes::routing::routing_create_config,
        routes::routing::routing_link_config,
        routes::routing::routing_retrieve_config,
        routes::routing::list_routing_configs,
        routes::routing::routing_unlink_config,
        routes::routing::routing_update_default_config,
        routes::routing::routing_retrieve_default_config,
        routes::routing::routing_retrieve_linked_config,
        routes::routing::routing_retrieve_default_config_for_profiles,
        routes::routing::routing_update_default_config_for_profile,

        // Routes for blocklist
        routes::blocklist::remove_entry_from_blocklist,
        routes::blocklist::list_blocked_payment_methods,
        routes::blocklist::add_entry_to_blocklist,
        routes::blocklist::toggle_blocklist_guard,

        // Routes for payouts
        routes::payouts::payouts_create,
        routes::payouts::payouts_retrieve,
        routes::payouts::payouts_update,
        routes::payouts::payouts_cancel,
        routes::payouts::payouts_fulfill,
        routes::payouts::payouts_list,
        routes::payouts::payouts_filter,

        // Routes for api keys
        routes::api_keys::api_key_create,
        routes::api_keys::api_key_retrieve,
        routes::api_keys::api_key_update,
        routes::api_keys::api_key_revoke,

        // Routes for events
        routes::webhook_events::list_initial_webhook_delivery_attempts,
        routes::webhook_events::list_webhook_delivery_attempts,
        routes::webhook_events::retry_webhook_delivery_attempt,

        // Routes for poll apis
        routes::poll::retrieve_poll_status,
    ),
    components(schemas(
        api_models::refunds::RefundRequest,
        api_models::refunds::RefundType,
        api_models::refunds::RefundResponse,
        api_models::refunds::RefundStatus,
        api_models::refunds::RefundUpdateRequest,
        api_models::admin::MerchantAccountCreate,
        api_models::admin::MerchantAccountUpdate,
        api_models::admin::MerchantAccountDeleteResponse,
        api_models::admin::MerchantConnectorDeleteResponse,
        api_models::admin::MerchantConnectorResponse,
        api_models::admin::AuthenticationConnectorDetails,
        api_models::admin::ExtendedCardInfoConfig,
        api_models::customers::CustomerRequest,
        api_models::customers::CustomerDeleteResponse,
        api_models::payment_methods::PaymentMethodCreate,
        api_models::payment_methods::PaymentMethodResponse,
        api_models::payment_methods::PaymentMethodList,
        api_models::payment_methods::CustomerPaymentMethod,
        api_models::payment_methods::PaymentMethodListResponse,
        api_models::payment_methods::CustomerPaymentMethodsListResponse,
        api_models::payment_methods::PaymentMethodDeleteResponse,
        api_models::payment_methods::PaymentMethodUpdate,
        api_models::payment_methods::CustomerDefaultPaymentMethodResponse,
        api_models::payment_methods::CardDetailFromLocker,
        api_models::payment_methods::PaymentMethodCreateData,
        api_models::payment_methods::CardDetail,
        api_models::payment_methods::CardDetailUpdate,
        api_models::payment_methods::RequestPaymentMethodTypes,
        api_models::poll::PollResponse,
        api_models::poll::PollStatus,
        api_models::customers::CustomerResponse,
        api_models::admin::AcceptedCountries,
        api_models::admin::AcceptedCurrencies,
        api_models::enums::PaymentType,
        api_models::enums::PaymentMethod,
        api_models::enums::PaymentMethodType,
        api_models::enums::ConnectorType,
        api_models::enums::PayoutConnectors,
        api_models::enums::AuthenticationConnectors,
        api_models::enums::Currency,
        api_models::enums::IntentStatus,
        api_models::enums::CaptureMethod,
        api_models::enums::FutureUsage,
        api_models::enums::AuthenticationType,
        api_models::enums::Connector,
        api_models::enums::PaymentMethod,
        api_models::enums::PaymentMethodIssuerCode,
        api_models::enums::MandateStatus,
        api_models::enums::PaymentExperience,
        api_models::enums::BankNames,
        api_models::enums::CardNetwork,
        api_models::enums::DisputeStage,
        api_models::enums::DisputeStatus,
        api_models::enums::CountryAlpha2,
        api_models::enums::FieldType,
        api_models::enums::FrmAction,
        api_models::enums::FrmPreferredFlowTypes,
        api_models::enums::RetryAction,
        api_models::enums::AttemptStatus,
        api_models::enums::CaptureStatus,
        api_models::enums::ReconStatus,
        api_models::enums::ConnectorStatus,
        api_models::enums::AuthorizationStatus,
        api_models::enums::PaymentMethodStatus,
        api_models::admin::MerchantConnectorCreate,
        api_models::admin::MerchantConnectorUpdate,
        api_models::admin::PrimaryBusinessDetails,
        api_models::admin::FrmConfigs,
        api_models::admin::FrmPaymentMethod,
        api_models::admin::FrmPaymentMethodType,
        api_models::admin::PaymentMethodsEnabled,
        api_models::admin::MerchantConnectorDetailsWrap,
        api_models::admin::MerchantConnectorDetails,
        api_models::admin::MerchantConnectorWebhookDetails,
        api_models::admin::BusinessProfileCreate,
        api_models::admin::BusinessProfileResponse,
        api_models::admin::BusinessPaymentLinkConfig,
        api_models::admin::PaymentLinkConfigRequest,
        api_models::admin::PaymentLinkConfig,
        api_models::disputes::DisputeResponse,
        api_models::disputes::DisputeResponsePaymentsRetrieve,
        api_models::gsm::GsmCreateRequest,
        api_models::gsm::GsmRetrieveRequest,
        api_models::gsm::GsmUpdateRequest,
        api_models::gsm::GsmDeleteRequest,
        api_models::gsm::GsmDeleteResponse,
        api_models::gsm::GsmResponse,
        api_models::gsm::GsmDecision,
        api_models::payments::AddressDetails,
        api_models::payments::BankDebitData,
        api_models::payments::AliPayQr,
        api_models::payments::AliPayRedirection,
        api_models::payments::MomoRedirection,
        api_models::payments::TouchNGoRedirection,
        api_models::payments::GcashRedirection,
        api_models::payments::KakaoPayRedirection,
        api_models::payments::AliPayHkRedirection,
        api_models::payments::GoPayRedirection,
        api_models::payments::MbWayRedirection,
        api_models::payments::MobilePayRedirection,
        api_models::payments::WeChatPayRedirection,
        api_models::payments::WeChatPayQr,
        api_models::payments::BankDebitBilling,
        api_models::payments::CryptoData,
        api_models::payments::RewardData,
        api_models::payments::UpiData,
        api_models::payments::VoucherData,
        api_models::payments::BoletoVoucherData,
        api_models::payments::AlfamartVoucherData,
        api_models::payments::IndomaretVoucherData,
        api_models::payments::Address,
        api_models::payments::VoucherData,
        api_models::payments::JCSVoucherData,
        api_models::payments::AlfamartVoucherData,
        api_models::payments::IndomaretVoucherData,
        api_models::payments::BankRedirectData,
        api_models::payments::BankRedirectBilling,
        api_models::payments::BankRedirectBilling,
        api_models::payments::ConnectorMetadata,
        api_models::payments::FeatureMetadata,
        api_models::payments::ApplepayConnectorMetadataRequest,
        api_models::payments::SessionTokenInfo,
        api_models::payments::SwishQrData,
        api_models::payments::AirwallexData,
        api_models::payments::NoonData,
        api_models::payments::OrderDetails,
        api_models::payments::OrderDetailsWithAmount,
        api_models::payments::NextActionType,
        api_models::payments::WalletData,
        api_models::payments::NextActionData,
        api_models::payments::PayLaterData,
        api_models::payments::MandateData,
        api_models::payments::PhoneDetails,
        api_models::payments::PaymentMethodData,
        api_models::payments::PaymentMethodDataRequest,
        api_models::payments::MandateType,
        api_models::payments::AcceptanceType,
        api_models::payments::MandateAmountData,
        api_models::payments::OnlineMandate,
        api_models::payments::Card,
        api_models::payments::CardRedirectData,
        api_models::payments::CardToken,
        api_models::payments::CustomerAcceptance,
        api_models::payments::PaymentsRequest,
        api_models::payments::PaymentsCreateRequest,
        api_models::payments::PaymentsUpdateRequest,
        api_models::payments::PaymentsConfirmRequest,
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
        api_models::payments::ThirdPartySdkSessionResponse,
        api_models::payments::NoThirdPartySdkSessionResponse,
        api_models::payments::SecretInfoToInitiateSdk,
        api_models::payments::ApplePayPaymentRequest,
        api_models::payments::AmountInfo,
        api_models::payments::ProductType,
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
        api_models::payments::GooglePayThirdPartySdkData,
        api_models::payments::KlarnaSessionTokenResponse,
        api_models::payments::PaypalSessionTokenResponse,
        api_models::payments::ApplepaySessionTokenResponse,
        api_models::payments::SdkNextAction,
        api_models::payments::NextActionCall,
        api_models::payments::SamsungPayWalletData,
        api_models::payments::WeChatPay,
        api_models::payments::GpayTokenizationData,
        api_models::payments::GooglePayPaymentMethodInfo,
        api_models::payments::ApplePayWalletData,
        api_models::payments::ApplepayPaymentMethod,
        api_models::payments::PaymentsCancelRequest,
        api_models::payments::PaymentListConstraints,
        api_models::payments::PaymentListResponse,
        api_models::payments::CashappQr,
        api_models::payments::BankTransferData,
        api_models::payments::BankTransferNextStepsData,
        api_models::payments::SepaAndBacsBillingDetails,
        api_models::payments::AchBillingDetails,
        api_models::payments::MultibancoBillingDetails,
        api_models::payments::DokuBillingDetails,
        api_models::payments::BankTransferInstructions,
        api_models::payments::ReceiverDetails,
        api_models::payments::AchTransfer,
        api_models::payments::MultibancoTransferInstructions,
        api_models::payments::DokuBankTransferInstructions,
        api_models::payments::ApplePayRedirectData,
        api_models::payments::ApplePayThirdPartySdkData,
        api_models::payments::GooglePayRedirectData,
        api_models::payments::GooglePayThirdPartySdk,
        api_models::payments::GooglePaySessionResponse,
        api_models::payments::SepaBankTransferInstructions,
        api_models::payments::BacsBankTransferInstructions,
        api_models::payments::RedirectResponse,
        api_models::payments::RequestSurchargeDetails,
        api_models::payments::PaymentAttemptResponse,
        api_models::payments::CaptureResponse,
        api_models::payments::PaymentsIncrementalAuthorizationRequest,
        api_models::payments::IncrementalAuthorizationResponse,
        api_models::payments::PaymentsExternalAuthenticationRequest,
        api_models::payments::PaymentsExternalAuthenticationResponse,
        api_models::payments::SdkInformation,
        api_models::payments::DeviceChannel,
        api_models::payments::ThreeDsCompletionIndicator,
        api_models::enums::TransactionStatus,
        api_models::payments::BrowserInformation,
        api_models::payments::PaymentCreatePaymentLinkConfig,
        api_models::payments::ThreeDsData,
        api_models::payments::ThreeDsMethodData,
        api_models::payments::PollConfigResponse,
        api_models::payments::ExternalAuthenticationDetailsResponse,
        api_models::payments::ExtendedCardInfo,
        api_models::payment_methods::RequiredFieldInfo,
        api_models::payment_methods::DefaultPaymentMethod,
        api_models::payment_methods::MaskedBankDetails,
        api_models::payment_methods::SurchargeDetailsResponse,
        api_models::payment_methods::SurchargeResponse,
        api_models::payment_methods::SurchargePercentage,
        api_models::refunds::RefundListRequest,
        api_models::refunds::RefundListResponse,
        api_models::payments::TimeRange,
        api_models::mandates::MandateRevokedResponse,
        api_models::mandates::MandateResponse,
        api_models::mandates::MandateCardDetails,
        api_models::mandates::RecurringDetails,
        api_models::ephemeral_key::EphemeralKeyCreateResponse,
        api_models::payments::CustomerDetails,
        api_models::payments::GiftCardData,
        api_models::payments::GiftCardDetails,
        api_models::payments::Address,
        api_models::payouts::Card,
        api_models::payouts::Wallet,
        api_models::payouts::Paypal,
        api_models::payouts::Venmo,
        api_models::payouts::AchBankTransfer,
        api_models::payouts::BacsBankTransfer,
        api_models::payouts::SepaBankTransfer,
        api_models::payouts::PixBankTransfer,
        api_models::payouts::PayoutRequest,
        api_models::payouts::PayoutAttemptResponse,
        api_models::payouts::PayoutActionRequest,
        api_models::payouts::PayoutCreateRequest,
        api_models::payouts::PayoutCreateResponse,
        api_models::payouts::PayoutListConstraints,
        api_models::payouts::PayoutListFilterConstraints,
        api_models::payouts::PayoutListResponse,
        api_models::payouts::PayoutRetrieveBody,
        api_models::payouts::PayoutRetrieveRequest,
        api_models::payouts::PayoutMethodData,
        api_models::payouts::Bank,
        api_models::enums::PayoutEntityType,
        api_models::enums::PayoutStatus,
        api_models::enums::PayoutType,
        api_models::enums::TransactionType,
        api_models::payments::FrmMessage,
        api_models::webhooks::OutgoingWebhook,
        api_models::webhooks::OutgoingWebhookContent,
        api_models::enums::EventClass,
        api_models::enums::EventType,
        api_models::enums::DecoupledAuthenticationType,
        api_models::enums::AuthenticationStatus,
        api_models::admin::MerchantAccountResponse,
        api_models::admin::MerchantConnectorId,
        api_models::admin::MerchantDetails,
        api_models::admin::ToggleKVRequest,
        api_models::admin::ToggleKVResponse,
        api_models::admin::WebhookDetails,
        api_models::api_keys::ApiKeyExpiration,
        api_models::api_keys::CreateApiKeyRequest,
        api_models::api_keys::CreateApiKeyResponse,
        api_models::api_keys::RetrieveApiKeyResponse,
        api_models::api_keys::RevokeApiKeyResponse,
        api_models::api_keys::UpdateApiKeyRequest,
        api_models::payments::RetrievePaymentLinkRequest,
        api_models::payments::PaymentLinkResponse,
        api_models::payments::RetrievePaymentLinkResponse,
        api_models::payments::PaymentLinkInitiateRequest,
        api_models::payments::ExtendedCardInfoResponse,
        api_models::routing::RoutingConfigRequest,
        api_models::routing::RoutingDictionaryRecord,
        api_models::routing::RoutingKind,
        api_models::routing::RoutableConnectorChoice,
        api_models::routing::LinkedRoutingConfigRetrieveResponse,
        api_models::routing::RoutingRetrieveResponse,
        api_models::routing::ProfileDefaultRoutingConfig,
        api_models::routing::MerchantRoutingAlgorithm,
        api_models::routing::RoutingAlgorithmKind,
        api_models::routing::RoutingDictionary,
        api_models::routing::RoutingAlgorithm,
        api_models::routing::StraightThroughAlgorithm,
        api_models::routing::ConnectorVolumeSplit,
        api_models::routing::ConnectorSelection,
        api_models::routing::ast::RoutableChoiceKind,
        api_models::enums::RoutableConnectors,
        api_models::routing::ast::ProgramConnectorSelection,
        api_models::routing::ast::RuleConnectorSelection,
        api_models::routing::ast::IfStatement,
        api_models::routing::ast::Comparison,
        api_models::routing::ast::ComparisonType,
        api_models::routing::ast::ValueType,
        api_models::routing::ast::MetadataValue,
        api_models::routing::ast::NumberComparison,
        api_models::payment_methods::RequestPaymentMethodTypes,
        api_models::payments::PaymentLinkStatus,
        api_models::blocklist::BlocklistRequest,
        api_models::blocklist::BlocklistResponse,
        api_models::blocklist::ToggleBlocklistResponse,
        api_models::blocklist::ListBlocklistQuery,
        api_models::enums::BlocklistDataKind,
        api_models::webhook_events::EventListItemResponse,
        api_models::webhook_events::EventRetrieveResponse,
        api_models::webhook_events::OutgoingWebhookRequestContent,
        api_models::webhook_events::OutgoingWebhookResponseContent,
        api_models::enums::WebhookDeliveryAttempt,
    )),
    modifiers(&SecurityAddon)
)]
// Bypass clippy lint for not being constructed
#[allow(dead_code)]
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
                        "Use the API key created under your merchant account from the HyperSwitch dashboard. API key is used to authenticate API requests from your merchant server only. Don't expose this key on a website or embed it in a mobile application."
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
