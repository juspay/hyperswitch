$version: "2.0"

namespace openapi_spec_v1

use alloy#dataExamples
use alloy#dateFormat
use alloy#discriminated
use alloy#untagged
use alloy.openapi#openapiExtensions
use error#AuthenticationData
use error#ComparisonMetadataItem
use error#FrmReason
use error#PaymentAttemptResponseConnectorMetadata
use error#PaymentChecks
use error#ProgramConnectorSelectionMetadataItem
use error#ProgramThreeDsDecisionRuleMetadataItem
use smithytranslate#contentType

/// Bearer Format: JWT
@openapiExtensions(
    "x-mcp": {
        enabled: true
    }
)
@httpApiKeyAuth(
    name: "api-key"
    in: "header"
)
@httpBearerAuth
service OpenapiSpecV1Service {
    operations: [
        ActivateARoutingConfig
        BlockAFingerprint
        CancelAPayment
        CancelAPayout
        CaptureAPayment
        CompleteAuthorizeAPayment
        ConfirmAPayment
        ConfirmAPayout
        CreateACustomer
        CreateAMerchantAccount
        CreateAMerchantConnector
        CreateAnAPIKey
        CreateAnAuthentication
        CreateAnOrganization
        CreateAPayment
        CreateAPaymentMethod
        CreateAPayout
        CreateAProfile
        CreateAProfileAcquirer
        CreateARefund
        CreateARoutingConfig
        CreateGsmRule
        CreatePostSessionTokensForAPayment
        CreateSessionTokensForAPayment
        DeactivateARoutingConfig
        DeleteACustomer
        DeleteAMerchantAccount
        DeleteAMerchantConnector
        DeleteAPaymentMethod
        DeleteGsmRule
        DeleteTheProfile
        EnableDisableKVForAMerchantAccount
        Execute3DSDecisionRule
        FilterPayoutsUsingSpecificConstraints
        FulfillAPayout
        IncrementAuthorizedAmountForAPayment
        InitiateExternalAuthenticationForAPayment
        ListAllAPIKeysAssociatedWithAMerchantAccount
        ListAllCustomersForAMerchant
        ListAllDeliveryAttemptsForAnEvent
        ListAllEventsAssociatedWithAMerchantAccountOrProfile
        ListAllEventsAssociatedWithAProfile
        ListAllMerchantConnectors
        ListAllPaymentMethodsForACustomer
        ListAllPaymentMethodsForAMerchant
        ListAllPayments
        ListAllRefunds
        ListAvailablePayoutFilters
        ListBlockedFingerprintsOfAParticularKind
        ListCustomerPaymentMethodsViaClientSecret
        ListDisputes
        ListMandatesForACustomer
        ListPayoutsUsingGenericConstraints
        ListProfiles
        ListRoutingConfigs
        ManuallyRetryTheDeliveryOfAnEvent
        RelayRequest
        RetrieveActiveConfig
        RetrieveACustomer
        RetrieveADispute
        RetrieveAMandate
        RetrieveAMerchantAccount
        RetrieveAMerchantConnector
        RetrieveAnAPIKey
        RetrieveAnOrganization
        RetrieveAPayment
        RetrieveAPaymentLink
        RetrieveAPaymentMethod
        RetrieveAPayout
        RetrieveAProfile
        RetrieveARefund
        RetrieveARelayDetails
        RetrieveARoutingConfig
        RetrieveDefaultConfigsForAllProfiles
        RetrieveDefaultFallbackConfig
        RetrieveGsmRule
        RetrievePollStatus
        RevokeAMandate
        RevokeAnAPIKey
        SetThePaymentMethodAsDefault
        ToggleBlocklistGuardForAParticularMerchant
        ToggleContractRoutingAlgorithm
        ToggleEliminationRoutingAlgorithm
        ToggleSuccessBasedDynamicRoutingAlgorithm
        UnblockAFingerprint
        UpdateACustomer
        UpdateAMerchantAccount
        UpdateAMerchantConnector
        UpdateAnAPIKey
        UpdateAnOrganization
        UpdateAPayment
        UpdateAPaymentMethod
        UpdateAPayout
        UpdateAProfile
        UpdateAProfileAcquirer
        UpdateARefund
        UpdateContractBasedDynamicRoutingConfigs
        UpdateDefaultConfigsForAllProfiles
        UpdateDefaultFallbackConfig
        UpdateGsmRule
        UpdateMetadataForAPayment
        UpdateSuccessBasedDynamicRoutingConfigs
    ]
}

/// Activate a routing config
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/routing/{routing_algorithm_id}/activate"
    code: 200
)
@tags([
    "Routing"
])
operation ActivateARoutingConfig {
    input: ActivateARoutingConfigInput
    output: ActivateARoutingConfig200
    errors: [
        ActivateARoutingConfig400
        ActivateARoutingConfig404
        ActivateARoutingConfig500
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/blocklist"
    code: 200
)
@tags([
    "Blocklist"
])
operation BlockAFingerprint {
    input: BlockAFingerprintInput
    output: BlockAFingerprint200
    errors: [
        BlockAFingerprint400
    ]
}

/// A Payment could can be cancelled when it is in one of these statuses: `requires_payment_method`, `requires_capture`, `requires_confirmation`, `requires_customer_action`.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/cancel"
    code: 200
)
@tags([
    "Payments"
])
operation CancelAPayment {
    input: CancelAPaymentInput
    output: Unit
    errors: [
        CancelAPayment400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/{payout_id}/cancel"
    code: 200
)
@tags([
    "Payouts"
])
operation CancelAPayout {
    input: CancelAPayoutInput
    output: CancelAPayout200
    errors: [
        CancelAPayout400
    ]
}

/// Captures the funds for a previously authorized payment intent where `capture_method` was set to `manual` and the payment is in a `requires_capture` state.
/// 
/// Upon successful capture, the payment status usually transitions to `succeeded`.
/// The `amount_to_capture` can be specified in the request body; it must be less than or equal to the payment's `amount_capturable`. If omitted, the full capturable amount is captured.
/// 
/// A payment must be in a capturable state (e.g., `requires_capture`). Attempting to capture an already `succeeded` (and fully captured) payment or one in an invalid state will lead to an error.
/// 
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/capture"
    code: 200
)
@tags([
    "Payments"
])
operation CaptureAPayment {
    input: CaptureAPaymentInput
    output: CaptureAPayment200
    errors: [
        CaptureAPayment400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/complete_authorize"
    code: 200
)
@tags([
    "Payments"
])
operation CompleteAuthorizeAPayment {
    input: CompleteAuthorizeAPaymentInput
    output: CompleteAuthorizeAPayment200
    errors: [
        CompleteAuthorizeAPayment400
    ]
}

/// Confirms a payment intent that was previously created with `confirm: false`. This action attempts to authorize the payment with the payment processor.
/// 
/// Expected status transitions after confirmation:
/// - `succeeded`: If authorization is successful and `capture_method` is `automatic`.
/// - `requires_capture`: If authorization is successful and `capture_method` is `manual`.
/// - `failed`: If authorization fails.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/confirm"
    code: 200
)
@tags([
    "Payments"
])
operation ConfirmAPayment {
    input: ConfirmAPaymentInput
    output: ConfirmAPayment200
    errors: [
        ConfirmAPayment400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/{payout_id}/confirm"
    code: 200
)
@tags([
    "Payouts"
])
operation ConfirmAPayout {
    input: ConfirmAPayoutInput
    output: ConfirmAPayout200
    errors: [
        ConfirmAPayout400
    ]
}

/// Creates a customer object and stores the customer details to be reused for future payments.
/// Incase the customer already exists in the system, this API will respond with the customer details.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/customers"
    code: 200
)
@tags([
    "Customers"
])
operation CreateACustomer {
    input: CreateACustomerInput
    output: CreateACustomer200
    errors: [
        CreateACustomer400
    ]
}

/// Create a new account for a *merchant* and the *merchant* could be a seller or retailer or client who likes to receive and send payments.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/accounts"
    code: 200
)
@tags([
    "Merchant Account"
])
operation CreateAMerchantAccount {
    input: CreateAMerchantAccountInput
    output: CreateAMerchantAccount200
    errors: [
        CreateAMerchantAccount400
    ]
}

/// Creates a new Merchant Connector for the merchant account. The connector could be a payment processor/facilitator/acquirer or a provider of specialized services like Fraud/Accounting etc.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/connectors"
    code: 200
)
@tags([
    "Merchant Connector Account"
])
operation CreateAMerchantConnector {
    input: CreateAMerchantConnectorInput
    output: CreateAMerchantConnector200
    errors: [
        CreateAMerchantConnector400
    ]
}

/// Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
/// displayed only once on creation, so ensure you store it securely.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/api_keys/{merchant_id}"
    code: 200
)
@tags([
    "API Key"
])
operation CreateAnAPIKey {
    input: CreateAnAPIKeyInput
    output: CreateAnAPIKey200
    errors: [
        CreateAnAPIKey400
    ]
}

/// Create a new authentication for accessing our APIs from your servers.
/// 
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/authentication"
    code: 200
)
@tags([
    "Authentication"
])
operation CreateAnAuthentication {
    input: CreateAnAuthenticationInput
    output: CreateAnAuthentication200
    errors: [
        CreateAnAuthentication400
    ]
}

/// Create a new organization
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/organization"
    code: 200
)
@tags([
    "Organization"
])
operation CreateAnOrganization {
    input: CreateAnOrganizationInput
    output: CreateAnOrganization200
    errors: [
        CreateAnOrganization400
    ]
}

/// Creates a payment resource, which represents a customer's intent to pay.
/// This endpoint is the starting point for various payment flows:
/// 
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments"
    code: 200
)
@tags([
    "Payments"
])
operation CreateAPayment {
    input: CreateAPaymentInput
    output: CreateAPayment200
    errors: [
        CreateAPayment400
    ]
}

/// Creates and stores a payment method against a customer.
/// In case of cards, this API should be used only by PCI compliant merchants.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payment_methods"
    code: 200
)
@tags([
    "Payment Methods"
])
operation CreateAPaymentMethod {
    input: CreateAPaymentMethodInput
    output: CreateAPaymentMethod200
    errors: [
        CreateAPaymentMethod400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/create"
    code: 200
)
@tags([
    "Payouts"
])
operation CreateAPayout {
    input: CreateAPayoutInput
    output: CreateAPayout200
    errors: [
        CreateAPayout400
    ]
}

/// Creates a new *profile* for a merchant
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/business_profile"
    code: 200
)
@tags([
    "Profile"
])
operation CreateAProfile {
    input: CreateAProfileInput
    output: CreateAProfile200
    errors: [
        CreateAProfile400
    ]
}

/// Create a new Profile Acquirer for accessing our APIs from your servers.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/profile_acquirers"
    code: 200
)
@tags([
    "Profile Acquirer"
])
operation CreateAProfileAcquirer {
    input: CreateAProfileAcquirerInput
    output: CreateAProfileAcquirer200
    errors: [
        CreateAProfileAcquirer400
    ]
}

/// Creates a refund against an already processed payment. In case of some processors, you can even opt to refund only a partial amount multiple times until the original charge amount has been refunded
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/refunds"
    code: 200
)
@tags([
    "Refunds"
])
operation CreateARefund {
    input: CreateARefundInput
    output: CreateARefund200
    errors: [
        CreateARefund400
    ]
}

/// Create a routing config
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/routing"
    code: 200
)
@tags([
    "Routing"
])
operation CreateARoutingConfig {
    input: CreateARoutingConfigInput
    output: CreateARoutingConfig200
    errors: [
        CreateARoutingConfig400
        CreateARoutingConfig403
        CreateARoutingConfig404
        CreateARoutingConfig422
        CreateARoutingConfig500
    ]
}

/// Creates a GSM (Global Status Mapping) Rule. A GSM rule is used to map a connector's error message/error code combination during a particular payments flow/sub-flow to Hyperswitch's unified status/error code/error message combination. It is also used to decide the next action in the flow - retry/requeue/do_default
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/gsm"
    code: 200
)
@tags([
    "Gsm"
])
operation CreateGsmRule {
    input: CreateGsmRuleInput
    output: CreateGsmRule200
    errors: [
        CreateGsmRule400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/post_session_tokens"
    code: 200
)
@tags([
    "Payments"
])
operation CreatePostSessionTokensForAPayment {
    input: CreatePostSessionTokensForAPaymentInput
    output: CreatePostSessionTokensForAPayment200
    errors: [
        CreatePostSessionTokensForAPayment400
    ]
}

/// Creates a session object or a session token for wallets like Apple Pay, Google Pay, etc. These tokens are used by Hyperswitch's SDK to initiate these wallets' SDK.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/session_tokens"
    code: 200
)
@tags([
    "Payments"
])
operation CreateSessionTokensForAPayment {
    input: CreateSessionTokensForAPaymentInput
    output: CreateSessionTokensForAPayment200
    errors: [
        CreateSessionTokensForAPayment400
    ]
}

/// Deactivates a routing config
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/routing/deactivate"
    code: 200
)
@tags([
    "Routing"
])
operation DeactivateARoutingConfig {
    input: DeactivateARoutingConfigInput
    output: DeactivateARoutingConfig200
    errors: [
        DeactivateARoutingConfig400
        DeactivateARoutingConfig403
        DeactivateARoutingConfig422
        DeactivateARoutingConfig500
    ]
}

/// Delete a customer record.
@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/customers/{customer_id}"
    code: 200
)
@tags([
    "Customers"
])
operation DeleteACustomer {
    input: DeleteACustomerInput
    output: DeleteACustomer200
    errors: [
        DeleteACustomer404
    ]
}

/// Delete a *merchant* account
@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/accounts/{account_id}"
    code: 200
)
@tags([
    "Merchant Account"
])
operation DeleteAMerchantAccount {
    input: DeleteAMerchantAccountInput
    output: DeleteAMerchantAccount200
    errors: [
        DeleteAMerchantAccount404
    ]
}

/// Delete or Detach a Merchant Connector from Merchant Account
@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/account/{account_id}/connectors/{merchant_connector_id}"
    code: 200
)
@tags([
    "Merchant Connector Account"
])
operation DeleteAMerchantConnector {
    input: DeleteAMerchantConnectorInput
    output: DeleteAMerchantConnector200
    errors: [
        DeleteAMerchantConnector401
        DeleteAMerchantConnector404
    ]
}

/// Deletes a payment method of a customer.
@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/payment_methods/{method_id}"
    code: 200
)
@tags([
    "Payment Methods"
])
operation DeleteAPaymentMethod {
    input: DeleteAPaymentMethodInput
    output: DeleteAPaymentMethod200
    errors: [
        DeleteAPaymentMethod404
    ]
}

/// Deletes a Gsm Rule
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/gsm/delete"
    code: 200
)
@tags([
    "Gsm"
])
operation DeleteGsmRule {
    input: DeleteGsmRuleInput
    output: DeleteGsmRule200
    errors: [
        DeleteGsmRule400
    ]
}

/// Delete the *profile*
@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/account/{account_id}/business_profile/{profile_id}"
    code: 200
)
@tags([
    "Profile"
])
operation DeleteTheProfile {
    input: DeleteTheProfileInput
    output: DeleteTheProfile200
    errors: [
        DeleteTheProfile400
    ]
}

/// Toggle KV mode for the Merchant Account
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/accounts/{account_id}/kv"
    code: 200
)
@tags([
    "Merchant Account"
])
operation EnableDisableKVForAMerchantAccount {
    input: EnableDisableKVForAMerchantAccountInput
    output: EnableDisableKVForAMerchantAccount200
    errors: [
        EnableDisableKVForAMerchantAccount400
        EnableDisableKVForAMerchantAccount404
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/three_ds_decision/execute"
    code: 200
)
@tags([
    "3DS Decision Rule"
])
operation Execute3DSDecisionRule {
    input: Execute3DSDecisionRuleInput
    output: Execute3DSDecisionRule200
    errors: [
        Execute3DSDecisionRule400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/list"
    code: 200
)
@tags([
    "Payouts"
])
operation FilterPayoutsUsingSpecificConstraints {
    input: FilterPayoutsUsingSpecificConstraintsInput
    output: FilterPayoutsUsingSpecificConstraints200
    errors: [
        FilterPayoutsUsingSpecificConstraints404
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/{payout_id}/fulfill"
    code: 200
)
@tags([
    "Payouts"
])
operation FulfillAPayout {
    input: FulfillAPayoutInput
    output: FulfillAPayout200
    errors: [
        FulfillAPayout400
    ]
}

/// Authorized amount for a payment can be incremented if it is in status: requires_capture
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/incremental_authorization"
    code: 200
)
@tags([
    "Payments"
])
operation IncrementAuthorizedAmountForAPayment {
    input: IncrementAuthorizedAmountForAPaymentInput
    output: IncrementAuthorizedAmountForAPayment200
    errors: [
        IncrementAuthorizedAmountForAPayment400
    ]
}

/// External 3DS Authentication is performed and returns the AuthenticationResponse
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/3ds/authentication"
    code: 200
)
@tags([
    "Payments"
])
operation InitiateExternalAuthenticationForAPayment {
    input: InitiateExternalAuthenticationForAPaymentInput
    output: InitiateExternalAuthenticationForAPayment200
    errors: [
        InitiateExternalAuthenticationForAPayment400
    ]
}

/// List all the API Keys associated to a merchant account.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/api_keys/{merchant_id}/list"
    code: 200
)
@tags([
    "API Key"
])
operation ListAllAPIKeysAssociatedWithAMerchantAccount {
    input: ListAllAPIKeysAssociatedWithAMerchantAccountInput
    output: ListAllAPIKeysAssociatedWithAMerchantAccount200
}

/// Lists all the customers for a particular merchant id.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/customers/list"
    code: 200
)
@tags([
    "Customers"
])
operation ListAllCustomersForAMerchant {
    input: ListAllCustomersForAMerchantInput
    output: ListAllCustomersForAMerchant200
    errors: [
        ListAllCustomersForAMerchant400
    ]
}

/// List all delivery attempts for the specified Event.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/events/{merchant_id}/{event_id}/attempts"
    code: 200
)
@tags([
    "Event"
])
operation ListAllDeliveryAttemptsForAnEvent {
    input: ListAllDeliveryAttemptsForAnEventInput
    output: ListAllDeliveryAttemptsForAnEvent200
}

/// List all Events associated with a Merchant Account or Profile.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/events/{merchant_id}"
    code: 200
)
@tags([
    "Event"
])
operation ListAllEventsAssociatedWithAMerchantAccountOrProfile {
    input: ListAllEventsAssociatedWithAMerchantAccountOrProfileInput
    output: ListAllEventsAssociatedWithAMerchantAccountOrProfile200
}

/// List all Events associated with a Profile.
@auth([
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/events/profile/list"
    code: 200
)
@tags([
    "Event"
])
operation ListAllEventsAssociatedWithAProfile {
    input: ListAllEventsAssociatedWithAProfileInput
    output: ListAllEventsAssociatedWithAProfile200
}

/// List Merchant Connector Details for the merchant
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/account/{account_id}/connectors"
    code: 200
)
@tags([
    "Merchant Connector Account"
])
operation ListAllMerchantConnectors {
    input: ListAllMerchantConnectorsInput
    output: ListAllMerchantConnectors200
    errors: [
        ListAllMerchantConnectors401
        ListAllMerchantConnectors404
    ]
}

/// Lists all the applicable payment methods for a particular Customer ID.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/customers/{customer_id}/payment_methods"
    code: 200
)
@tags([
    "Payment Methods"
])
operation ListAllPaymentMethodsForACustomer {
    input: ListAllPaymentMethodsForACustomerInput
    output: ListAllPaymentMethodsForACustomer200
    errors: [
        ListAllPaymentMethodsForACustomer400
        ListAllPaymentMethodsForACustomer404
    ]
}

/// Lists the applicable payment methods for a particular Merchant ID.
/// Use the client secret and publishable key authorization to list all relevant payment methods of the merchant for the payment corresponding to the client secret.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/account/payment_methods"
    code: 200
)
@tags([
    "Payment Methods"
])
operation ListAllPaymentMethodsForAMerchant {
    input: ListAllPaymentMethodsForAMerchantInput
    output: ListAllPaymentMethodsForAMerchant200
    errors: [
        ListAllPaymentMethodsForAMerchant400
        ListAllPaymentMethodsForAMerchant404
    ]
}

/// To list the *payments*
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/payments/list"
    code: 200
)
@tags([
    "Payments"
])
operation ListAllPayments {
    input: ListAllPaymentsInput
    output: ListAllPayments200
    errors: [
        ListAllPayments404
    ]
}

/// Lists all the refunds associated with the merchant, or for a specific payment if payment_id is provided
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/refunds/list"
    code: 200
)
@tags([
    "Refunds"
])
operation ListAllRefunds {
    input: ListAllRefundsInput
    output: ListAllRefunds200
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/filter"
    code: 200
)
@tags([
    "Payouts"
])
operation ListAvailablePayoutFilters {
    input: ListAvailablePayoutFiltersInput
    output: ListAvailablePayoutFilters200
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/blocklist"
    code: 200
)
@tags([
    "Blocklist"
])
operation ListBlockedFingerprintsOfAParticularKind {
    input: ListBlockedFingerprintsOfAParticularKindInput
    output: ListBlockedFingerprintsOfAParticularKind200
    errors: [
        ListBlockedFingerprintsOfAParticularKind400
    ]
}

/// Lists all the applicable payment methods for a particular payment tied to the `client_secret`.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/customers/payment_methods"
    code: 200
)
@tags([
    "Payment Methods"
])
operation ListCustomerPaymentMethodsViaClientSecret {
    input: ListCustomerPaymentMethodsViaClientSecretInput
    output: ListCustomerPaymentMethodsViaClientSecret200
    errors: [
        ListCustomerPaymentMethodsViaClientSecret400
        ListCustomerPaymentMethodsViaClientSecret404
    ]
}

/// Lists all the Disputes for a merchant
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/disputes/list"
    code: 200
)
@tags([
    "Disputes"
])
operation ListDisputes {
    input: ListDisputesInput
    output: ListDisputes200
    errors: [
        ListDisputes401
    ]
}

/// Lists all the mandates for a particular customer id.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/customers/{customer_id}/mandates"
    code: 200
)
@tags([
    "Mandates"
])
operation ListMandatesForACustomer {
    input: ListMandatesForACustomerInput
    output: ListMandatesForACustomer200
    errors: [
        ListMandatesForACustomer400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/payouts/list"
    code: 200
)
@tags([
    "Payouts"
])
operation ListPayoutsUsingGenericConstraints {
    input: ListPayoutsUsingGenericConstraintsInput
    output: ListPayoutsUsingGenericConstraints200
    errors: [
        ListPayoutsUsingGenericConstraints404
    ]
}

/// Lists all the *profiles* under a merchant
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/account/{account_id}/business_profile"
    code: 200
)
@tags([
    "Profile"
])
operation ListProfiles {
    input: ListProfilesInput
    output: ListProfiles200
}

/// List all routing configs
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "GET"
    uri: "/routing"
    code: 200
)
@tags([
    "Routing"
])
operation ListRoutingConfigs {
    input: ListRoutingConfigsInput
    output: ListRoutingConfigs200
    errors: [
        ListRoutingConfigs404
        ListRoutingConfigs500
    ]
}

/// Manually retry the delivery of the specified Event.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/events/{merchant_id}/{event_id}/retry"
    code: 200
)
@tags([
    "Event"
])
operation ManuallyRetryTheDeliveryOfAnEvent {
    input: ManuallyRetryTheDeliveryOfAnEventInput
    output: ManuallyRetryTheDeliveryOfAnEvent200
}

/// Creates a relay request.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/relay"
    code: 200
)
@tags([
    "Relay"
])
operation RelayRequest {
    input: RelayRequestInput
    output: RelayRequest200
    errors: [
        RelayRequest400
    ]
}

/// Retrieve active config
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "GET"
    uri: "/routing/active"
    code: 200
)
@tags([
    "Routing"
])
operation RetrieveActiveConfig {
    input: RetrieveActiveConfigInput
    output: RetrieveActiveConfig200
    errors: [
        RetrieveActiveConfig403
        RetrieveActiveConfig404
        RetrieveActiveConfig500
    ]
}

/// Retrieves a customer's details.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/customers/{customer_id}"
    code: 200
)
@tags([
    "Customers"
])
operation RetrieveACustomer {
    input: RetrieveACustomerInput
    output: RetrieveACustomer200
    errors: [
        RetrieveACustomer404
    ]
}

/// Retrieves a dispute
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/disputes/{dispute_id}"
    code: 200
)
@tags([
    "Disputes"
])
operation RetrieveADispute {
    input: RetrieveADisputeInput
    output: RetrieveADispute200
    errors: [
        RetrieveADispute404
    ]
}

/// Retrieves a mandate created using the Payments/Create API
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/mandates/{mandate_id}"
    code: 200
)
@tags([
    "Mandates"
])
operation RetrieveAMandate {
    input: RetrieveAMandateInput
    output: RetrieveAMandate200
    errors: [
        RetrieveAMandate404
    ]
}

/// Retrieve a *merchant* account details.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/accounts/{account_id}"
    code: 200
)
@tags([
    "Merchant Account"
])
operation RetrieveAMerchantAccount {
    input: RetrieveAMerchantAccountInput
    output: RetrieveAMerchantAccount200
    errors: [
        RetrieveAMerchantAccount404
    ]
}

/// Retrieves details of a Connector account
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/account/{account_id}/connectors/{merchant_connector_id}"
    code: 200
)
@tags([
    "Merchant Connector Account"
])
operation RetrieveAMerchantConnector {
    input: RetrieveAMerchantConnectorInput
    output: RetrieveAMerchantConnector200
    errors: [
        RetrieveAMerchantConnector401
        RetrieveAMerchantConnector404
    ]
}

/// Retrieve information about the specified API Key.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/api_keys/{merchant_id}/{key_id}"
    code: 200
)
@tags([
    "API Key"
])
operation RetrieveAnAPIKey {
    input: RetrieveAnAPIKeyInput
    output: RetrieveAnAPIKey200
    errors: [
        RetrieveAnAPIKey404
    ]
}

/// Retrieve an existing organization
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/organization/{id}"
    code: 200
)
@tags([
    "Organization"
])
operation RetrieveAnOrganization {
    input: RetrieveAnOrganizationInput
    output: RetrieveAnOrganization200
    errors: [
        RetrieveAnOrganization400
    ]
}

/// Retrieves a Payment. This API can also be used to get the status of a previously initiated payment or next action for an ongoing payment
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/payments/{payment_id}"
    code: 200
)
@tags([
    "Payments"
])
operation RetrieveAPayment {
    input: RetrieveAPaymentInput
    output: RetrieveAPayment200
    errors: [
        RetrieveAPayment404
    ]
}

/// To retrieve the properties of a Payment Link. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/payment_link/{payment_link_id}"
    code: 200
)
@tags([
    "Payments"
])
operation RetrieveAPaymentLink {
    input: RetrieveAPaymentLinkInput
    output: RetrieveAPaymentLink200
    errors: [
        RetrieveAPaymentLink404
    ]
}

/// Retrieves a payment method of a customer.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/payment_methods/{method_id}"
    code: 200
)
@tags([
    "Payment Methods"
])
operation RetrieveAPaymentMethod {
    input: RetrieveAPaymentMethodInput
    output: RetrieveAPaymentMethod200
    errors: [
        RetrieveAPaymentMethod404
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/payouts/{payout_id}"
    code: 200
)
@tags([
    "Payouts"
])
operation RetrieveAPayout {
    input: RetrieveAPayoutInput
    output: RetrieveAPayout200
    errors: [
        RetrieveAPayout404
    ]
}

/// Retrieve existing *profile*
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/account/{account_id}/business_profile/{profile_id}"
    code: 200
)
@tags([
    "Profile"
])
operation RetrieveAProfile {
    input: RetrieveAProfileInput
    output: RetrieveAProfile200
    errors: [
        RetrieveAProfile400
    ]
}

/// Retrieves a Refund. This may be used to get the status of a previously initiated refund
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/refunds/{refund_id}"
    code: 200
)
@tags([
    "Refunds"
])
operation RetrieveARefund {
    input: RetrieveARefundInput
    output: RetrieveARefund200
    errors: [
        RetrieveARefund404
    ]
}

/// Retrieves a relay details.
@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/relay/{relay_id}"
    code: 200
)
@tags([
    "Relay"
])
operation RetrieveARelayDetails {
    input: RetrieveARelayDetailsInput
    output: RetrieveARelayDetails200
    errors: [
        RetrieveARelayDetails404
    ]
}

/// Retrieve a routing algorithm
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "GET"
    uri: "/routing/{routing_algorithm_id}"
    code: 200
)
@tags([
    "Routing"
])
operation RetrieveARoutingConfig {
    input: RetrieveARoutingConfigInput
    output: RetrieveARoutingConfig200
    errors: [
        RetrieveARoutingConfig403
        RetrieveARoutingConfig404
        RetrieveARoutingConfig500
    ]
}

/// Retrieve default config for profiles
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "GET"
    uri: "/routing/default/profile"
    code: 200
)
@tags([
    "Routing"
])
operation RetrieveDefaultConfigsForAllProfiles {
    input: Unit
    output: RetrieveDefaultConfigsForAllProfiles200
    errors: [
        RetrieveDefaultConfigsForAllProfiles404
        RetrieveDefaultConfigsForAllProfiles500
    ]
}

/// Retrieve default fallback config
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "GET"
    uri: "/routing/default"
    code: 200
)
@tags([
    "Routing"
])
operation RetrieveDefaultFallbackConfig {
    input: Unit
    output: RetrieveDefaultFallbackConfig200
    errors: [
        RetrieveDefaultFallbackConfig500
    ]
}

/// Retrieves a Gsm Rule
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/gsm/get"
    code: 200
)
@tags([
    "Gsm"
])
operation RetrieveGsmRule {
    input: RetrieveGsmRuleInput
    output: RetrieveGsmRule200
    errors: [
        RetrieveGsmRule400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "GET"
    uri: "/poll/status/{poll_id}"
    code: 200
)
@tags([
    "Poll"
])
operation RetrievePollStatus {
    input: RetrievePollStatusInput
    output: RetrievePollStatus200
    errors: [
        RetrievePollStatus404
    ]
}

/// Revokes a mandate created using the Payments/Create API
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/mandates/revoke/{mandate_id}"
    code: 200
)
@tags([
    "Mandates"
])
operation RevokeAMandate {
    input: RevokeAMandateInput
    output: RevokeAMandate200
    errors: [
        RevokeAMandate400
    ]
}

/// Revoke the specified API Key. Once revoked, the API Key can no longer be used for
/// authenticating with our APIs.
@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/api_keys/{merchant_id}/{key_id}"
    code: 200
)
@tags([
    "API Key"
])
operation RevokeAnAPIKey {
    input: RevokeAnAPIKeyInput
    output: RevokeAnAPIKey200
    errors: [
        RevokeAnAPIKey404
    ]
}

/// Set the Payment Method as Default for the Customer.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/{customer_id}/payment_methods/{payment_method_id}/default"
    code: 200
)
@tags([
    "Payment Methods"
])
operation SetThePaymentMethodAsDefault {
    input: SetThePaymentMethodAsDefaultInput
    output: SetThePaymentMethodAsDefault200
    errors: [
        SetThePaymentMethodAsDefault400
        SetThePaymentMethodAsDefault404
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/blocklist/toggle"
    code: 200
)
@tags([
    "Blocklist"
])
operation ToggleBlocklistGuardForAParticularMerchant {
    input: ToggleBlocklistGuardForAParticularMerchantInput
    output: ToggleBlocklistGuardForAParticularMerchant200
    errors: [
        ToggleBlocklistGuardForAParticularMerchant400
    ]
}

/// Create a Contract based dynamic routing algorithm
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/contracts/toggle"
    code: 200
)
@tags([
    "Routing"
])
operation ToggleContractRoutingAlgorithm {
    input: ToggleContractRoutingAlgorithmInput
    output: ToggleContractRoutingAlgorithm200
    errors: [
        ToggleContractRoutingAlgorithm400
        ToggleContractRoutingAlgorithm403
        ToggleContractRoutingAlgorithm404
        ToggleContractRoutingAlgorithm422
        ToggleContractRoutingAlgorithm500
    ]
}

/// Create a elimination based dynamic routing algorithm
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/elimination/toggle"
    code: 200
)
@tags([
    "Routing"
])
operation ToggleEliminationRoutingAlgorithm {
    input: ToggleEliminationRoutingAlgorithmInput
    output: ToggleEliminationRoutingAlgorithm200
    errors: [
        ToggleEliminationRoutingAlgorithm400
        ToggleEliminationRoutingAlgorithm403
        ToggleEliminationRoutingAlgorithm404
        ToggleEliminationRoutingAlgorithm422
        ToggleEliminationRoutingAlgorithm500
    ]
}

/// Create a success based dynamic routing algorithm
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/success_based/toggle"
    code: 200
)
@tags([
    "Routing"
])
operation ToggleSuccessBasedDynamicRoutingAlgorithm {
    input: ToggleSuccessBasedDynamicRoutingAlgorithmInput
    output: ToggleSuccessBasedDynamicRoutingAlgorithm200
    errors: [
        ToggleSuccessBasedDynamicRoutingAlgorithm400
        ToggleSuccessBasedDynamicRoutingAlgorithm403
        ToggleSuccessBasedDynamicRoutingAlgorithm404
        ToggleSuccessBasedDynamicRoutingAlgorithm422
        ToggleSuccessBasedDynamicRoutingAlgorithm500
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "DELETE"
    uri: "/blocklist"
    code: 200
)
@tags([
    "Blocklist"
])
operation UnblockAFingerprint {
    input: UnblockAFingerprintInput
    output: UnblockAFingerprint200
    errors: [
        UnblockAFingerprint400
    ]
}

/// Updates the customer's details in a customer object.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/customers/{customer_id}"
    code: 200
)
@tags([
    "Customers"
])
operation UpdateACustomer {
    input: UpdateACustomerInput
    output: UpdateACustomer200
    errors: [
        UpdateACustomer404
    ]
}

/// Updates details of an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/accounts/{account_id}"
    code: 200
)
@tags([
    "Merchant Account"
])
operation UpdateAMerchantAccount {
    input: UpdateAMerchantAccountInput
    output: UpdateAMerchantAccount200
    errors: [
        UpdateAMerchantAccount404
    ]
}

/// To update an existing Merchant Connector account. Helpful in enabling/disabling different payment methods and other settings for the connector
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/connectors/{merchant_connector_id}"
    code: 200
)
@tags([
    "Merchant Connector Account"
])
operation UpdateAMerchantConnector {
    input: UpdateAMerchantConnectorInput
    output: UpdateAMerchantConnector200
    errors: [
        UpdateAMerchantConnector401
        UpdateAMerchantConnector404
    ]
}

/// Update information for the specified API Key.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/api_keys/{merchant_id}/{key_id}"
    code: 200
)
@tags([
    "API Key"
])
operation UpdateAnAPIKey {
    input: UpdateAnAPIKeyInput
    output: UpdateAnAPIKey200
    errors: [
        UpdateAnAPIKey404
    ]
}

/// Create a new organization for .
@auth([
    httpApiKeyAuth
])
@http(
    method: "PUT"
    uri: "/organization/{id}"
    code: 200
)
@tags([
    "Organization"
])
operation UpdateAnOrganization {
    input: UpdateAnOrganizationInput
    output: UpdateAnOrganization200
    errors: [
        UpdateAnOrganization400
    ]
}

/// To update the properties of a *PaymentIntent* object. This may include attaching a payment method, or attaching customer object or metadata fields after the Payment is created
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}"
    code: 200
)
@tags([
    "Payments"
])
operation UpdateAPayment {
    input: UpdateAPaymentInput
    output: UpdateAPayment200
    errors: [
        UpdateAPayment400
    ]
}

/// Update an existing payment method of a customer.
/// This API is useful for use cases such as updating the card number for expired cards to prevent discontinuity in recurring payments.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payment_methods/{method_id}/update"
    code: 200
)
@tags([
    "Payment Methods"
])
operation UpdateAPaymentMethod {
    input: UpdateAPaymentMethodInput
    output: UpdateAPaymentMethod200
    errors: [
        UpdateAPaymentMethod404
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payouts/{payout_id}"
    code: 200
)
@tags([
    "Payouts"
])
operation UpdateAPayout {
    input: UpdateAPayoutInput
    output: UpdateAPayout200
    errors: [
        UpdateAPayout400
    ]
}

/// Update the *profile*
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/account/{account_id}/business_profile/{profile_id}"
    code: 200
)
@tags([
    "Profile"
])
operation UpdateAProfile {
    input: UpdateAProfileInput
    output: UpdateAProfile200
    errors: [
        UpdateAProfile400
    ]
}

/// Update a Profile Acquirer for accessing our APIs from your servers.
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/profile_acquirers/{profile_id}/{profile_acquirer_id}"
    code: 200
)
@tags([
    "Profile Acquirer"
])
operation UpdateAProfileAcquirer {
    input: UpdateAProfileAcquirerInput
    output: UpdateAProfileAcquirer200
    errors: [
        UpdateAProfileAcquirer400
    ]
}

/// Updates the properties of a Refund object. This API can be used to attach a reason for the refund or metadata fields
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/refunds/{refund_id}"
    code: 200
)
@tags([
    "Refunds"
])
operation UpdateARefund {
    input: UpdateARefundInput
    output: UpdateARefund200
    errors: [
        UpdateARefund400
    ]
}

/// Update contract based dynamic routing algorithm
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "PATCH"
    uri: "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/contracts/config/{algorithm_id}"
    code: 200
)
@tags([
    "Routing"
])
operation UpdateContractBasedDynamicRoutingConfigs {
    input: UpdateContractBasedDynamicRoutingConfigsInput
    output: UpdateContractBasedDynamicRoutingConfigs200
    errors: [
        UpdateContractBasedDynamicRoutingConfigs400
        UpdateContractBasedDynamicRoutingConfigs403
        UpdateContractBasedDynamicRoutingConfigs404
        UpdateContractBasedDynamicRoutingConfigs422
        UpdateContractBasedDynamicRoutingConfigs500
    ]
}

/// Update default config for profiles
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/routing/default/profile/{profile_id}"
    code: 200
)
@tags([
    "Routing"
])
operation UpdateDefaultConfigsForAllProfiles {
    input: UpdateDefaultConfigsForAllProfilesInput
    output: UpdateDefaultConfigsForAllProfiles200
    errors: [
        UpdateDefaultConfigsForAllProfiles400
        UpdateDefaultConfigsForAllProfiles403
        UpdateDefaultConfigsForAllProfiles404
        UpdateDefaultConfigsForAllProfiles422
        UpdateDefaultConfigsForAllProfiles500
    ]
}

/// Update default fallback config
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "POST"
    uri: "/routing/default"
    code: 200
)
@tags([
    "Routing"
])
operation UpdateDefaultFallbackConfig {
    input: UpdateDefaultFallbackConfigInput
    output: UpdateDefaultFallbackConfig200
    errors: [
        UpdateDefaultFallbackConfig400
        UpdateDefaultFallbackConfig422
        UpdateDefaultFallbackConfig500
    ]
}

/// Updates a Gsm Rule
@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/gsm/update"
    code: 200
)
@tags([
    "Gsm"
])
operation UpdateGsmRule {
    input: UpdateGsmRuleInput
    output: UpdateGsmRule200
    errors: [
        UpdateGsmRule400
    ]
}

@auth([
    httpApiKeyAuth
])
@http(
    method: "POST"
    uri: "/payments/{payment_id}/update_metadata"
    code: 200
)
@tags([
    "Payments"
])
operation UpdateMetadataForAPayment {
    input: UpdateMetadataForAPaymentInput
    output: UpdateMetadataForAPayment200
    errors: [
        UpdateMetadataForAPayment400
    ]
}

/// Update success based dynamic routing algorithm
@auth([
    httpApiKeyAuth
    httpBearerAuth
])
@http(
    method: "PATCH"
    uri: "/account/{account_id}/business_profile/{profile_id}/dynamic_routing/success_based/config/{algorithm_id}"
    code: 200
)
@tags([
    "Routing"
])
operation UpdateSuccessBasedDynamicRoutingConfigs {
    input: UpdateSuccessBasedDynamicRoutingConfigsInput
    output: UpdateSuccessBasedDynamicRoutingConfigs200
    errors: [
        UpdateSuccessBasedDynamicRoutingConfigs400
        UpdateSuccessBasedDynamicRoutingConfigs403
        UpdateSuccessBasedDynamicRoutingConfigs404
        UpdateSuccessBasedDynamicRoutingConfigs422
        UpdateSuccessBasedDynamicRoutingConfigs500
    ]
}

structure AcceptedCountriesOneOfAlt0 {
    @required
    list: AcceptedCountriesOneOfAlt0List
}

structure AcceptedCountriesOneOfAlt1 {
    @required
    list: AcceptedCountriesOneOfAlt1List
}

structure AcceptedCurrenciesOneOfAlt0 {
    @required
    list: AcceptedCurrenciesOneOfAlt0List
}

structure AcceptedCurrenciesOneOfAlt1 {
    @required
    list: AcceptedCurrenciesOneOfAlt1List
}

/// Payment Method data for Ach bank debit
structure AchBankDebit {
    billing_details: BankDebitDataOneOfAlt0AchBankDebitBillingDetails
    /// Account number for ach bank debit payment
    @dataExamples([
        {
            json: "000123456789"
        }
    ])
    @required
    account_number: String
    /// Routing number for ach bank debit payment
    @dataExamples([
        {
            json: "110000000"
        }
    ])
    @required
    routing_number: String
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @required
    bank_account_holder_name: String
    @dataExamples([
        {
            json: "ACH"
        }
    ])
    @required
    bank_name: String
    @dataExamples([
        {
            json: "Checking"
        }
    ])
    @required
    bank_type: String
    @dataExamples([
        {
            json: "Personal"
        }
    ])
    @required
    bank_holder_type: String
}

structure AchBankDebitAdditionalData {
    /// Partially masked account number for ach bank debit payment
    @dataExamples([
        {
            json: "0001****3456"
        }
    ])
    @required
    account_number: String
    /// Partially masked routing number for ach bank debit payment
    @dataExamples([
        {
            json: "110***000"
        }
    ])
    @required
    routing_number: String
    /// Card holder's name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    card_holder_name: String
    /// Bank account's owner name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    bank_account_holder_name: String
    bank_name: AchBankDebitAdditionalDataBankName
    bank_type: BankType
    bank_holder_type: BankHolderType
}

structure AchBankDebitAdditionalDataBankName {}

structure AchBankTransfer {
    billing_details: BankTransferDataOneOfAlt0AchBankTransferBillingDetails
}

/// Masked payout method details for ach bank transfer payout method
structure AchBankTransferAdditionalData {
    /// Partially masked account number for ach bank debit payment
    @dataExamples([
        {
            json: "0001****3456"
        }
    ])
    @required
    bank_account_number: String
    /// Partially masked routing number for ach bank debit payment
    @dataExamples([
        {
            json: "110***000"
        }
    ])
    @required
    bank_routing_number: String
    bank_name: AchBankTransferAdditionalDataBankName
    bank_country_code: AchBankTransferAdditionalDataBankCountryCode
    /// Bank city
    @dataExamples([
        {
            json: "California"
        }
    ])
    bank_city: String
}

structure AchBankTransferAdditionalDataBankCountryCode {}

structure AchBankTransferAdditionalDataBankName {}

structure AchBankTransferBankCountryCode {}

@mixin
structure AchBillingDetails {
    /// The Email ID for ACH billing
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
}

structure AchTransfer {
    @dataExamples([
        {
            json: "122385736258"
        }
    ])
    @required
    account_number: String
    @required
    bank_name: String
    @dataExamples([
        {
            json: "012"
        }
    ])
    @required
    routing_number: String
    @dataExamples([
        {
            json: "234"
        }
    ])
    @required
    swift_code: String
}

structure Acquirer with [AcquirerData] {}

/// Acquirer configuration
structure AcquirerConfig {
    /// The merchant id assigned by the acquirer
    @dataExamples([
        {
            json: "M123456789"
        }
    ])
    @required
    acquirer_assigned_merchant_id: String
    /// merchant name
    @dataExamples([
        {
            json: "NewAge Retailer"
        }
    ])
    @required
    merchant_name: String
    /// Merchant country code assigned by acquirer
    @dataExamples([
        {
            json: "US"
        }
    ])
    @required
    merchant_country_code: String
    /// Network provider
    @dataExamples([
        {
            json: "VISA"
        }
    ])
    @required
    network: String
    /// Acquirer bin
    @dataExamples([
        {
            json: "456789"
        }
    ])
    @required
    acquirer_bin: String
    /// Acquirer ica provided by acquirer
    @dataExamples([
        {
            json: "401288"
        }
    ])
    acquirer_ica: String
    /// Fraud rate for the particular acquirer configuration
    @dataExamples([
        {
            json: "0.01"
        }
    ])
    @required
    acquirer_fraud_rate: String
}

/// Represents data about the acquirer used in the 3DS decision rule.
@mixin
structure AcquirerData {
    @required
    country: Country
    /// The fraud rate associated with the acquirer.
    fraud_rate: Double
}

@mixin
structure AcquirerDetails {
    /// The bin of the card.
    @dataExamples([
        {
            json: "123456"
        }
    ])
    bin: String
    /// The merchant id of the card.
    @dataExamples([
        {
            json: "merchant_abc"
        }
    ])
    merchant_id: String
    /// The country code of the card.
    @dataExamples([
        {
            json: "US/34456"
        }
    ])
    country_code: String
}

structure ActivateARoutingConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure ActivateARoutingConfig400 {}

@error("client")
@httpError(404)
structure ActivateARoutingConfig404 {}

@error("server")
@httpError(500)
structure ActivateARoutingConfig500 {}

structure ActivateARoutingConfigInput {
    /// The unique identifier for a config
    @httpLabel
    @required
    routing_algorithm_id: String
}

structure AdditionalMerchantDataOneOfAlt0 {
    @required
    open_banking_recipient_data: MerchantRecipientData
}

structure AdditionalPayoutMethodDataOneOfAlt0 {
    @required
    Card: CardAdditionalData
}

structure AdditionalPayoutMethodDataOneOfAlt1 {
    @required
    Bank: BankAdditionalData
}

structure AdditionalPayoutMethodDataOneOfAlt2 {
    @required
    Wallet: WalletAdditionalData
}

structure AddressAddress with [AddressDetails] {}

/// Address details
@mixin
structure AddressDetails {
    /// The city, district, suburb, town, or village of the address.
    @dataExamples([
        {
            json: "New York"
        }
    ])
    @length(
        max: 50
    )
    city: String
    country: AddressDetailsCountry
    /// The first line of the street address or P.O. Box.
    @dataExamples([
        {
            json: "123, King Street"
        }
    ])
    @length(
        max: 200
    )
    line1: String
    /// The second line of the street address or P.O. Box (e.g., apartment, suite, unit, or building).
    @dataExamples([
        {
            json: "Powelson Avenue"
        }
    ])
    @length(
        max: 50
    )
    line2: String
    /// The third line of the street address, if applicable.
    @dataExamples([
        {
            json: "Bridgewater"
        }
    ])
    @length(
        max: 50
    )
    line3: String
    /// The zip/postal code for the address
    @dataExamples([
        {
            json: "08807"
        }
    ])
    @length(
        max: 50
    )
    zip: String
    /// The address state
    @dataExamples([
        {
            json: "New York"
        }
    ])
    state: String
    /// The first name for the address
    @dataExamples([
        {
            json: "John"
        }
    ])
    @length(
        max: 255
    )
    first_name: String
    /// The last name for the address
    @dataExamples([
        {
            json: "Doe"
        }
    ])
    @length(
        max: 255
    )
    last_name: String
}

structure AddressDetailsCountry {}

structure Adyen with [AdyenConnectorMetadata] {}

@mixin
structure AdyenConnectorMetadata {
    @required
    testing: AdyenTestingData
}

/// Fee information for Split Payments to be charged on the payment being collected for Adyen
structure AdyenSplitData {
    /// The store identifier
    store: String
    @required
    split_items: SplitItems
}

/// Data for the split items
structure AdyenSplitItem {
    /// The amount of the split item
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    @required
    split_type: AdyenSplitType
    /// The unique identifier of the account to which the split amount is allocated.
    account: String
    /// Unique Identifier for the split item
    @required
    reference: String
    /// Description for the part of the payment that will be allocated to the specified account.
    description: String
}

structure AdyenTestingData {
    /// Holder name to be sent to Adyen for a card payment(CIT) or a generic payment(MIT). This value overrides the values for card.card_holder_name and applies during both CIT and MIT payment transactions.
    @required
    holder_name: String
}

/// For AfterpayClearpay redirect as PayLater Option
structure AfterpayClearpayRedirect {
    /// The billing email
    billing_email: String
    /// The billing name
    billing_name: String
}

structure Airwallex with [AirwallexData] {}

@mixin
structure AirwallexData {
    /// payload required by airwallex
    payload: String
}

structure AlfamartVoucherData {
    /// The billing first name for Alfamart
    @dataExamples([
        {
            json: "Jane"
        }
    ])
    first_name: String
    /// The billing second name for Alfamart
    @dataExamples([
        {
            json: "Doe"
        }
    ])
    last_name: String
    /// The Email ID for Alfamart
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
}

structure AlgorithmFor {}

structure AmountInfo {
    /// The label must be the name of the merchant.
    @required
    label: String
    /// A value that indicates whether the line item(Ex: total, tax, discount, or grand total) is final or pending.
    type: String
    /// The total amount for the payment in majot unit string (Ex: 38.02)
    @dataExamples([
        {
            json: "38.02"
        }
    ])
    @required
    amount: String
}

structure ApplePay with [ApplepayConnectorMetadataRequest] {}

@mixin
structure ApplepayConnectorMetadataRequest {
    session_token_data: ApplepayConnectorMetadataRequestSessionTokenData
}

structure ApplepayConnectorMetadataRequestSessionTokenData with [SessionTokenInfo] {}

structure ApplepayPaymentMethod {
    /// The name to be displayed on Apple Pay button
    @required
    display_name: String
    /// The network of the Apple pay payment method
    @required
    network: String
    /// The type of the payment method
    @required
    type: String
}

@mixin
structure ApplePayPaymentRequest {
    @required
    country_code: CountryAlpha2
    @required
    currency_code: Currency
    @required
    total: AmountInfo
    merchant_capabilities: MerchantCapabilities
    supported_networks: SupportedNetworks
    merchant_identifier: String
    required_billing_contact_fields: RequiredBillingContactFields
    required_shipping_contact_fields: RequiredShippingContactFields
    recurring_payment_request: RecurringPaymentRequest
}

@mixin
structure ApplePayRecurringPaymentRequest {
    /// A description of the recurring payment that Apple Pay displays to the user in the payment sheet
    @required
    payment_description: String
    @required
    regular_billing: ApplePayRegularBillingRequest
    /// A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment
    billing_agreement: String
    /// A URL to a web page where the user can update or delete the payment method for the recurring payment
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    @required
    management_u_r_l: String
}

structure ApplePayRegularBillingDetails {
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    @required
    label: String
    /// The date of the first payment
    @dataExamples([
        {
            json: "2023-09-10T23:59:59Z"
        }
    ])
    @timestampFormat("date-time")
    recurring_payment_start_date: Timestamp
    /// The date of the final payment
    @dataExamples([
        {
            json: "2023-09-10T23:59:59Z"
        }
    ])
    @timestampFormat("date-time")
    recurring_payment_end_date: Timestamp
    recurring_payment_interval_unit: ApplePayRegularBillingDetailsRecurringPaymentIntervalUnit
    /// The number of interval units that make up the total payment interval
    recurring_payment_interval_count: Integer
}

structure ApplePayRegularBillingDetailsRecurringPaymentIntervalUnit {}

structure ApplePayRegularBillingRequest {
    /// The amount of the recurring payment
    @dataExamples([
        {
            json: "38.02"
        }
    ])
    @required
    amount: String
    /// The label that Apple Pay displays to the user in the payment sheet with the recurring details
    @required
    label: String
    @required
    payment_timing: ApplePayPaymentTiming
    /// The date of the first payment
    @timestampFormat("date-time")
    recurring_payment_start_date: Timestamp
    /// The date of the final payment
    @timestampFormat("date-time")
    recurring_payment_end_date: Timestamp
    recurring_payment_interval_unit: ApplePayRegularBillingRequestRecurringPaymentIntervalUnit
    /// The number of interval units that make up the total payment interval
    recurring_payment_interval_count: Integer
}

structure ApplePayRegularBillingRequestRecurringPaymentIntervalUnit {}

@mixin
structure ApplepaySessionTokenResponse {
    session_token_data: ApplepaySessionTokenResponseSessionTokenData
    payment_request_data: PaymentRequestData
    /// The session token is w.r.t this connector
    @required
    connector: String
    /// Identifier for the delayed session response
    @required
    delayed_session_token: Boolean
    @required
    sdk_next_action: SdkNextAction
    /// The connector transaction id
    connector_reference_id: String
    /// The public key id is to invoke third party sdk
    connector_sdk_public_key: String
    /// The connector merchant id
    connector_merchant_id: String
}

structure ApplepaySessionTokenResponseSessionTokenData {}

structure ApplePayWalletData {
    /// The payment data of Apple pay
    @required
    payment_data: String
    @required
    payment_method: ApplepayPaymentMethod
    /// The unique identifier for the transaction
    @required
    transaction_identifier: String
}

structure AssuranceDetails with [GooglePayAssuranceDetails] {}

@mixin
structure AuthenticationConnectorDetails {
    @required
    authentication_connectors: AuthenticationConnectors
    /// URL of the (customer service) website that will be shown to the shopper in case of technical errors during the 3D Secure 2 process.
    @required
    three_ds_requestor_url: String
    /// Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred.
    three_ds_requestor_app_url: String
}

structure AuthenticationCreateRequest {
    /// The unique identifier for this authentication.
    @dataExamples([
        {
            json: "auth_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    authentication_id: String
    /// The business profile that is associated with this authentication
    profile_id: String
    authentication_connector: AuthenticationCreateRequestAuthenticationConnector
    customer: AuthenticationCreateRequestCustomer
    @required
    amount: MinorUnit
    @required
    currency: Currency
    /// The URL to which the user should be redirected after authentication.
    @dataExamples([
        {
            json: "https://example.com/redirect"
        }
    ])
    return_url: String
    acquirer_details: AuthenticationCreateRequestAcquirerDetails
    /// Force 3DS challenge.
    force_3ds_challenge: Boolean
    psd2_sca_exemption_type: AuthenticationCreateRequestPsd2ScaExemptionType
}

structure AuthenticationCreateRequestAcquirerDetails with [AcquirerDetails] {}

structure AuthenticationCreateRequestAuthenticationConnector {}

structure AuthenticationCreateRequestPsd2ScaExemptionType {}

structure AuthenticationFlow {}

structure AuthenticationResponse {
    /// The unique identifier for this authentication.
    @dataExamples([
        {
            json: "auth_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @required
    authentication_id: String
    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    @dataExamples([
        {
            json: "merchant_abc"
        }
    ])
    @required
    merchant_id: String
    @required
    status: AuthenticationStatus
    /// The client secret for this authentication, to be used for client-side operations.
    @dataExamples([
        {
            json: "auth_mbabizu24mvu3mela5njyhpit4_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    client_secret: String
    @required
    amount: MinorUnit
    @required
    currency: Currency
    /// Whether 3DS challenge was forced.
    force_3ds_challenge: Boolean
    authentication_connector: AuthenticationResponseAuthenticationConnector
    /// The URL to which the user should be redirected after authentication, if provided.
    return_url: String
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    created_at: Timestamp
    @dataExamples([
        {
            json: "E0001"
        }
    ])
    error_code: String
    /// If there was an error while calling the connector the error message is received here
    @dataExamples([
        {
            json: "Failed while verifying the card"
        }
    ])
    error_message: String
    /// The business profile that is associated with this payment
    profile_id: String
    psd2_sca_exemption_type: AuthenticationResponsePsd2ScaExemptionType
    acquirer_details: AuthenticationResponseAcquirerDetails
}

structure AuthenticationResponseAcquirerDetails with [AcquirerDetails] {}

structure AuthenticationResponseAuthenticationConnector {}

structure AuthenticationResponsePsd2ScaExemptionType {}

/// UK BACS payment system
structure Bacs {
    /// 8-digit UK account number
    @required
    account_number: String
    /// 6-digit UK sort code
    @dataExamples([
        {
            json: "123456"
        }
    ])
    @required
    sort_code: String
    /// Account holder name
    @required
    name: String
    connector_recipient_id: String
}

structure BacsBankDebit {
    billing_details: BankDebitDataOneOfAlt3BacsBankDebitBillingDetails
    /// Account number for Bacs payment method
    @dataExamples([
        {
            json: "00012345"
        }
    ])
    @required
    account_number: String
    /// Sort code for Bacs payment method
    @dataExamples([
        {
            json: "108800"
        }
    ])
    @required
    sort_code: String
    /// holder name for bank debit
    @dataExamples([
        {
            json: "A. Schneider"
        }
    ])
    @required
    bank_account_holder_name: String
}

structure BacsBankDebitAdditionalData {
    /// Partially masked account number for Bacs payment method
    @dataExamples([
        {
            json: "0001****3456"
        }
    ])
    @required
    account_number: String
    /// Partially masked sort code for Bacs payment method
    @dataExamples([
        {
            json: "108800"
        }
    ])
    @required
    sort_code: String
    /// Bank account's owner name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    bank_account_holder_name: String
}

structure BacsBankTransfer {
    billing_details: BankTransferDataOneOfAlt2BacsBankTransferBillingDetails
}

/// Masked payout method details for bacs bank transfer payout method
structure BacsBankTransferAdditionalData {
    /// Partially masked sort code for Bacs payment method
    @dataExamples([
        {
            json: "108800"
        }
    ])
    @required
    bank_sort_code: String
    /// Bank account's owner name
    @dataExamples([
        {
            json: "0001****3456"
        }
    ])
    @required
    bank_account_number: String
    /// Bank name
    @dataExamples([
        {
            json: "Deutsche Bank"
        }
    ])
    bank_name: String
    bank_country_code: BacsBankTransferAdditionalDataBankCountryCode
    /// Bank city
    @dataExamples([
        {
            json: "California"
        }
    ])
    bank_city: String
}

structure BacsBankTransferAdditionalDataBankCountryCode {}

structure BacsBankTransferBankCountryCode {}

structure BacsBankTransferInstructions {
    @dataExamples([
        {
            json: "Jane Doe"
        }
    ])
    @required
    account_holder_name: String
    @dataExamples([
        {
            json: "10244123908"
        }
    ])
    @required
    account_number: String
    @dataExamples([
        {
            json: "012"
        }
    ])
    @required
    sort_code: String
}

structure BancontactBankRedirectAdditionalData {
    /// Last 4 digits of the card number
    @dataExamples([
        {
            json: "4242"
        }
    ])
    last4: String
    /// The card's expiry month
    @dataExamples([
        {
            json: "12"
        }
    ])
    card_exp_month: String
    /// The card's expiry year
    @dataExamples([
        {
            json: "24"
        }
    ])
    card_exp_year: String
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    card_holder_name: String
}

structure BancontactCard {
    /// The card number
    @dataExamples([
        {
            json: "4242424242424242"
        }
    ])
    @required
    card_number: String
    /// The card's expiry month
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_month: String
    /// The card's expiry year
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_year: String
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
    billing_details: BankRedirectDataOneOfAlt0BancontactCardBillingDetails
}

structure BankCodeResponse {
    @required
    bank_name: BankCodeResponseBankName
    @required
    eligible_connectors: BankCodeResponseEligibleConnectors
}

structure BankDebitAdditionalDataOneOfAlt0 {
    @required
    ach: AchBankDebitAdditionalData
}

structure BankDebitAdditionalDataOneOfAlt1 {
    @required
    bacs: BacsBankDebitAdditionalData
}

structure BankDebitAdditionalDataOneOfAlt2 {
    @required
    becs: BecsBankDebitAdditionalData
}

structure BankDebitAdditionalDataOneOfAlt3 {
    @required
    sepa: SepaBankDebitAdditionalData
}

@mixin
structure BankDebitBilling {
    /// The billing name for bank debits
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    name: String
    /// The billing email for bank debits
    @dataExamples([
        {
            json: "example@example.com"
        }
    ])
    email: String
    address: BankDebitBillingAddress
}

structure BankDebitBillingAddress with [AddressDetails] {}

structure BankDebitDataOneOfAlt0 {
    @required
    ach_bank_debit: AchBankDebit
}

structure BankDebitDataOneOfAlt0AchBankDebitBillingDetails with [BankDebitBilling] {}

structure BankDebitDataOneOfAlt1 {
    @required
    sepa_bank_debit: SepaBankDebit
}

structure BankDebitDataOneOfAlt1SepaBankDebitBillingDetails with [BankDebitBilling] {}

structure BankDebitDataOneOfAlt2 {
    @required
    becs_bank_debit: BecsBankDebit
}

structure BankDebitDataOneOfAlt2BecsBankDebitBillingDetails with [BankDebitBilling] {}

structure BankDebitDataOneOfAlt3 {
    @required
    bacs_bank_debit: BacsBankDebit
}

structure BankDebitDataOneOfAlt3BacsBankDebitBillingDetails with [BankDebitBilling] {}

structure BankDebitResponse {}

structure BankDebits with [BankDebitTypes] {}

@mixin
structure BankDebitTypes {
    @required
    eligible_connectors: BankDebitTypesEligibleConnectors
}

/// Swedish Bankgiro system
structure Bankgiro {
    /// Bankgiro number (7-8 digits)
    @dataExamples([
        {
            json: "5402-9656"
        }
    ])
    @required
    number: String
    /// Account holder name
    @dataExamples([
        {
            json: "Erik Andersson"
        }
    ])
    @required
    name: String
    connector_recipient_id: String
}

@mixin
structure BankRedirectBilling {
    /// The name for which billing is issued
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @required
    billing_name: String
    /// The billing email for bank redirect
    @dataExamples([
        {
            json: "example@example.com"
        }
    ])
    @required
    email: String
}

structure BankRedirectDataOneOfAlt0 {
    @required
    bancontact_card: BancontactCard
}

structure BankRedirectDataOneOfAlt0BancontactCardBillingDetails with [BankRedirectBilling] {}

structure BankRedirectDataOneOfAlt1 {
    @required
    bizum: Document
}

structure BankRedirectDataOneOfAlt10 {
    @required
    online_banking_slovakia: OnlineBankingSlovakia
}

structure BankRedirectDataOneOfAlt11 {
    @required
    open_banking_uk: OpenBankingUk
}

structure BankRedirectDataOneOfAlt12 {
    @required
    przelewy24: Przelewy24
}

structure BankRedirectDataOneOfAlt12Przelewy24BankName {}

structure BankRedirectDataOneOfAlt12Przelewy24BillingDetails with [BankRedirectBilling] {}

structure BankRedirectDataOneOfAlt13 {
    @required
    sofort: Sofort
}

structure BankRedirectDataOneOfAlt13SofortBillingDetails with [BankRedirectBilling] {}

structure BankRedirectDataOneOfAlt14 {
    @required
    trustly: Trustly
}

structure BankRedirectDataOneOfAlt15 {
    @required
    online_banking_fpx: OnlineBankingFpx
}

structure BankRedirectDataOneOfAlt16 {
    @required
    online_banking_thailand: OnlineBankingThailand
}

structure BankRedirectDataOneOfAlt17 {
    @required
    local_bank_redirect: Document
}

structure BankRedirectDataOneOfAlt18 {
    @required
    eft: Eft
}

structure BankRedirectDataOneOfAlt2 {
    @required
    blik: Blik
}

structure BankRedirectDataOneOfAlt3 {
    @required
    eps: Eps
}

structure BankRedirectDataOneOfAlt3EpsBillingDetails with [BankRedirectBilling] {}

structure BankRedirectDataOneOfAlt4 {
    @required
    giropay: Giropay
}

structure BankRedirectDataOneOfAlt4GiropayBillingDetails with [BankRedirectBilling] {}

structure BankRedirectDataOneOfAlt5 {
    @required
    ideal: Ideal
}

structure BankRedirectDataOneOfAlt5IdealBillingDetails with [BankRedirectBilling] {}

structure BankRedirectDataOneOfAlt6 {
    @required
    interac: Interac
}

structure BankRedirectDataOneOfAlt6InteracCountry {}

structure BankRedirectDataOneOfAlt7 {
    @required
    online_banking_czech_republic: OnlineBankingCzechRepublic
}

structure BankRedirectDataOneOfAlt8 {
    @required
    online_banking_finland: OnlineBankingFinland
}

structure BankRedirectDataOneOfAlt9 {
    @required
    online_banking_poland: OnlineBankingPoland
}

structure BankRedirectDetailsOneOfAlt0 {
    @required
    BancontactCard: BancontactBankRedirectAdditionalData
}

structure BankRedirectDetailsOneOfAlt1 {
    @required
    Blik: BlikBankRedirectAdditionalData
}

structure BankRedirectDetailsOneOfAlt2 {
    @required
    Giropay: GiropayBankRedirectAdditionalData
}

structure BankRedirectResponse {
    bank_name: BankRedirectResponseAllOf1BankName
}

structure BankRedirectResponseAllOf1BankName {}

structure BankTransferAdditionalDataOneOfAlt0 {
    @required
    ach: Document
}

structure BankTransferAdditionalDataOneOfAlt1 {
    @required
    sepa: Document
}

structure BankTransferAdditionalDataOneOfAlt10 {
    @required
    mandiri_va: Document
}

structure BankTransferAdditionalDataOneOfAlt11 {
    @required
    pix: PixBankTransferAdditionalData
}

structure BankTransferAdditionalDataOneOfAlt12 {
    @required
    pse: Document
}

structure BankTransferAdditionalDataOneOfAlt13 {
    @required
    local_bank_transfer: LocalBankTransferAdditionalData
}

structure BankTransferAdditionalDataOneOfAlt14 {
    @required
    instant_bank_transfer: Document
}

structure BankTransferAdditionalDataOneOfAlt15 {
    @required
    instant_bank_transfer_finland: Document
}

structure BankTransferAdditionalDataOneOfAlt16 {
    @required
    instant_bank_transfer_poland: Document
}

structure BankTransferAdditionalDataOneOfAlt2 {
    @required
    bacs: Document
}

structure BankTransferAdditionalDataOneOfAlt3 {
    @required
    multibanco: Document
}

structure BankTransferAdditionalDataOneOfAlt4 {
    @required
    permata: Document
}

structure BankTransferAdditionalDataOneOfAlt5 {
    @required
    bca: Document
}

structure BankTransferAdditionalDataOneOfAlt6 {
    @required
    bni_va: Document
}

structure BankTransferAdditionalDataOneOfAlt7 {
    @required
    bri_va: Document
}

structure BankTransferAdditionalDataOneOfAlt8 {
    @required
    cimb_va: Document
}

structure BankTransferAdditionalDataOneOfAlt9 {
    @required
    danamon_va: Document
}

structure BankTransferDataOneOfAlt0 {
    @required
    ach_bank_transfer: AchBankTransfer
}

structure BankTransferDataOneOfAlt0AchBankTransferBillingDetails with [AchBillingDetails] {}

structure BankTransferDataOneOfAlt1 {
    @required
    sepa_bank_transfer: SepaBankTransfer
}

structure BankTransferDataOneOfAlt10 {
    @required
    mandiri_va_bank_transfer: MandiriVaBankTransfer
}

structure BankTransferDataOneOfAlt10MandiriVaBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferDataOneOfAlt11 {
    @required
    pix: Pix
}

structure BankTransferDataOneOfAlt12 {
    @required
    pse: Document
}

structure BankTransferDataOneOfAlt13 {
    @required
    local_bank_transfer: LocalBankTransfer
}

structure BankTransferDataOneOfAlt14 {
    @required
    instant_bank_transfer: Document
}

structure BankTransferDataOneOfAlt15 {
    @required
    instant_bank_transfer_finland: Document
}

structure BankTransferDataOneOfAlt16 {
    @required
    instant_bank_transfer_poland: Document
}

structure BankTransferDataOneOfAlt1SepaBankTransferBillingDetails with [SepaAndBacsBillingDetails] {}

structure BankTransferDataOneOfAlt2 {
    @required
    bacs_bank_transfer: BacsBankTransfer
}

structure BankTransferDataOneOfAlt2BacsBankTransferBillingDetails with [SepaAndBacsBillingDetails] {}

structure BankTransferDataOneOfAlt3 {
    @required
    multibanco_bank_transfer: MultibancoBankTransfer
}

structure BankTransferDataOneOfAlt3MultibancoBankTransferBillingDetails with [MultibancoBillingDetails] {}

structure BankTransferDataOneOfAlt4 {
    @required
    permata_bank_transfer: PermataBankTransfer
}

structure BankTransferDataOneOfAlt4PermataBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferDataOneOfAlt5 {
    @required
    bca_bank_transfer: BcaBankTransfer
}

structure BankTransferDataOneOfAlt5BcaBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferDataOneOfAlt6 {
    @required
    bni_va_bank_transfer: BniVaBankTransfer
}

structure BankTransferDataOneOfAlt6BniVaBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferDataOneOfAlt7 {
    @required
    bri_va_bank_transfer: BriVaBankTransfer
}

structure BankTransferDataOneOfAlt7BriVaBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferDataOneOfAlt8 {
    @required
    cimb_va_bank_transfer: CimbVaBankTransfer
}

structure BankTransferDataOneOfAlt8CimbVaBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferDataOneOfAlt9 {
    @required
    danamon_va_bank_transfer: DanamonVaBankTransfer
}

structure BankTransferDataOneOfAlt9DanamonVaBankTransferBillingDetails with [DokuBillingDetails] {}

structure BankTransferInstructionsOneOfAlt0 {
    @required
    doku_bank_transfer_instructions: DokuBankTransferInstructions
}

structure BankTransferInstructionsOneOfAlt1 {
    @required
    ach_credit_transfer: AchTransfer
}

structure BankTransferInstructionsOneOfAlt2 {
    @required
    sepa_bank_instructions: SepaBankTransferInstructions
}

structure BankTransferInstructionsOneOfAlt3 {
    @required
    bacs_bank_instructions: BacsBankTransferInstructions
}

structure BankTransferInstructionsOneOfAlt4 {
    @required
    multibanco: MultibancoTransferInstructions
}

structure BankTransferNextStepsData {
    receiver: Receiver
}

structure BankTransferResponse {}

structure BankTransfers with [BankTransferTypes] {}

@mixin
structure BankTransferTypes {
    @required
    eligible_connectors: BankTransferTypesEligibleConnectors
}

structure BcaBankTransfer {
    billing_details: BankTransferDataOneOfAlt5BcaBankTransferBillingDetails
}

structure BecsBankDebit {
    billing_details: BankDebitDataOneOfAlt2BecsBankDebitBillingDetails
    /// Account number for Becs payment method
    @dataExamples([
        {
            json: "000123456"
        }
    ])
    @required
    account_number: String
    /// Bank-State-Branch (bsb) number
    @dataExamples([
        {
            json: "000000"
        }
    ])
    @required
    bsb_number: String
    /// Owner name for bank debit
    @dataExamples([
        {
            json: "A. Schneider"
        }
    ])
    bank_account_holder_name: String
}

structure BecsBankDebitAdditionalData {
    /// Partially masked account number for Becs payment method
    @dataExamples([
        {
            json: "0001****3456"
        }
    ])
    @required
    account_number: String
    /// Bank-State-Branch (bsb) number
    @dataExamples([
        {
            json: "000000"
        }
    ])
    @required
    bsb_number: String
    /// Bank account's owner name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    bank_account_holder_name: String
}

structure BillingAddressParameters with [GpayBillingAddressParameters] {}

structure BillingCountry {}

structure Blik {
    blik_code: String
}

structure BlikBankRedirectAdditionalData {
    @dataExamples([
        {
            json: "3GD9MO"
        }
    ])
    blik_code: String
}

structure BlockAFingerprint200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: BlocklistResponse
}

@error("client")
@httpError(400)
structure BlockAFingerprint400 {}

structure BlockAFingerprintInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: BlocklistRequest
}

structure BlocklistRequestOneOfAlt0 {
    @required
    data: String
}

structure BlocklistRequestOneOfAlt1 {
    @required
    data: String
}

structure BlocklistRequestOneOfAlt2 {
    @required
    data: String
}

structure BlocklistResponse {
    @required
    fingerprint_id: String
    @required
    data_kind: BlocklistDataKind
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
}

structure BniVaBankTransfer {
    billing_details: BankTransferDataOneOfAlt6BniVaBankTransferBillingDetails
}

structure BoletoVoucherData {
    /// The shopper's social security number
    social_security_number: String
}

structure Braintree with [BraintreeData] {}

@mixin
structure BraintreeData {
    /// Information about the merchant_account_id that merchant wants to specify at connector level.
    @required
    merchant_account_id: String
    /// Information about the merchant_config_currency that merchant wants to specify at connector level.
    @required
    merchant_config_currency: String
}

structure BriVaBankTransfer {
    billing_details: BankTransferDataOneOfAlt7BriVaBankTransferBillingDetails
}

/// Browser information to be used for 3DS 2.0
@mixin
structure BrowserInformation {
    /// Color depth supported by the browser
    @range(
        min: 0
    )
    color_depth: Integer
    /// Whether java is enabled in the browser
    java_enabled: Boolean
    /// Whether javascript is enabled in the browser
    java_script_enabled: Boolean
    /// Language supported
    language: String
    /// The screen height in pixels
    @range(
        min: 0
    )
    screen_height: Integer
    /// The screen width in pixels
    @range(
        min: 0
    )
    screen_width: Integer
    /// Time zone of the client
    time_zone: Integer
    /// Ip address of the client
    ip_address: String
    /// List of headers that are accepted
    @dataExamples([
        {
            json: "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8"
        }
    ])
    accept_header: String
    /// User-agent of the browser
    user_agent: String
    /// The os type of the client device
    os_type: String
    /// The os version of the client device
    os_version: String
    /// The device model of the client
    device_model: String
}

@mixin
structure BusinessCollectLinkConfig with [BusinessGenericLinkConfig] {
    @required
    enabled_payment_methods: BusinessCollectLinkConfigAllOf1EnabledPaymentMethods
}

@mixin
structure BusinessGenericLinkConfig with [GenericLinkUiConfig] {
    /// Custom domain name to be used for hosting the link
    domain_name: String
    @required
    allowed_domains: BusinessGenericLinkConfigAllOf1AllowedDomains
}

@mixin
structure BusinessPayoutLinkConfig with [BusinessGenericLinkConfig] {
    form_layout: BusinessPayoutLinkConfigAllOf1FormLayout
    /// Allows for removing any validations / pre-requisites which are necessary in a production environment
    payout_test_mode: Boolean
}

structure BusinessPayoutLinkConfigAllOf1FormLayout {}

@error("client")
@httpError(400)
structure CancelAPayment400 {}

structure CancelAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCancelRequest
}

structure CancelAPayout200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCreateResponse
}

@error("client")
@httpError(400)
structure CancelAPayout400 {}

structure CancelAPayoutInput {
    /// The identifier for payout
    @httpLabel
    @required
    payout_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCancelRequest
}

structure CaptureAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsResponse
}

@error("client")
@httpError(400)
structure CaptureAPayment400 {}

structure CaptureAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCaptureRequest
}

structure CaptureResponse {
    /// A unique identifier for this specific capture operation.
    @required
    capture_id: String
    @required
    status: CaptureStatus
    /// The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    currency: CaptureResponseCurrency
    /// The name of the payment connector that processed this capture.
    @required
    connector: String
    /// The ID of the payment attempt that was successfully authorized and subsequently captured by this operation.
    @required
    authorized_attempt_id: String
    /// A unique identifier for this capture provided by the connector
    connector_capture_id: String
    /// Sequence number of this capture, in the series of captures made for the parent attempt
    @required
    capture_sequence: Integer
    /// A human-readable message from the connector explaining why this capture operation failed, if applicable.
    error_message: String
    /// The error code returned by the connector if this capture operation failed. This code is connector-specific.
    error_code: String
    /// A more detailed reason from the connector explaining the capture failure, if available.
    error_reason: String
    /// The connector's own reference or transaction ID for this specific capture operation. Useful for reconciliation.
    reference_id: String
}

structure CaptureResponseCurrency {}

structure Card {
    /// The card number
    @dataExamples([
        {
            json: "4242424242424242"
        }
    ])
    @required
    card_number: String
    /// The card's expiry month
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_month: String
    /// The card's expiry year
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_year: String
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
    /// The CVC number for the card
    @dataExamples([
        {
            json: "242"
        }
    ])
    @required
    card_cvc: String
    /// The name of the issuer of card
    @dataExamples([
        {
            json: "chase"
        }
    ])
    card_issuer: String
    card_network: CardCardNetwork
    @dataExamples([
        {
            json: "CREDIT"
        }
    ])
    card_type: String
    @dataExamples([
        {
            json: "INDIA"
        }
    ])
    card_issuing_country: String
    @dataExamples([
        {
            json: "JP_AMEX"
        }
    ])
    bank_code: String
    /// The card holder's nick name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    nick_name: String
}

/// Masked payout method details for card payout method
structure CardAdditionalData {
    /// Issuer of the card
    card_issuer: String
    card_network: CardAdditionalDataCardNetwork
    /// Card type, can be either `credit` or `debit`
    card_type: String
    /// Card issuing country
    card_issuing_country: String
    /// Code for Card issuing bank
    bank_code: String
    /// Last 4 digits of the card number
    last4: String
    /// The ISIN of the card
    card_isin: String
    /// Extended bin of card, contains the first 8 digits of card number
    card_extended_bin: String
    /// Card expiry month
    @dataExamples([
        {
            json: "01"
        }
    ])
    @required
    card_exp_month: String
    /// Card expiry year
    @dataExamples([
        {
            json: "2026"
        }
    ])
    @required
    card_exp_year: String
    /// Card holder name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @required
    card_holder_name: String
}

structure CardAdditionalDataCardNetwork {}

structure CardCardNetwork {}

structure CardDetail with [CardDetailMixin] {}

structure CardDetailCardNetwork {}

@mixin
structure CardDetailFromLocker {
    scheme: String
    issuer_country: String
    last4_digits: String
    expiry_month: String
    expiry_year: String
    card_token: String
    card_holder_name: String
    card_fingerprint: String
    nick_name: String
    card_network: CardDetailFromLockerCardNetwork
    card_isin: String
    card_issuer: String
    card_type: String
    @required
    saved_to_locker: Boolean
}

structure CardDetailFromLockerCardNetwork {}

@mixin
structure CardDetailMixin {
    /// Card Number
    @dataExamples([
        {
            json: "4111111145551142"
        }
    ])
    @required
    card_number: String
    /// Card Expiry Month
    @dataExamples([
        {
            json: "10"
        }
    ])
    @required
    card_exp_month: String
    /// Card Expiry Year
    @dataExamples([
        {
            json: "25"
        }
    ])
    @required
    card_exp_year: String
    /// Card Holder Name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @required
    card_holder_name: String
    /// Card Holder's Nick Name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    nick_name: String
    /// Card Issuing Country
    card_issuing_country: String
    card_network: CardDetailCardNetwork
    /// Issuer Bank for Card
    card_issuer: String
    /// Card Type
    card_type: String
}

@mixin
structure CardDetailUpdate {
    /// Card Expiry Month
    @dataExamples([
        {
            json: "10"
        }
    ])
    @required
    card_exp_month: String
    /// Card Expiry Year
    @dataExamples([
        {
            json: "25"
        }
    ])
    @required
    card_exp_year: String
    /// Card Holder Name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @required
    card_holder_name: String
    /// Card Holder's Nick Name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    nick_name: String
}

structure CardNetworkTokenizeRequest {
    /// Merchant ID associated with the tokenization request
    @dataExamples([
        {
            json: "merchant_1671528864"
        }
    ])
    @required
    merchant_id: String
    @required
    customer: CustomerDetails
    billing: CardNetworkTokenizeRequestAllOf1Billing
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// The name of the bank/ provider issuing the payment method to the end user
    payment_method_issuer: String
}

structure CardNetworkTokenizeResponse {
    payment_method_response: PaymentMethodResponse
    @required
    customer: CustomerDetails
    /// Card network tokenization status
    @required
    card_tokenized: Boolean
    /// Error code
    error_code: String
    /// Error message
    error_message: String
    tokenization_data: TokenizationData
}

structure CardNetworkTypes {
    card_network: CardNetworkTypesCardNetwork
    surcharge_details: CardNetworkTypesSurchargeDetails
    @required
    eligible_connectors: CardNetworkTypesEligibleConnectors
}

structure CardNetworkTypesCardNetwork {}

structure CardNetworkTypesSurchargeDetails with [SurchargeDetailsResponse] {}

structure CardPayout {
    /// The card number
    @dataExamples([
        {
            json: "4242424242424242"
        }
    ])
    @required
    card_number: String
    /// The card's expiry month
    @required
    expiry_month: String
    /// The card's expiry year
    @required
    expiry_year: String
    /// The card holder's name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @required
    card_holder_name: String
}

structure CardRedirectDataOneOfAlt0 {
    @required
    knet: Document
}

structure CardRedirectDataOneOfAlt1 {
    @required
    benefit: Document
}

structure CardRedirectDataOneOfAlt2 {
    @required
    momo_atm: Document
}

structure CardRedirectDataOneOfAlt3 {
    @required
    card_redirect: Document
}

structure CardRedirectResponse {}

structure CardResponse {
    last4: String
    card_type: String
    card_network: CardResponseCardNetwork
    card_issuer: String
    card_issuing_country: String
    card_isin: String
    card_extended_bin: String
    card_exp_month: String
    card_exp_year: String
    card_holder_name: String
    payment_checks: PaymentChecks
    authentication_data: AuthenticationData
}

structure CardResponseCardNetwork {}

structure CardSpecificFeatures {
    @required
    three_ds: FeatureStatus
    @required
    no_three_ds: FeatureStatus
    @required
    supported_card_networks: SupportedCardNetworks
}

@mixin
structure CardTestingGuardConfig {
    @required
    card_ip_blocking_status: CardTestingGuardStatus
    /// Determines the unsuccessful payment threshold for Card IP Blocking for profile
    @required
    card_ip_blocking_threshold: Integer
    @required
    guest_user_card_blocking_status: CardTestingGuardStatus
    /// Determines the unsuccessful payment threshold for Guest User Card Blocking for profile
    @required
    guest_user_card_blocking_threshold: Integer
    @required
    customer_id_blocking_status: CardTestingGuardStatus
    /// Determines the unsuccessful payment threshold for Customer Id Blocking for profile
    @required
    customer_id_blocking_threshold: Integer
    /// Determines Redis Expiry for Card Testing Guard for profile
    @required
    card_testing_guard_expiry: Integer
}

structure CardToken {
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
    /// The CVC number for the card
    card_cvc: String
}

structure CardTokenAdditionalData {
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
}

structure CardTokenResponse {}

structure CardType {}

structure Category {}

/// Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.
structure ChargeRefunds {
    /// Identifier for charge created for the payment
    @required
    charge_id: String
    /// Toggle for reverting the application fee that was collected for the payment.
    /// If set to false, the funds are pulled from the destination account.
    revert_platform_fee: Boolean
    /// Toggle for reverting the transfer that was made during the charge.
    /// If set to false, the funds are pulled from the main platform's account.
    revert_transfer: Boolean
}

structure CimbVaBankTransfer {
    billing_details: BankTransferDataOneOfAlt8CimbVaBankTransferBillingDetails
}

@mixin
structure ClickToPaySessionResponse {
    @required
    dpa_id: String
    @required
    dpa_name: String
    @required
    locale: String
    @required
    card_brands: CardBrands
    @required
    acquirer_bin: String
    @required
    acquirer_merchant_id: String
    @required
    merchant_category_code: String
    @required
    merchant_country_code: String
    @dataExamples([
        {
            json: "38.02"
        }
    ])
    @required
    transaction_amount: String
    @required
    transaction_currency_code: Currency
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone_number: String
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    phone_country_code: String
    provider: ClickToPaySessionResponseProvider
    dpa_client_id: String
}

structure ClickToPaySessionResponseProvider {}

/// Represents a single comparison condition.
structure Comparison {
    /// The left hand side which will always be a domain input identifier like "payment.method.cardtype"
    @required
    lhs: String
    @required
    comparison: ComparisonType
    @required
    value: ValueType
    @required
    metadata: ComparisonMetadata
}

structure CompleteAuthorizeAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsResponse
}

@error("client")
@httpError(400)
structure CompleteAuthorizeAPayment400 {}

structure CompleteAuthorizeAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCompleteAuthorizeRequest
}

structure ConfirmAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCreateResponseOpenApi
}

@error("client")
@httpError(400)
structure ConfirmAPayment400 {}

structure ConfirmAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsConfirmRequest
}

structure ConfirmAPayout200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCreateResponse
}

@error("client")
@httpError(400)
structure ConfirmAPayout400 {}

structure ConfirmAPayoutInput {
    /// The identifier for payout
    @httpLabel
    @required
    payout_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutConfirmRequest
}

structure ConnectorChargeResponseDataOneOfAlt0 {
    @required
    stripe_split_payment: StripeChargeResponseData
}

structure ConnectorChargeResponseDataOneOfAlt1 {
    @required
    adyen_split_payment: AdyenSplitData
}

structure ConnectorChargeResponseDataOneOfAlt2 {
    @required
    xendit_split_payment: XenditChargeResponseData
}

structure ConnectorFeatureMatrixResponse {
    /// The name of the connector
    @required
    name: String
    /// The display name of the connector
    display_name: String
    /// The description of the connector
    description: String
    category: Category
    @required
    supported_payment_methods: SupportedPaymentMethods
    supported_webhook_flows: SupportedWebhookFlows
}

/// Some connectors like Apple Pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
@mixin
structure ConnectorMetadata {
    apple_pay: ApplePay
    airwallex: Airwallex
    noon: Noon
    braintree: Braintree
    adyen: Adyen
}

structure ConnectorSelectionOneOfAlt0 {
    @required
    data: ConnectorSelectionOneOfAlt0Data
}

structure ConnectorSelectionOneOfAlt1 {
    @required
    data: ConnectorSelectionOneOfAlt1Data
}

structure ConnectorVolumeSplit {
    @required
    connector: RoutableConnectorChoice
    @range(
        min: 0
    )
    @required
    split: Integer
}

@mixin
structure ConnectorWalletDetails {
    /// This field contains the Apple Pay certificates and credentials for iOS and Web Apple Pay flow
    apple_pay_combined: Document
    /// This field is for our legacy Apple Pay flow that contains the Apple Pay certificates and credentials for only iOS Apple Pay flow
    apple_pay: Document
    /// This field contains the Samsung Pay certificates and credentials
    samsung_pay: Document
    /// This field contains the Paze certificates and credentials
    paze: Document
    /// This field contains the Google Pay certificates and credentials
    google_pay: Document
}

structure ContractBasedRoutingConfig {
    config: ContractBasedRoutingConfigConfig
    label_info: LabelInfo
}

@mixin
structure ContractBasedRoutingConfigBody {
    constants: Constants
    time_scale: TimeScale
}

structure ContractBasedRoutingConfigConfig with [ContractBasedRoutingConfigBody] {}

structure CreateACustomer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerResponse
}

@error("client")
@httpError(400)
structure CreateACustomer400 {}

structure CreateACustomerInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerRequest
}

structure CreateAMerchantAccount200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantAccountResponse
}

@error("client")
@httpError(400)
structure CreateAMerchantAccount400 {}

structure CreateAMerchantAccountInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantAccountCreate
}

structure CreateAMerchantConnector200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantConnectorResponse
}

@error("client")
@httpError(400)
structure CreateAMerchantConnector400 {}

structure CreateAMerchantConnectorInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantConnectorCreate
}

structure CreateAnAPIKey200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CreateApiKeyResponse
}

@error("client")
@httpError(400)
structure CreateAnAPIKey400 {}

structure CreateAnAPIKeyInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    merchant_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: CreateApiKeyRequest
}

structure CreateAnAuthentication200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: AuthenticationResponse
}

@error("client")
@httpError(400)
structure CreateAnAuthentication400 {}

structure CreateAnAuthenticationInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: AuthenticationCreateRequest
}

structure CreateAnOrganization200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: OrganizationResponse
}

@error("client")
@httpError(400)
structure CreateAnOrganization400 {}

structure CreateAnOrganizationInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: OrganizationCreateRequest
}

structure CreateAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCreateResponseOpenApi
}

@error("client")
@httpError(400)
structure CreateAPayment400 {}

structure CreateAPaymentInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCreateRequest
}

structure CreateAPaymentMethod200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodResponse
}

@error("client")
@httpError(400)
structure CreateAPaymentMethod400 {}

structure CreateAPaymentMethodInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodCreate
}

structure CreateAPayout200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCreateResponse
}

@error("client")
@httpError(400)
structure CreateAPayout400 {}

structure CreateAPayoutInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutsCreateRequest
}

/// The request body for creating an API Key.
structure CreateApiKeyRequest {
    /// A unique name for the API Key to help you identify it.
    @dataExamples([
        {
            json: "Sandbox integration key"
        }
    ])
    @length(
        max: 64
    )
    @required
    name: String
    /// A description to provide more context about the API Key.
    @dataExamples([
        {
            json: "Key used by our developers to integrate with the sandbox environment"
        }
    ])
    @length(
        max: 256
    )
    description: String
    @required
    expiration: ApiKeyExpiration
}

/// The response body for creating an API Key.
structure CreateApiKeyResponse {
    /// The identifier for the API Key.
    @dataExamples([
        {
            json: "5hEEqkgJUyuxgSKGArHA4mWSnX"
        }
    ])
    @length(
        max: 64
    )
    @required
    key_id: String
    /// The identifier for the Merchant Account.
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// The unique name for the API Key to help you identify it.
    @dataExamples([
        {
            json: "Sandbox integration key"
        }
    ])
    @length(
        max: 64
    )
    @required
    name: String
    /// The description to provide more context about the API Key.
    @dataExamples([
        {
            json: "Key used by our developers to integrate with the sandbox environment"
        }
    ])
    @length(
        max: 256
    )
    description: String
    /// The plaintext API Key used for server-side API access. Ensure you store the API Key
    /// securely as you will not be able to see it again.
    @length(
        max: 128
    )
    @required
    api_key: String
    /// The time at which the API Key was created.
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    created: Timestamp
    @required
    expiration: ApiKeyExpiration
}

structure CreateAProfile200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileResponse
}

@error("client")
@httpError(400)
structure CreateAProfile400 {}

structure CreateAProfileAcquirer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileAcquirerResponse
}

@error("client")
@httpError(400)
structure CreateAProfileAcquirer400 {}

structure CreateAProfileAcquirerInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileAcquirerCreate
}

structure CreateAProfileInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileCreate
}

structure CreateARefund200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundResponse
}

@error("client")
@httpError(400)
structure CreateARefund400 {}

structure CreateARefundInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundRequest
}

structure CreateARoutingConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure CreateARoutingConfig400 {}

@error("client")
@httpError(403)
structure CreateARoutingConfig403 {}

@error("client")
@httpError(404)
structure CreateARoutingConfig404 {}

@error("client")
@httpError(422)
structure CreateARoutingConfig422 {}

@error("server")
@httpError(500)
structure CreateARoutingConfig500 {}

structure CreateARoutingConfigInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingConfigRequest
}

structure CreateGsmRule200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmResponse
}

@error("client")
@httpError(400)
structure CreateGsmRule400 {}

structure CreateGsmRuleInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmCreateRequest
}

structure CreatePostSessionTokensForAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsPostSessionTokensResponse
}

@error("client")
@httpError(400)
structure CreatePostSessionTokensForAPayment400 {}

structure CreatePostSessionTokensForAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsPostSessionTokensRequest
}

structure CreateSessionTokensForAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsSessionResponse
}

@error("client")
@httpError(400)
structure CreateSessionTokensForAPayment400 {}

structure CreateSessionTokensForAPaymentInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsSessionRequest
}

structure CryptoData with [CryptoDataMixin] {}

@mixin
structure CryptoDataMixin {
    pay_currency: String
    network: String
}

structure CryptoResponse {}

@mixin
structure CtpServiceDetails {
    /// merchant transaction id
    merchant_transaction_id: String
    /// network transaction correlation id
    correlation_id: String
    /// session transaction flow id
    x_src_flow_id: String
    provider: CtpServiceDetailsProvider
    /// Encrypted payload
    encrypted_payload: String
}

structure CtpServiceDetailsProvider {}

/// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
@mixin
structure CustomerAcceptance {
    @required
    acceptance_type: AcceptanceType
    /// Specifying when the customer acceptance was provided
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    accepted_at: Timestamp
    online: Online
}

structure CustomerDefaultPaymentMethodResponse {
    /// The unique identifier of the Payment method
    @dataExamples([
        {
            json: "card_rGK4Vi5iSW70MY7J2mIg"
        }
    ])
    default_payment_method_id: String
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    customer_id: String
    @required
    payment_method: PaymentMethod
    payment_method_type: CustomerDefaultPaymentMethodResponsePaymentMethodType
}

structure CustomerDefaultPaymentMethodResponsePaymentMethodType {}

structure CustomerDeleteResponse {
    /// The identifier for the customer object
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    @required
    customer_id: String
    /// Whether customer was deleted or not
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    customer_deleted: Boolean
    /// Whether address was deleted or not
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    address_deleted: Boolean
    /// Whether payment methods deleted or not
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    payment_methods_deleted: Boolean
}

/// Details of customer attached to this payment
@mixin
structure CustomerDetailsResponse {
    /// The identifier for the customer.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    id: String
    /// The customer's name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's email address
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// The customer's phone number
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 10
    )
    phone: String
    /// The country code for the customer's phone number
    @dataExamples([
        {
            json: "+1"
        }
    ])
    @length(
        max: 2
    )
    phone_country_code: String
}

structure CustomerDevice with [CustomerDeviceData] {}

/// Represents data about the customer's device used in the 3DS decision rule.
@mixin
structure CustomerDeviceData {
    platform: Platform
    device_type: DeviceType
    display_size: DisplaySize
}

structure CustomerPaymentMethod {
    /// Token for payment method in temporary card locker which gets refreshed often
    @dataExamples([
        {
            json: "7ebf443f-a050-4067-84e5-e6f6d4800aef"
        }
    ])
    @required
    payment_token: String
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "pm_iouuy468iyuowqs"
        }
    ])
    @required
    payment_method_id: String
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    customer_id: String
    @required
    payment_method: PaymentMethod
    payment_method_type: CustomerPaymentMethodPaymentMethodType
    /// The name of the bank/ provider issuing the payment method to the end user
    @dataExamples([
        {
            json: "Citibank"
        }
    ])
    payment_method_issuer: String
    payment_method_issuer_code: CustomerPaymentMethodPaymentMethodIssuerCode
    /// Indicates whether the payment method supports recurring payments. Optional.
    @dataExamples([
        {
            json: true
        }
    ])
    recurring_enabled: Boolean
    /// Indicates whether the payment method is eligible for installment payments (e.g., EMI, BNPL). Optional.
    @dataExamples([
        {
            json: true
        }
    ])
    installment_payment_enabled: Boolean
    payment_experience: CustomerPaymentMethodPaymentExperience
    card: CustomerPaymentMethodCard
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// A timestamp (ISO 8601 code) that determines when the payment method was created
    @dataExamples([
        {
            json: "2023-01-18T11:04:09.922Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
    bank_transfer: CustomerPaymentMethodBankTransfer
    bank: CustomerPaymentMethodBank
    surcharge_details: CustomerPaymentMethodSurchargeDetails
    /// Whether this payment method requires CVV to be collected
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    requires_cvv: Boolean
    /// A timestamp (ISO 8601 code) that determines when the payment method was last used
    @dataExamples([
        {
            json: "2024-02-24T11:04:09.922Z"
        }
    ])
    @timestampFormat("date-time")
    last_used_at: Timestamp
    /// Indicates if the payment method has been set to default or not
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    default_payment_method_set: Boolean
    billing: CustomerPaymentMethodBilling
}

structure CustomerPaymentMethodBank with [MaskedBankDetails] {}

structure CustomerPaymentMethodBankTransfer {}

structure CustomerPaymentMethodCard with [CardDetailFromLocker] {}

structure CustomerPaymentMethodPaymentMethodIssuerCode {}

structure CustomerPaymentMethodPaymentMethodType {}

structure CustomerPaymentMethodsListResponse {
    @required
    customer_payment_methods: CustomerPaymentMethods
    /// Returns whether a customer id is not tied to a payment intent (only when the request is made against a client secret)
    is_guest_customer: Boolean
}

structure CustomerPaymentMethodSurchargeDetails with [SurchargeDetailsResponse] {}

/// The customer details
structure CustomerRequest {
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// The customer's name
    @dataExamples([
        {
            json: "Jon Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's email address
    @dataExamples([
        {
            json: "JonTest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// The customer's phone number
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// An arbitrary string that you can attach to a customer object.
    @dataExamples([
        {
            json: "First Customer"
        }
    ])
    @length(
        max: 255
    )
    description: String
    /// The country code for the customer phone number
    @dataExamples([
        {
            json: "+65"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    address: CustomerRequestAddress
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    metadata: Document
}

structure CustomerRequestAddress with [AddressDetails] {}

structure CustomerResponse {
    /// The identifier for the customer object
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    customer_id: String
    /// The customer's name
    @dataExamples([
        {
            json: "Jon Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's email address
    @dataExamples([
        {
            json: "JonTest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// The customer's phone number
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// The country code for the customer phone number
    @dataExamples([
        {
            json: "+65"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    /// An arbitrary string that you can attach to a customer object.
    @dataExamples([
        {
            json: "First Customer"
        }
    ])
    @length(
        max: 255
    )
    description: String
    address: CustomerResponseAddress
    /// A timestamp (ISO 8601 code) that determines when the customer was created
    @dataExamples([
        {
            json: "2023-01-18T11:04:09.922Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    metadata: Document
    /// The identifier for the default payment method.
    @dataExamples([
        {
            json: "pm_djh2837dwduh890123"
        }
    ])
    @length(
        max: 64
    )
    default_payment_method_id: String
}

structure CustomerResponseAddress with [AddressDetails] {}

/// The identifier for the customer object. If not provided the customer ID will be autogenerated.
structure CustomerUpdateRequest {
    /// The customer's name
    @dataExamples([
        {
            json: "Jon Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's email address
    @dataExamples([
        {
            json: "JonTest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// The customer's phone number
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// An arbitrary string that you can attach to a customer object.
    @dataExamples([
        {
            json: "First Customer"
        }
    ])
    @length(
        max: 255
    )
    description: String
    /// The country code for the customer phone number
    @dataExamples([
        {
            json: "+65"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    address: CustomerUpdateRequestAddress
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    metadata: Document
}

structure CustomerUpdateRequestAddress with [AddressDetails] {}

structure DanamonVaBankTransfer {
    billing_details: BankTransferDataOneOfAlt9DanamonVaBankTransferBillingDetails
}

structure DeactivateARoutingConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure DeactivateARoutingConfig400 {}

@error("client")
@httpError(403)
structure DeactivateARoutingConfig403 {}

@error("client")
@httpError(422)
structure DeactivateARoutingConfig422 {}

@error("server")
@httpError(500)
structure DeactivateARoutingConfig500 {}

structure DeactivateARoutingConfigInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingConfigRequest
}

structure Decision {}

structure DecisionEngineEliminationData {
    @required
    threshold: Double
}

structure DecisionEngineGatewayWiseExtraScore {
    @required
    gatewayName: String
    @required
    gatewaySigmaFactor: Double
}

structure DecisionEngineSRSubLevelInputConfig {
    paymentMethodType: String
    paymentMethod: String
    latencyThreshold: Double
    bucketSize: Integer
    hedgingPercent: Double
    lowerResetFactor: Double
    upperResetFactor: Double
    gatewayExtraScore: GatewayExtraScore
}

structure DecisionEngineSuccessRateData {
    defaultLatencyThreshold: Double
    defaultBucketSize: Integer
    defaultHedgingPercent: Double
    defaultLowerResetFactor: Double
    defaultUpperResetFactor: Double
    defaultGatewayExtraScore: DefaultGatewayExtraScore
    subLevelInputConfig: SubLevelInputConfig
}

structure DefaultPaymentMethod {
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    customer_id: String
    @required
    payment_method_id: String
}

structure DeleteACustomer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerDeleteResponse
}

@error("client")
@httpError(404)
structure DeleteACustomer404 {}

structure DeleteACustomerInput {
    /// The unique identifier for the Customer
    @httpLabel
    @required
    customer_id: String
}

structure DeleteAMerchantAccount200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantAccountDeleteResponse
}

@error("client")
@httpError(404)
structure DeleteAMerchantAccount404 {}

structure DeleteAMerchantAccountInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
}

structure DeleteAMerchantConnector200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantConnectorDeleteResponse
}

@error("client")
@httpError(401)
structure DeleteAMerchantConnector401 {}

@error("client")
@httpError(404)
structure DeleteAMerchantConnector404 {}

structure DeleteAMerchantConnectorInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    /// The unique identifier for the Merchant Connector
    @httpLabel
    @required
    merchant_connector_id: String
}

structure DeleteAPaymentMethod200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodDeleteResponse
}

@error("client")
@httpError(404)
structure DeleteAPaymentMethod404 {}

structure DeleteAPaymentMethodInput {
    /// The unique identifier for the Payment Method
    @httpLabel
    @required
    method_id: String
}

structure DeleteGsmRule200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmDeleteResponse
}

@error("client")
@httpError(400)
structure DeleteGsmRule400 {}

structure DeleteGsmRuleInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmDeleteRequest
}

structure DeleteTheProfile200 {
    @httpPayload
    @required
    @contentType("text/plain")
    body: Boolean
}

@error("client")
@httpError(400)
structure DeleteTheProfile400 {}

structure DeleteTheProfileInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    /// The unique identifier for the profile
    @httpLabel
    @required
    profile_id: String
}

structure DeliveryAttempt {}

structure DeviceType {}

structure DirectCarrierBilling {
    /// The phone number of the user
    @dataExamples([
        {
            json: "1234567890"
        }
    ])
    @required
    msisdn: String
    /// Unique user id
    @dataExamples([
        {
            json: "02iacdYXGI9CnyJdoN8c7"
        }
    ])
    client_uid: String
}

structure DisplayAmountOnSdk {
    /// net amount = amount + order_tax_amount + shipping_cost
    @required
    net_amount: String
    /// order tax amount calculated by tax connectors
    @required
    order_tax_amount: String
    /// shipping cost for the order
    @required
    shipping_cost: String
}

structure DisplaySize {}

structure DisputeResponse {
    /// The identifier for dispute
    @required
    dispute_id: String
    /// The identifier for payment_intent
    @required
    payment_id: String
    /// The identifier for payment_attempt
    @required
    attempt_id: String
    @required
    amount: StringMinorUnit
    @required
    currency: Currency
    @required
    dispute_stage: DisputeStage
    @required
    dispute_status: DisputeStatus
    /// connector to which dispute is associated with
    @required
    connector: String
    /// Status of the dispute sent by connector
    @required
    connector_status: String
    /// Dispute id sent by connector
    @required
    connector_dispute_id: String
    /// Reason of dispute sent by connector
    connector_reason: String
    /// Reason code of dispute sent by connector
    connector_reason_code: String
    /// Evidence deadline of dispute sent by connector
    @timestampFormat("date-time")
    challenge_required_by: Timestamp
    /// Dispute created time sent by connector
    @timestampFormat("date-time")
    connector_created_at: Timestamp
    /// Dispute updated time sent by connector
    @timestampFormat("date-time")
    connector_updated_at: Timestamp
    /// Time at which dispute is received
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
    /// The `profile_id` associated with the dispute
    profile_id: String
    /// The `merchant_connector_id` of the connector / processor through which the dispute was processed
    merchant_connector_id: String
}

structure DisputeResponsePaymentsRetrieve {
    /// The identifier for dispute
    @required
    dispute_id: String
    @required
    dispute_stage: DisputeStage
    @required
    dispute_status: DisputeStatus
    /// Status of the dispute sent by connector
    @required
    connector_status: String
    /// Dispute id sent by connector
    @required
    connector_dispute_id: String
    /// Reason of dispute sent by connector
    connector_reason: String
    /// Reason code of dispute sent by connector
    connector_reason_code: String
    /// Evidence deadline of dispute sent by connector
    @timestampFormat("date-time")
    challenge_required_by: Timestamp
    /// Dispute created time sent by connector
    @timestampFormat("date-time")
    connector_created_at: Timestamp
    /// Dispute updated time sent by connector
    @timestampFormat("date-time")
    connector_updated_at: Timestamp
    /// Time at which dispute is received
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
}

structure DisputeStage {}

structure DisputeStatus {}

structure DokuBankTransferInstructions {
    @dataExamples([
        {
            json: "1707091200000"
        }
    ])
    @required
    expires_at: String
    @dataExamples([
        {
            json: "122385736258"
        }
    ])
    @required
    reference: String
    @required
    instructions_url: String
}

@mixin
structure DokuBillingDetails {
    /// The billing first name for Doku
    @dataExamples([
        {
            json: "Jane"
        }
    ])
    first_name: String
    /// The billing second name for Doku
    @dataExamples([
        {
            json: "Doe"
        }
    ])
    last_name: String
    /// The Email ID for Doku billing
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
}

structure DropDown {
    @required
    options: FieldTypeOneOfAlt33DropDownOptions
}

structure Eft {
    /// The preferred eft provider
    @dataExamples([
        {
            json: "ozow"
        }
    ])
    @required
    provider: String
}

structure ElementSizeOneOfAlt0 {
    @required
    Variants: SizeVariants
}

structure ElementSizeOneOfAlt1 {
    @range(
        min: 0
    )
    @required
    Percentage: Integer
}

structure ElementSizeOneOfAlt2 {
    @range(
        min: 0
    )
    @required
    Pixels: Integer
}

structure EliminationRoutingConfig {
    params: EliminationRoutingConfigParams
    elimination_analyser_config: EliminationAnalyserConfig
    @required
    decision_engine_configs: DecisionEngineEliminationData
}

/// Polish Elixir payment system
structure Elixir {
    /// Polish account number (26 digits)
    @dataExamples([
        {
            json: "12345678901234567890123456"
        }
    ])
    @required
    account_number: String
    /// Polish IBAN (28 chars)
    @dataExamples([
        {
            json: "PL27114020040000300201355387"
        }
    ])
    @required
    iban: String
    /// Account holder name
    @required
    name: String
    connector_recipient_id: String
}

structure EnableDisableKVForAMerchantAccount200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ToggleKVResponse
}

@error("client")
@httpError(400)
structure EnableDisableKVForAMerchantAccount400 {}

@error("client")
@httpError(404)
structure EnableDisableKVForAMerchantAccount404 {}

structure EnableDisableKVForAMerchantAccountInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: ToggleKVRequest
}

/// Object for EnabledPaymentMethod
structure EnabledPaymentMethod {
    @required
    payment_method: PaymentMethod
    @required
    payment_method_types: EnabledPaymentMethodPaymentMethodTypes
}

structure EncodedData with [MerchantConnectorDetails] {}

/// ephemeral_key for the customer_id mentioned
@mixin
structure EphemeralKeyCreateResponse {
    /// customer_id to which this ephemeral key belongs to
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    customer_id: String
    /// time at which this ephemeral key was created
    @required
    created_at: Long
    /// time at which this ephemeral key would expire
    @required
    expires: Long
    /// ephemeral key
    @required
    secret: String
}

structure Eps {
    billing_details: BankRedirectDataOneOfAlt3EpsBillingDetails
    @required
    bank_name: BankNames
    @required
    country: CountryAlpha2
}

structure Error with [RelayError] {}

/// The constraints to apply when filtering events.
structure EventListConstraints {
    /// Filter events created after the specified time.
    @timestampFormat("date-time")
    created_after: Timestamp
    /// Filter events created before the specified time.
    @timestampFormat("date-time")
    created_before: Timestamp
    /// Include at most the specified number of events.
    @range(
        min: 0
    )
    limit: Integer
    /// Include events after the specified offset.
    @range(
        min: 0
    )
    offset: Integer
    /// Filter all events associated with the specified object identifier (Payment Intent ID,
    /// Refund ID, etc.)
    object_id: String
    /// Filter all events associated with the specified business profile ID.
    profile_id: String
    event_classes: EventClasses
    event_types: EventTypes
    /// Filter all events by `is_overall_delivery_successful` field of the event.
    is_delivered: Boolean
}

structure EventListItemResponse with [EventListItemResponseMixin] {}

/// The response body for each item when listing events.
@mixin
structure EventListItemResponseMixin {
    /// The identifier for the Event.
    @dataExamples([
        {
            json: "evt_018e31720d1b7a2b82677d3032cab959"
        }
    ])
    @length(
        max: 64
    )
    @required
    event_id: String
    /// The identifier for the Merchant Account.
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// The identifier for the Business Profile.
    @dataExamples([
        {
            json: "SqB0zwDGR5wHppWf0bx7GKr1f2"
        }
    ])
    @length(
        max: 64
    )
    @required
    profile_id: String
    /// The identifier for the object (Payment Intent ID, Refund ID, etc.)
    @dataExamples([
        {
            json: "QHrfd5LUDdZaKtAjdJmMu0dMa1"
        }
    ])
    @length(
        max: 64
    )
    @required
    object_id: String
    @required
    event_type: EventType
    @required
    event_class: EventClass
    /// Indicates whether the webhook was ultimately delivered or not.
    is_delivery_successful: Boolean
    /// The identifier for the initial delivery attempt. This will be the same as `event_id` for
    /// the initial delivery attempt.
    @dataExamples([
        {
            json: "evt_018e31720d1b7a2b82677d3032cab959"
        }
    ])
    @length(
        max: 64
    )
    @required
    initial_attempt_id: String
    /// Time at which the event was created.
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    created: Timestamp
}

/// The response body for retrieving an event.
structure EventRetrieveResponse with [EventListItemResponseMixin] {
    @required
    request: OutgoingWebhookRequestContent
    @required
    response: OutgoingWebhookResponseContent
    delivery_attempt: DeliveryAttempt
}

structure Execute3DSDecisionRule200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ThreeDsDecisionRuleExecuteResponse
}

@error("client")
@httpError(400)
structure Execute3DSDecisionRule400 {}

structure Execute3DSDecisionRuleInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: ThreeDsDecisionRuleExecuteRequest
}

structure Expiration {}

structure ExtendedCardInfo {
    /// The card number
    @dataExamples([
        {
            json: "4242424242424242"
        }
    ])
    @required
    card_number: String
    /// The card's expiry month
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_month: String
    /// The card's expiry year
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_year: String
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
    /// The CVC number for the card
    @dataExamples([
        {
            json: "242"
        }
    ])
    @required
    card_cvc: String
    /// The name of the issuer of card
    @dataExamples([
        {
            json: "chase"
        }
    ])
    card_issuer: String
    card_network: ExtendedCardInfoCardNetwork
    @dataExamples([
        {
            json: "CREDIT"
        }
    ])
    card_type: String
    @dataExamples([
        {
            json: "INDIA"
        }
    ])
    card_issuing_country: String
    @dataExamples([
        {
            json: "JP_AMEX"
        }
    ])
    bank_code: String
}

structure ExtendedCardInfoCardNetwork {}

structure ExtendedCardInfoResponse {
    @required
    payload: String
}

/// Details of external authentication
@mixin
structure ExternalAuthenticationDetailsResponse {
    authentication_flow: AuthenticationFlow
    /// Electronic Commerce Indicator (eci)
    electronic_commerce_indicator: String
    @required
    status: AuthenticationStatus
    /// DS Transaction ID
    ds_transaction_id: String
    /// Message Version
    version: String
    /// Error Code
    error_code: String
    /// Error Message
    error_message: String
}

/// UK Faster Payments (instant transfers)
structure FasterPayments {
    /// 8-digit UK account number
    @required
    account_number: String
    /// 6-digit UK sort code
    @required
    sort_code: String
    /// Account holder name
    @required
    name: String
    connector_recipient_id: String
}

structure FeatureMatrixListResponse {
    /// The number of connectors included in the response
    @range(
        min: 0
    )
    @required
    connector_count: Integer
    @required
    connectors: FeatureMatrixListResponseConnectors
}

structure FeatureMatrixRequest {
    connectors: FeatureMatrixRequestConnectors
}

/// additional data that might be required by hyperswitch
@mixin
structure FeatureMetadata {
    redirect_response: RedirectResponse
    search_tags: SearchTags
    apple_pay_recurring_details: ApplePayRecurringDetails
}

structure FieldTypeOneOfAlt10 {
    @required
    user_currency: UserCurrency
}

structure FieldTypeOneOfAlt18 {
    @required
    user_address_country: UserAddressCountry
}

structure FieldTypeOneOfAlt25 {
    @required
    user_shipping_address_country: UserShippingAddressCountry
}

structure FieldTypeOneOfAlt33 {
    @required
    drop_down: DropDown
}

structure FieldTypeOneOfAlt36 {
    @required
    language_preference: LanguagePreference
}

structure FieldTypeOneOfAlt9 {
    @required
    user_country: UserCountry
}

structure FilterPayoutsUsingSpecificConstraints200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutListResponse
}

@error("client")
@httpError(404)
structure FilterPayoutsUsingSpecificConstraints404 {}

structure FilterPayoutsUsingSpecificConstraintsInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutListFilterConstraints
}

structure FlatAmount {}

structure Flow {}

/// Details of FrmConfigs are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
structure FrmConfigs {
    @required
    gateway: ConnectorType
    @required
    payment_methods: FrmConfigsPaymentMethods
}

/// frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its None
@mixin
structure FrmMessage {
    @required
    frm_name: String
    frm_transaction_id: String
    frm_transaction_type: String
    frm_status: String
    frm_score: Integer
    frm_reason: FrmReason
    frm_error: String
}

/// Details of FrmPaymentMethod are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
structure FrmPaymentMethod {
    @required
    payment_method: PaymentMethod
    payment_method_types: FrmPaymentMethodPaymentMethodTypes
    flow: Flow
}

/// Details of FrmPaymentMethodType are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table
structure FrmPaymentMethodType {
    @required
    payment_method_type: PaymentMethodType
    @required
    card_networks: CardNetwork
    @required
    flow: FrmPreferredFlowTypes
    @required
    action: FrmAction
}

structure FrmRoutingAlgorithm {}

structure FulfillAPayout200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCreateResponse
}

@error("client")
@httpError(400)
structure FulfillAPayout400 {}

structure FulfillAPayoutInput {
    /// The identifier for payout
    @httpLabel
    @required
    payout_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutFulfillRequest
}

/// Object for GenericLinkUiConfig
@mixin
structure GenericLinkUiConfig {
    /// Merchant's display logo
    @dataExamples([
        {
            json: "https://hyperswitch.io/favicon.ico"
        }
    ])
    @length(
        max: 255
    )
    logo: String
    /// Custom merchant name for the link
    @dataExamples([
        {
            json: "Hyperswitch"
        }
    ])
    @length(
        max: 255
    )
    merchant_name: String
    /// Primary color to be used in the form represented in hex format
    @dataExamples([
        {
            json: "#4285F4"
        }
    ])
    @length(
        max: 255
    )
    theme: String
}

structure GiftCardAdditionalDataOneOfAlt0 {
    @required
    givex: GivexGiftCardAdditionalData
}

structure GiftCardAdditionalDataOneOfAlt1 {
    @required
    pay_safe_card: Document
}

structure GiftCardDataOneOfAlt0 {
    @required
    givex: GiftCardDetails
}

structure GiftCardDataOneOfAlt1 {
    @required
    pay_safe_card: Document
}

structure GiftCardDetails {
    /// The gift card number
    @required
    number: String
    /// The card verification code.
    @required
    cvc: String
}

structure GiftCardResponse {}

structure Giropay {
    billing_details: BankRedirectDataOneOfAlt4GiropayBillingDetails
    /// Bank account bic code
    bank_account_bic: String
    /// Bank account iban
    bank_account_iban: String
    @required
    country: CountryAlpha2
}

structure GiropayBankRedirectAdditionalData {
    /// Masked bank account bic code
    bic: String
    /// Partially masked international bank account number (iban) for SEPA
    iban: String
    country: GiropayBankRedirectAdditionalDataCountry
}

structure GiropayBankRedirectAdditionalDataCountry {}

structure GivexGiftCardAdditionalData {
    /// Last 4 digits of the gift card number
    @dataExamples([
        {
            json: "4242"
        }
    ])
    @required
    last4: String
}

@mixin
structure GooglePayAssuranceDetails {
    /// indicates that Cardholder possession validation has been performed
    @required
    card_holder_authenticated: Boolean
    /// indicates that identification and verifications (ID&V) was performed
    @required
    account_verified: Boolean
}

structure GooglePayPaymentMethodInfo {
    /// The name of the card network
    @required
    card_network: String
    /// The details of the card
    @required
    card_details: String
    assurance_details: AssuranceDetails
}

structure GooglePaySessionResponse {
    @required
    merchant_info: GpayMerchantInfo
    /// Is shipping address required
    @required
    shipping_address_required: Boolean
    /// Is email required
    @required
    email_required: Boolean
    @required
    shipping_address_parameters: GpayShippingAddressParameters
    @required
    allowed_payment_methods: AllowedPaymentMethods
    @required
    transaction_info: GpayTransactionInfo
    /// Identifier for the delayed session response
    @required
    delayed_session_token: Boolean
    /// The name of the connector
    @required
    connector: String
    @required
    sdk_next_action: SdkNextAction
    secrets: Secrets
}

structure GooglePayThirdPartySdk {
    /// Identifier for the delayed session response
    @required
    delayed_session_token: Boolean
    /// The name of the connector
    @required
    connector: String
    @required
    sdk_next_action: SdkNextAction
}

structure GooglePayWalletData {
    /// The type of payment method
    @required
    type: String
    /// User-facing message to describe the payment method that funds this transaction.
    @required
    description: String
    @required
    info: GooglePayPaymentMethodInfo
    @required
    tokenization_data: GpayTokenizationData
}

structure GpayAllowedMethodsParameters {
    @required
    allowed_auth_methods: AllowedAuthMethods
    @required
    allowed_card_networks: AllowedCardNetworks
    /// Is billing address required
    billing_address_required: Boolean
    billing_address_parameters: BillingAddressParameters
    /// Whether assurance details are required
    assurance_details_required: Boolean
}

structure GpayAllowedPaymentMethods {
    /// The type of payment method
    @required
    type: String
    @required
    parameters: GpayAllowedMethodsParameters
    @required
    tokenization_specification: GpayTokenizationSpecification
}

@mixin
structure GpayBillingAddressParameters {
    /// Is billing phone number required
    @required
    phone_number_required: Boolean
    @required
    format: GpayBillingAddressFormat
}

structure GpayMerchantInfo {
    /// The merchant Identifier that needs to be passed while invoking Gpay SDK
    merchant_id: String
    /// The name of the merchant that needs to be displayed on Gpay PopUp
    @required
    merchant_name: String
}

structure GpayShippingAddressParameters {
    /// Is shipping phone number required
    @required
    phone_number_required: Boolean
}

structure GpayTokenizationData {
    /// The type of the token
    @required
    type: String
    /// Token generated for the wallet
    @required
    token: String
}

structure GpayTokenizationSpecification {
    /// The token specification type(ex: PAYMENT_GATEWAY)
    @required
    type: String
    @required
    parameters: GpayTokenParameters
}

structure GpayTokenParameters {
    /// The name of the connector
    gateway: String
    /// The merchant ID registered in the connector associated
    gateway_merchant_id: String
    @jsonName("stripe:version")
    stripeversion: String
    @jsonName("stripe:publishableKey")
    stripepublishableKey: String
    /// The protocol version for encryption
    protocol_version: String
    /// The public key provided by the merchant
    public_key: String
}

structure GpayTransactionInfo {
    @required
    country_code: CountryAlpha2
    @required
    currency_code: Currency
    /// The total price status (ex: 'FINAL')
    @required
    total_price_status: String
    /// The total price
    @dataExamples([
        {
            json: "38.02"
        }
    ])
    @required
    total_price: String
}

structure GsmCreateRequest {
    @required
    connector: Connector
    /// The flow in which the code and message occurred for a connector
    @required
    flow: String
    /// The sub_flow in which the code and message occurred  for a connector
    @required
    sub_flow: String
    /// code received from the connector
    @required
    code: String
    /// message received from the connector
    @required
    message: String
    /// status provided by the router
    @required
    status: String
    /// optional error provided by the router
    router_error: String
    @required
    decision: GsmDecision
    /// indicates if step_up retry is possible
    @required
    step_up_possible: Boolean
    /// error code unified across the connectors
    unified_code: String
    /// error message unified across the connectors
    unified_message: String
    error_category: GsmCreateRequestErrorCategory
    /// indicates if retry with pan is possible
    @required
    clear_pan_possible: Boolean
}

structure GsmCreateRequestErrorCategory {}

structure GsmDeleteRequest {
    /// The connector through which payment has gone through
    @required
    connector: String
    /// The flow in which the code and message occurred for a connector
    @required
    flow: String
    /// The sub_flow in which the code and message occurred  for a connector
    @required
    sub_flow: String
    /// code received from the connector
    @required
    code: String
    /// message received from the connector
    @required
    message: String
}

structure GsmDeleteResponse {
    @required
    gsm_rule_delete: Boolean
    /// The connector through which payment has gone through
    @required
    connector: String
    /// The flow in which the code and message occurred for a connector
    @required
    flow: String
    /// The sub_flow in which the code and message occurred  for a connector
    @required
    sub_flow: String
    /// code received from the connector
    @required
    code: String
}

structure GsmResponse {
    /// The connector through which payment has gone through
    @required
    connector: String
    /// The flow in which the code and message occurred for a connector
    @required
    flow: String
    /// The sub_flow in which the code and message occurred  for a connector
    @required
    sub_flow: String
    /// code received from the connector
    @required
    code: String
    /// message received from the connector
    @required
    message: String
    /// status provided by the router
    @required
    status: String
    /// optional error provided by the router
    router_error: String
    /// decision to be taken for auto retries flow
    @required
    decision: String
    /// indicates if step_up retry is possible
    @required
    step_up_possible: Boolean
    /// error code unified across the connectors
    unified_code: String
    /// error message unified across the connectors
    unified_message: String
    error_category: GsmResponseErrorCategory
    /// indicates if retry with pan is possible
    @required
    clear_pan_possible: Boolean
}

structure GsmResponseErrorCategory {}

structure GsmRetrieveRequest {
    @required
    connector: Connector
    /// The flow in which the code and message occurred for a connector
    @required
    flow: String
    /// The sub_flow in which the code and message occurred  for a connector
    @required
    sub_flow: String
    /// code received from the connector
    @required
    code: String
    /// message received from the connector
    @required
    message: String
}

structure GsmUpdateRequest {
    /// The connector through which payment has gone through
    @required
    connector: String
    /// The flow in which the code and message occurred for a connector
    @required
    flow: String
    /// The sub_flow in which the code and message occurred  for a connector
    @required
    sub_flow: String
    /// code received from the connector
    @required
    code: String
    /// message received from the connector
    @required
    message: String
    /// status provided by the router
    status: String
    /// optional error provided by the router
    router_error: String
    decision: Decision
    /// indicates if step_up retry is possible
    step_up_possible: Boolean
    /// error code unified across the connectors
    unified_code: String
    /// error message unified across the connectors
    unified_message: String
    error_category: GsmUpdateRequestErrorCategory
    /// indicates if retry with pan is possible
    clear_pan_possible: Boolean
}

structure GsmUpdateRequestErrorCategory {}

/// IBAN-based account for international transfers
structure Iban {
    /// International Bank Account Number (up to 34 characters)
    @required
    iban: String
    /// Account holder name
    @required
    name: String
    connector_recipient_id: String
}

structure Ideal {
    billing_details: BankRedirectDataOneOfAlt5IdealBillingDetails
    @required
    bank_name: BankNames
    @required
    country: CountryAlpha2
}

structure IframeDataOneOfAlt0 {
    /// ThreeDS method url
    @required
    three_ds_method_url: String
    /// Whether ThreeDS method data submission is required
    @required
    three_ds_method_data_submission: Boolean
    /// ThreeDS method data
    three_ds_method_data: String
    /// ThreeDS Server ID
    @required
    directory_server_id: String
    /// ThreeDS Protocol version
    message_version: String
}

/// Represents an IF statement with conditions and optional nested IF statements
/// 
/// ```text
/// payment.method = card {
/// payment.method.cardtype = (credit, debit) {
/// payment.method.network = (amex, rupay, diners)
/// }
/// }
/// ```
structure IfStatement {
    @required
    condition: Condition
    nested: Nested
}

structure IncrementalAuthorizationResponse {
    /// The unique identifier of authorization
    @required
    authorization_id: String
    /// Amount the authorization has been made for
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    @required
    status: AuthorizationStatus
    /// Error code sent by the connector for authorization
    error_code: String
    /// Error message sent by the connector for authorization
    error_message: String
    @required
    previously_authorized_amount: MinorUnit
}

structure IncrementAuthorizedAmountForAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsResponse
}

@error("client")
@httpError(400)
structure IncrementAuthorizedAmountForAPayment400 {}

structure IncrementAuthorizedAmountForAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsIncrementalAuthorizationRequest
}

structure IndomaretVoucherData {
    /// The billing first name for Alfamart
    @dataExamples([
        {
            json: "Jane"
        }
    ])
    first_name: String
    /// The billing second name for Alfamart
    @dataExamples([
        {
            json: "Doe"
        }
    ])
    last_name: String
    /// The Email ID for Alfamart
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
}

structure InitiateExternalAuthenticationForAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsExternalAuthenticationResponse
}

@error("client")
@httpError(400)
structure InitiateExternalAuthenticationForAPayment400 {}

structure InitiateExternalAuthenticationForAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsExternalAuthenticationRequest
}

structure Interac {
    country: BankRedirectDataOneOfAlt6InteracCountry
    @dataExamples([
        {
            json: "john.doe@example.com"
        }
    ])
    email: String
}

structure Issuer with [IssuerData] {}

/// Represents data about the issuer used in the 3DS decision rule.
@mixin
structure IssuerData {
    /// The name of the issuer.
    name: String
    @required
    country: Country
}

structure JCSVoucherData {
    /// The billing first name for Japanese convenience stores
    @dataExamples([
        {
            json: "Jane"
        }
    ])
    first_name: String
    /// The billing second name Japanese convenience stores
    @dataExamples([
        {
            json: "Doe"
        }
    ])
    last_name: String
    /// The Email ID for Japanese convenience stores
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
    /// The telephone number for Japanese convenience stores
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    phone_number: String
}

/// For KlarnaRedirect as PayLater Option
structure KlarnaRedirect {
    /// The billing email
    billing_email: String
    billing_country: BillingCountry
}

@mixin
structure KlarnaSdkPaymentMethodResponse {
    payment_type: String
}

@mixin
structure KlarnaSessionTokenResponse {
    /// The session token for Klarna
    @required
    session_token: String
    /// The identifier for the session
    @required
    session_id: String
}

structure LabelInformation {
    @required
    label: String
    @range(
        min: 0
    )
    @required
    target_count: Long
    @range(
        min: 0
    )
    @required
    target_time: Long
    @required
    mca_id: String
}

structure LanguagePreference {
    @required
    options: FieldTypeOneOfAlt36LanguagePreferenceOptions
}

structure ListAllAPIKeysAssociatedWithAMerchantAccount200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListAllAPIKeysAssociatedWithAMerchantAccount200Body
}

structure ListAllAPIKeysAssociatedWithAMerchantAccountInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    merchant_id: String
    /// The maximum number of API Keys to include in the response
    @httpQuery("limit")
    limit: Long
    /// The number of API Keys to skip when retrieving the list of API keys.
    @httpQuery("skip")
    skip: Long
}

structure ListAllCustomersForAMerchant200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListAllCustomersForAMerchant200Body
}

@error("client")
@httpError(400)
structure ListAllCustomersForAMerchant400 {}

structure ListAllCustomersForAMerchantInput {
    /// Offset for pagination
    @httpQuery("offset")
    @range(
        min: 0
    )
    offset: Integer
    /// Limit for pagination
    @httpQuery("limit")
    @range(
        min: 0
    )
    limit: Integer
}

structure ListAllDeliveryAttemptsForAnEvent200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListAllDeliveryAttemptsForAnEvent200Body
}

structure ListAllDeliveryAttemptsForAnEventInput {
    /// The unique identifier for the Merchant Account.
    @httpLabel
    @required
    merchant_id: String
    /// The unique identifier for the Event
    @httpLabel
    @required
    event_id: String
}

structure ListAllEventsAssociatedWithAMerchantAccountOrProfile200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: TotalEventsResponse
}

structure ListAllEventsAssociatedWithAMerchantAccountOrProfileInput {
    /// The unique identifier for the Merchant Account.
    @httpLabel
    @required
    merchant_id: String
    /// The constraints that can be applied when listing Events.
    @httpPayload
    @required
    @contentType("application/json")
    body: EventListConstraints
}

structure ListAllEventsAssociatedWithAProfile200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: TotalEventsResponse
}

structure ListAllEventsAssociatedWithAProfileInput {
    /// The constraints that can be applied when listing Events.
    @httpPayload
    @required
    @contentType("application/json")
    body: EventListConstraints
}

structure ListAllMerchantConnectors200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListAllMerchantConnectors200Body
}

@error("client")
@httpError(401)
structure ListAllMerchantConnectors401 {}

@error("client")
@httpError(404)
structure ListAllMerchantConnectors404 {}

structure ListAllMerchantConnectorsInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
}

structure ListAllPaymentMethodsForACustomer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerPaymentMethodsListResponse
}

@error("client")
@httpError(400)
structure ListAllPaymentMethodsForACustomer400 {}

@error("client")
@httpError(404)
structure ListAllPaymentMethodsForACustomer404 {}

structure ListAllPaymentMethodsForACustomerInput {
    /// The unique identifier for the customer account
    @httpLabel
    @required
    customer_id: String
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @httpQuery("client_secret")
    client_secret: String
    /// The two-letter ISO currency code
    @httpQuery("accepted_countries")
    accepted_countries: ListAllPaymentMethodsForACustomerInputAcceptedCountries
    /// The three-letter ISO currency code
    @httpQuery("accepted_currencies")
    accepted_currencies: ListAllPaymentMethodsForACustomerInputAcceptedCurrencies
    /// The amount accepted for processing by the particular payment method.
    @httpQuery("amount")
    amount: Long
    /// Indicates whether the payment method is eligible for recurring payments
    @httpQuery("recurring_enabled")
    recurring_enabled: Boolean
    /// Indicates whether the payment method is eligible for installment payments
    @httpQuery("installment_payment_enabled")
    installment_payment_enabled: Boolean
    /// Indicates the limit of last used payment methods
    @httpQuery("limit")
    limit: Long
    /// Indicates whether the payment method is eligible for card netwotks
    @httpQuery("card_networks")
    card_networks: ListAllPaymentMethodsForACustomerInputCardNetworks
}

structure ListAllPaymentMethodsForAMerchant200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodListResponse
}

@error("client")
@httpError(400)
structure ListAllPaymentMethodsForAMerchant400 {}

@error("client")
@httpError(404)
structure ListAllPaymentMethodsForAMerchant404 {}

structure ListAllPaymentMethodsForAMerchantInput {
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @httpQuery("client_secret")
    client_secret: String
    /// The two-letter ISO currency code
    @httpQuery("accepted_countries")
    accepted_countries: ListAllPaymentMethodsForAMerchantInputAcceptedCountries
    /// The three-letter ISO currency code
    @httpQuery("accepted_currencies")
    accepted_currencies: ListAllPaymentMethodsForAMerchantInputAcceptedCurrencies
    /// The amount accepted for processing by the particular payment method.
    @httpQuery("amount")
    amount: Long
    /// Indicates whether the payment method is eligible for recurring payments
    @httpQuery("recurring_enabled")
    recurring_enabled: Boolean
    /// Indicates whether the payment method is eligible for installment payments
    @httpQuery("installment_payment_enabled")
    installment_payment_enabled: Boolean
    /// Indicates the limit of last used payment methods
    @httpQuery("limit")
    limit: Long
    /// Indicates whether the payment method is eligible for card netwotks
    @httpQuery("card_networks")
    card_networks: ListAllPaymentMethodsForAMerchantInputCardNetworks
}

structure ListAllPayments200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListAllPayments200Body
}

@error("client")
@httpError(404)
structure ListAllPayments404 {}

structure ListAllPaymentsInput {
    /// The identifier for the customer
    @httpQuery("customer_id")
    customer_id: String
    /// A cursor for use in pagination, fetch the next list after some object
    @httpQuery("starting_after")
    starting_after: String
    /// A cursor for use in pagination, fetch the previous list before some object
    @httpQuery("ending_before")
    ending_before: String
    /// Limit on the number of objects to return
    @httpQuery("limit")
    limit: Long
    /// The time at which payment is created
    @httpQuery("created")
    @timestampFormat("date-time")
    created: Timestamp
    /// Time less than the payment created time
    @httpQuery("created_lt")
    @timestampFormat("date-time")
    created_lt: Timestamp
    /// Time greater than the payment created time
    @httpQuery("created_gt")
    @timestampFormat("date-time")
    created_gt: Timestamp
    /// Time less than or equals to the payment created time
    @httpQuery("created_lte")
    @timestampFormat("date-time")
    created_lte: Timestamp
    /// Time greater than or equals to the payment created time
    @httpQuery("created_gte")
    @timestampFormat("date-time")
    created_gte: Timestamp
}

structure ListAllRefunds200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundListResponse
}

structure ListAllRefundsInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundListRequest
}

structure ListAvailablePayoutFilters200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutListFilters
}

structure ListAvailablePayoutFiltersInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: TimeRange
}

structure ListBlockedFingerprintsOfAParticularKind200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: BlocklistResponse
}

@error("client")
@httpError(400)
structure ListBlockedFingerprintsOfAParticularKind400 {}

structure ListBlockedFingerprintsOfAParticularKindInput {
    /// Kind of the fingerprint list requested
    @httpQuery("data_kind")
    @required
    data_kind: BlocklistDataKind
}

structure ListBlocklistQuery {
    @required
    data_kind: BlocklistDataKind
    @range(
        min: 0
    )
    limit: Integer
    @range(
        min: 0
    )
    offset: Integer
}

structure ListCustomerPaymentMethodsViaClientSecret200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerPaymentMethodsListResponse
}

@error("client")
@httpError(400)
structure ListCustomerPaymentMethodsViaClientSecret400 {}

@error("client")
@httpError(404)
structure ListCustomerPaymentMethodsViaClientSecret404 {}

structure ListCustomerPaymentMethodsViaClientSecretInput {
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @httpQuery("client_secret")
    client_secret: String
    /// The two-letter ISO currency code
    @httpQuery("accepted_countries")
    accepted_countries: ListCustomerPaymentMethodsViaClientSecretInputAcceptedCountries
    /// The three-letter ISO currency code
    @httpQuery("accepted_currencies")
    accepted_currencies: ListCustomerPaymentMethodsViaClientSecretInputAcceptedCurrencies
    /// The amount accepted for processing by the particular payment method.
    @httpQuery("amount")
    amount: Long
    /// Indicates whether the payment method is eligible for recurring payments
    @httpQuery("recurring_enabled")
    recurring_enabled: Boolean
    /// Indicates whether the payment method is eligible for installment payments
    @httpQuery("installment_payment_enabled")
    installment_payment_enabled: Boolean
    /// Indicates the limit of last used payment methods
    @httpQuery("limit")
    limit: Long
    /// Indicates whether the payment method is eligible for card netwotks
    @httpQuery("card_networks")
    card_networks: ListCustomerPaymentMethodsViaClientSecretInputCardNetworks
}

structure ListDisputes200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListDisputes200Body
}

@error("client")
@httpError(401)
structure ListDisputes401 {}

structure ListDisputesInput {
    /// The maximum number of Dispute Objects to include in the response
    @httpQuery("limit")
    limit: Long
    /// The status of dispute
    @httpQuery("dispute_status")
    dispute_status: DisputeStatus
    /// The stage of dispute
    @httpQuery("dispute_stage")
    dispute_stage: DisputeStage
    /// The reason for dispute
    @httpQuery("reason")
    reason: String
    /// The connector linked to dispute
    @httpQuery("connector")
    connector: String
    /// The time at which dispute is received
    @httpQuery("received_time")
    @timestampFormat("date-time")
    received_time: Timestamp
    /// Time less than the dispute received time
    @httpQuery("received_time.lt")
    @timestampFormat("date-time")
    received_timelt: Timestamp
    /// Time greater than the dispute received time
    @httpQuery("received_time.gt")
    @timestampFormat("date-time")
    received_timegt: Timestamp
    /// Time less than or equals to the dispute received time
    @httpQuery("received_time.lte")
    @timestampFormat("date-time")
    received_timelte: Timestamp
    /// Time greater than or equals to the dispute received time
    @httpQuery("received_time.gte")
    @timestampFormat("date-time")
    received_timegte: Timestamp
}

structure ListMandatesForACustomer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListMandatesForACustomer200Body
}

@error("client")
@httpError(400)
structure ListMandatesForACustomer400 {}

structure ListMandatesForACustomerInput {
    /// The unique identifier for the customer
    @httpLabel
    @required
    customer_id: String
}

structure ListPayoutsUsingGenericConstraints200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutListResponse
}

@error("client")
@httpError(404)
structure ListPayoutsUsingGenericConstraints404 {}

structure ListPayoutsUsingGenericConstraintsInput {
    /// The identifier for customer
    @httpQuery("customer_id")
    @required
    customer_id: String
    /// A cursor for use in pagination, fetch the next list after some object
    @httpQuery("starting_after")
    @required
    starting_after: String
    /// A cursor for use in pagination, fetch the previous list before some object
    @httpQuery("ending_before")
    @required
    ending_before: String
    /// limit on the number of objects to return
    @httpQuery("limit")
    @required
    limit: String
    /// The time at which payout is created
    @httpQuery("created")
    @required
    created: String
    /// The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).
    @httpQuery("time_range")
    @required
    time_range: String
}

structure ListProfiles200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ListProfiles200Body
}

structure ListProfilesInput {
    /// Merchant Identifier
    @httpLabel
    @required
    account_id: String
}

structure ListRoutingConfigs200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingKind
}

@error("client")
@httpError(404)
structure ListRoutingConfigs404 {}

@error("server")
@httpError(500)
structure ListRoutingConfigs500 {}

structure ListRoutingConfigsInput {
    /// The number of records to be returned
    @httpQuery("limit")
    @range(
        min: 0
    )
    limit: Integer
    /// The record offset from which to start gathering of results
    @httpQuery("offset")
    @range(
        min: 0
    )
    offset: Integer
    /// The unique identifier for a merchant profile
    @httpQuery("profile_id")
    profile_id: String
}

structure LocalBankTransfer {
    bank_code: String
}

structure LocalBankTransferAdditionalData {
    /// Partially masked bank code
    @dataExamples([
        {
            json: "**** OA2312"
        }
    ])
    bank_code: String
}

structure MandateAmountData with [MandateAmountDataMixin] {}

@mixin
structure MandateAmountDataMixin {
    /// The maximum amount to be debited for the mandate transaction
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    @required
    currency: Currency
    /// Specifying start date of the mandate
    @dataExamples([
        {
            json: "2022-09-10T00:00Z"
        }
    ])
    @timestampFormat("date-time")
    start_date: Timestamp
    /// Specifying end date of the mandate
    @dataExamples([
        {
            json: "2023-09-10T23:59:59Z"
        }
    ])
    @timestampFormat("date-time")
    end_date: Timestamp
    /// Additional details required by mandate
    metadata: Document
}

@mixin
structure MandateCardDetails {
    /// The last 4 digits of card
    last4_digits: String
    /// The expiry month of card
    card_exp_month: String
    /// The expiry year of card
    card_exp_year: String
    /// The card holder name
    card_holder_name: String
    /// The token from card locker
    card_token: String
    /// The card scheme network for the particular card
    scheme: String
    /// The country code in in which the card was issued
    issuer_country: String
    /// A unique identifier alias to identify a particular card
    card_fingerprint: String
    /// The first 6 digits of card
    card_isin: String
    /// The bank that issued the card
    card_issuer: String
    card_network: MandateCardDetailsCardNetwork
    /// The type of the payment card
    card_type: String
    /// The nick_name of the card holder
    nick_name: String
}

structure MandateCardDetailsCardNetwork {}

/// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
@mixin
structure MandateData {
    /// A way to update the mandate's payment method details
    update_mandate_id: String
    customer_acceptance: MandateDataCustomerAcceptance
    mandate_type: MandateType
}

structure MandateDataCustomerAcceptance with [CustomerAcceptance] {}

structure MandateResponse {
    /// The identifier for mandate
    @required
    mandate_id: String
    @required
    status: MandateStatus
    /// The identifier for payment method
    @required
    payment_method_id: String
    /// The payment method
    @required
    payment_method: String
    /// The payment method type
    payment_method_type: String
    card: MandateResponseCard
    customer_acceptance: MandateResponseCustomerAcceptance
}

structure MandateResponseCard with [MandateCardDetails] {}

structure MandateResponseCustomerAcceptance with [CustomerAcceptance] {}

structure MandateRevokedResponse {
    /// The identifier for mandate
    @required
    mandate_id: String
    @required
    status: MandateStatus
    /// If there was an error while calling the connectors the code is received here
    @dataExamples([
        {
            json: "E0001"
        }
    ])
    error_code: String
    /// If there was an error while calling the connector the error message is received here
    @dataExamples([
        {
            json: "Failed while verifying the card"
        }
    ])
    error_message: String
}

structure MandateTypeOneOfAlt0 {
    @required
    single_use: MandateAmountData
}

structure MandateTypeOneOfAlt1 {
    @required
    multi_use: MultiUse
}

structure MandiriVaBankTransfer {
    billing_details: BankTransferDataOneOfAlt10MandiriVaBankTransferBillingDetails
}

structure ManuallyRetryTheDeliveryOfAnEvent200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: EventRetrieveResponse
}

structure ManuallyRetryTheDeliveryOfAnEventInput {
    /// The unique identifier for the Merchant Account.
    @httpLabel
    @required
    merchant_id: String
    /// The unique identifier for the Event
    @httpLabel
    @required
    event_id: String
}

@mixin
structure MaskedBankDetails {
    @required
    mask: String
}

structure MaximumAmount {}

structure MbWayRedirection {
    /// Telephone number of the shopper. Should be Portuguese phone number.
    @required
    telephone_number: String
}

structure MerchantAccountCreate {
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    merchant_id: String
    /// Name of the Merchant Account
    @dataExamples([
        {
            json: "NewAge Retailer"
        }
    ])
    merchant_name: String
    merchant_details: MerchantAccountCreateMerchantDetails
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://www.example.com/success"
        }
    ])
    @length(
        max: 255
    )
    return_url: String
    webhook_details: MerchantAccountCreateWebhookDetails
    payout_routing_algorithm: MerchantAccountCreatePayoutRoutingAlgorithm
    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    sub_merchants_enabled: Boolean
    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    @dataExamples([
        {
            json: "xkkdf909012sdjki2dkh5sdf"
        }
    ])
    @length(
        max: 255
    )
    parent_merchant_id: String
    /// A boolean value to indicate if payment response hash needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    enable_payment_response_hash: Boolean
    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    payment_response_hash_key: String
    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled.
    @dataExamples([
        {
            json: true
        }
    ])
    redirect_to_merchant_with_http_post: Boolean
    /// Metadata is useful for storing additional, unstructured information on an object
    metadata: Document
    /// API key that will be used for client side API access. A publishable key has to be always paired with a `client_secret`.
    /// A `client_secret` can be obtained by creating a payment with `confirm` set to false
    @dataExamples([
        {
            json: "AH3423bkjbkjdsfbkj"
        }
    ])
    publishable_key: String
    /// An identifier for the vault used to store payment method information.
    @dataExamples([
        {
            json: "locker_abc123"
        }
    ])
    locker_id: String
    primary_business_details: MerchantAccountCreatePrimaryBusinessDetails
    /// The frm routing algorithm to be used for routing payments to desired FRM's
    frm_routing_algorithm: Document
    /// The id of the organization to which the merchant belongs to, if not passed an organization is created
    @dataExamples([
        {
            json: "org_q98uSGAYbjEwqs0mJwnz"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    organization_id: String
    pm_collect_link_config: MerchantAccountCreatePmCollectLinkConfig
    product_type: MerchantAccountCreateProductType
    merchant_account_type: MerchantAccountType
}

structure MerchantAccountCreateMerchantDetails with [MerchantDetails] {}

structure MerchantAccountCreatePayoutRoutingAlgorithm {}

structure MerchantAccountCreatePmCollectLinkConfig with [BusinessCollectLinkConfig] {}

structure MerchantAccountCreatePrimaryBusinessDetails with [PrimaryBusinessDetailsMixin] {}

structure MerchantAccountCreateProductType {}

structure MerchantAccountCreateWebhookDetails with [WebhookDetails] {}

structure MerchantAccountDataOneOfAlt0 {
    @required
    iban: Iban
}

structure MerchantAccountDataOneOfAlt1 {
    @required
    bacs: Bacs
}

structure MerchantAccountDataOneOfAlt2 {
    @required
    faster_payments: FasterPayments
}

structure MerchantAccountDataOneOfAlt3 {
    @required
    sepa: Sepa
}

structure MerchantAccountDataOneOfAlt4 {
    @required
    sepa_instant: SepaInstant
}

structure MerchantAccountDataOneOfAlt5 {
    @required
    elixir: Elixir
}

structure MerchantAccountDataOneOfAlt6 {
    @required
    bankgiro: Bankgiro
}

structure MerchantAccountDataOneOfAlt7 {
    @required
    plusgiro: Plusgiro
}

structure MerchantAccountDeleteResponse {
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    @required
    merchant_id: String
    /// If the connector is deleted or not
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    deleted: Boolean
}

structure MerchantAccountResponse {
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// Name of the Merchant Account
    @dataExamples([
        {
            json: "NewAge Retailer"
        }
    ])
    merchant_name: String
    /// The URL to redirect after completion of the payment
    @dataExamples([
        {
            json: "https://www.example.com/success"
        }
    ])
    @length(
        max: 255
    )
    return_url: String
    /// A boolean value to indicate if payment response hash needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    enable_payment_response_hash: Boolean
    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    @dataExamples([
        {
            json: "xkkdf909012sdjki2dkh5sdf"
        }
    ])
    @length(
        max: 255
    )
    payment_response_hash_key: String
    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    redirect_to_merchant_with_http_post: Boolean
    merchant_details: MerchantAccountResponseMerchantDetails
    webhook_details: MerchantAccountResponseWebhookDetails
    payout_routing_algorithm: MerchantAccountResponsePayoutRoutingAlgorithm
    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    sub_merchants_enabled: Boolean
    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    @dataExamples([
        {
            json: "xkkdf909012sdjki2dkh5sdf"
        }
    ])
    @length(
        max: 255
    )
    parent_merchant_id: String
    /// API key that will be used for server side API access
    @dataExamples([
        {
            json: "AH3423bkjbkjdsfbkj"
        }
    ])
    publishable_key: String
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// An identifier for the vault used to store payment method information.
    @dataExamples([
        {
            json: "locker_abc123"
        }
    ])
    locker_id: String
    @required
    primary_business_details: MerchantAccountResponsePrimaryBusinessDetails
    frm_routing_algorithm: FrmRoutingAlgorithm
    /// The organization id merchant is associated with
    @dataExamples([
        {
            json: "org_q98uSGAYbjEwqs0mJwnz"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    organization_id: String
    /// A boolean value to indicate if the merchant has recon service is enabled or not, by default value is false
    @required
    is_recon_enabled: Boolean
    /// The default profile that must be used for creating merchant accounts and payments
    @length(
        max: 64
    )
    default_profile: String
    @required
    recon_status: ReconStatus
    pm_collect_link_config: MerchantAccountResponsePmCollectLinkConfig
    product_type: MerchantAccountResponseProductType
    @required
    merchant_account_type: MerchantAccountType
}

structure MerchantAccountResponseMerchantDetails with [MerchantDetails] {}

structure MerchantAccountResponsePayoutRoutingAlgorithm {}

structure MerchantAccountResponsePmCollectLinkConfig with [BusinessCollectLinkConfig] {}

structure MerchantAccountResponseProductType {}

structure MerchantAccountResponseWebhookDetails with [WebhookDetails] {}

structure MerchantAccountUpdate {
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// Name of the Merchant Account
    @dataExamples([
        {
            json: "NewAge Retailer"
        }
    ])
    merchant_name: String
    merchant_details: MerchantAccountUpdateMerchantDetails
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://www.example.com/success"
        }
    ])
    @length(
        max: 255
    )
    return_url: String
    webhook_details: MerchantAccountUpdateWebhookDetails
    payout_routing_algorithm: MerchantAccountUpdatePayoutRoutingAlgorithm
    /// A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    sub_merchants_enabled: Boolean
    /// Refers to the Parent Merchant ID if the merchant being created is a sub-merchant
    @dataExamples([
        {
            json: "xkkdf909012sdjki2dkh5sdf"
        }
    ])
    @length(
        max: 255
    )
    parent_merchant_id: String
    /// A boolean value to indicate if payment response hash needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    enable_payment_response_hash: Boolean
    /// Refers to the hash key used for calculating the signature for webhooks and redirect response.
    payment_response_hash_key: String
    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    redirect_to_merchant_with_http_post: Boolean
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// API key that will be used for server side API access
    @dataExamples([
        {
            json: "AH3423bkjbkjdsfbkj"
        }
    ])
    publishable_key: String
    /// An identifier for the vault used to store payment method information.
    @dataExamples([
        {
            json: "locker_abc123"
        }
    ])
    locker_id: String
    primary_business_details: MerchantAccountUpdatePrimaryBusinessDetails
    /// The frm routing algorithm to be used for routing payments to desired FRM's
    frm_routing_algorithm: Document
    /// The default profile that must be used for creating merchant accounts and payments
    @length(
        max: 64
    )
    default_profile: String
    pm_collect_link_config: MerchantAccountUpdatePmCollectLinkConfig
}

structure MerchantAccountUpdateMerchantDetails with [MerchantDetails] {}

structure MerchantAccountUpdatePayoutRoutingAlgorithm {}

structure MerchantAccountUpdatePmCollectLinkConfig with [BusinessCollectLinkConfig] {}

structure MerchantAccountUpdateWebhookDetails with [WebhookDetails] {}

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
structure MerchantConnectorCreate {
    @required
    connector_type: ConnectorType
    @required
    connector_name: Connector
    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is `default`, connector label can be `stripe_default`
    @dataExamples([
        {
            json: "stripe_US_travel"
        }
    ])
    connector_label: String
    /// Identifier for the profile, if not provided default will be chosen from merchant account
    @length(
        max: 64
    )
    profile_id: String
    connector_account_details: MerchantConnectorCreateConnectorAccountDetails
    payment_methods_enabled: MerchantConnectorCreatePaymentMethodsEnabled
    connector_webhook_details: MerchantConnectorCreateConnectorWebhookDetails
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    test_mode: Boolean
    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    disabled: Boolean
    frm_configs: MerchantConnectorCreateFrmConfigs
    business_country: MerchantConnectorCreateBusinessCountry
    /// The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    business_label: String
    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    @dataExamples([
        {
            json: "chase"
        }
    ])
    business_sub_label: String
    /// Unique ID of the connector
    @dataExamples([
        {
            json: "mca_5apGeP94tMts6rg3U3kR"
        }
    ])
    merchant_connector_id: String
    pm_auth_config: Document
    status: MerchantConnectorCreateStatus
    additional_merchant_data: MerchantConnectorCreateAdditionalMerchantData
    connector_wallets_details: MerchantConnectorCreateConnectorWalletsDetails
}

structure MerchantConnectorCreateAdditionalMerchantData {}

structure MerchantConnectorCreateBusinessCountry {}

structure MerchantConnectorCreateConnectorAccountDetails with [MerchantConnectorDetails] {}

structure MerchantConnectorCreateConnectorWalletsDetails with [ConnectorWalletDetails] {}

structure MerchantConnectorCreateConnectorWebhookDetails with [MerchantConnectorWebhookDetails] {}

structure MerchantConnectorCreateStatus {}

structure MerchantConnectorDeleteResponse {
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    @required
    merchant_id: String
    /// Unique ID of the connector
    @dataExamples([
        {
            json: "mca_5apGeP94tMts6rg3U3kR"
        }
    ])
    @required
    merchant_connector_id: String
    /// If the connector is deleted or not
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    deleted: Boolean
}

@mixin
structure MerchantConnectorDetails {
    /// Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object.
    connector_account_details: Document
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
}

/// Merchant connector details used to make payments.
@mixin
structure MerchantConnectorDetailsWrap {
    /// Creds Identifier is to uniquely identify the credentials. Do not send any sensitive info, like encoded_data in this field. And do not send the string "null".
    @required
    creds_identifier: String
    encoded_data: EncodedData
}

structure MerchantConnectorListResponse {
    @required
    connector_type: ConnectorType
    @required
    connector_name: Connector
    /// A unique label to identify the connector account created under a profile
    @dataExamples([
        {
            json: "stripe_US_travel"
        }
    ])
    connector_label: String
    /// Unique ID of the merchant connector account
    @dataExamples([
        {
            json: "mca_5apGeP94tMts6rg3U3kR"
        }
    ])
    @required
    merchant_connector_id: String
    /// Identifier for the profile, if not provided default will be chosen from merchant account
    @length(
        max: 64
    )
    @required
    profile_id: String
    payment_methods_enabled: MerchantConnectorListResponsePaymentMethodsEnabled
    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    test_mode: Boolean
    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    disabled: Boolean
    frm_configs: MerchantConnectorListResponseFrmConfigs
    business_country: MerchantConnectorListResponseBusinessCountry
    /// The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    @dataExamples([
        {
            json: "travel"
        }
    ])
    business_label: String
    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    @dataExamples([
        {
            json: "chase"
        }
    ])
    business_sub_label: String
    applepay_verified_domains: MerchantConnectorListResponseApplepayVerifiedDomains
    pm_auth_config: Document
    @required
    status: ConnectorStatus
}

structure MerchantConnectorListResponseBusinessCountry {}

/// Response of creating a new Merchant Connector for the merchant account."
structure MerchantConnectorResponse {
    @required
    connector_type: ConnectorType
    @required
    connector_name: Connector
    /// A unique label to identify the connector account created under a profile
    @dataExamples([
        {
            json: "stripe_US_travel"
        }
    ])
    connector_label: String
    /// Unique ID of the merchant connector account
    @dataExamples([
        {
            json: "mca_5apGeP94tMts6rg3U3kR"
        }
    ])
    @required
    merchant_connector_id: String
    /// Identifier for the profile, if not provided default will be chosen from merchant account
    @length(
        max: 64
    )
    @required
    profile_id: String
    connector_account_details: MerchantConnectorResponseConnectorAccountDetails
    payment_methods_enabled: MerchantConnectorResponsePaymentMethodsEnabled
    connector_webhook_details: MerchantConnectorResponseConnectorWebhookDetails
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    test_mode: Boolean
    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    disabled: Boolean
    frm_configs: MerchantConnectorResponseFrmConfigs
    business_country: MerchantConnectorResponseBusinessCountry
    /// The business label to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    @dataExamples([
        {
            json: "travel"
        }
    ])
    business_label: String
    /// The business sublabel to which the connector account is attached. To be deprecated soon. Use the 'profile_id' instead
    @dataExamples([
        {
            json: "chase"
        }
    ])
    business_sub_label: String
    applepay_verified_domains: MerchantConnectorResponseApplepayVerifiedDomains
    pm_auth_config: Document
    @required
    status: ConnectorStatus
    additional_merchant_data: MerchantConnectorResponseAdditionalMerchantData
    connector_wallets_details: MerchantConnectorResponseConnectorWalletsDetails
}

structure MerchantConnectorResponseAdditionalMerchantData {}

structure MerchantConnectorResponseBusinessCountry {}

structure MerchantConnectorResponseConnectorAccountDetails with [MerchantConnectorDetails] {}

structure MerchantConnectorResponseConnectorWalletsDetails with [ConnectorWalletDetails] {}

structure MerchantConnectorResponseConnectorWebhookDetails with [MerchantConnectorWebhookDetails] {}

/// Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc."
structure MerchantConnectorUpdate {
    @required
    connector_type: ConnectorType
    /// This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is `default`, connector label can be `stripe_default`
    @dataExamples([
        {
            json: "stripe_US_travel"
        }
    ])
    connector_label: String
    connector_account_details: MerchantConnectorUpdateConnectorAccountDetails
    payment_methods_enabled: MerchantConnectorUpdatePaymentMethodsEnabled
    connector_webhook_details: MerchantConnectorUpdateConnectorWebhookDetails
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// A boolean value to indicate if the connector is in Test mode. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    test_mode: Boolean
    /// A boolean value to indicate if the connector is disabled. By default, its value is false.
    @dataExamples([
        {
            json: false
        }
    ])
    disabled: Boolean
    frm_configs: MerchantConnectorUpdateFrmConfigs
    /// pm_auth_config will relate MCA records to their respective chosen auth services, based on payment_method and pmt
    pm_auth_config: Document
    @required
    status: ConnectorStatus
    additional_merchant_data: MerchantConnectorUpdateAdditionalMerchantData
    connector_wallets_details: MerchantConnectorUpdateConnectorWalletsDetails
}

structure MerchantConnectorUpdateAdditionalMerchantData {}

structure MerchantConnectorUpdateConnectorAccountDetails with [MerchantConnectorDetails] {}

structure MerchantConnectorUpdateConnectorWalletsDetails with [ConnectorWalletDetails] {}

structure MerchantConnectorUpdateConnectorWebhookDetails with [MerchantConnectorWebhookDetails] {}

@mixin
structure MerchantConnectorWebhookDetails {
    @dataExamples([
        {
            json: "12345678900987654321"
        }
    ])
    @required
    merchant_secret: String
    @dataExamples([
        {
            json: "12345678900987654321"
        }
    ])
    @required
    additional_secret: String
}

@mixin
structure MerchantDetails {
    /// The merchant's primary contact name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    @length(
        max: 255
    )
    primary_contact_person: String
    /// The merchant's primary phone number
    @dataExamples([
        {
            json: "999999999"
        }
    ])
    @length(
        max: 255
    )
    primary_phone: String
    /// The merchant's primary email address
    @dataExamples([
        {
            json: "johndoe@test.com"
        }
    ])
    @length(
        max: 255
    )
    primary_email: String
    /// The merchant's secondary contact name
    @dataExamples([
        {
            json: "John Doe2"
        }
    ])
    @length(
        max: 255
    )
    secondary_contact_person: String
    /// The merchant's secondary phone number
    @dataExamples([
        {
            json: "999999988"
        }
    ])
    @length(
        max: 255
    )
    secondary_phone: String
    /// The merchant's secondary email address
    @dataExamples([
        {
            json: "johndoe2@test.com"
        }
    ])
    @length(
        max: 255
    )
    secondary_email: String
    /// The business website of the merchant
    @dataExamples([
        {
            json: "www.example.com"
        }
    ])
    @length(
        max: 255
    )
    website: String
    /// A brief description about merchant's business
    @dataExamples([
        {
            json: "Online Retail with a wide selection of organic products for North America"
        }
    ])
    @length(
        max: 255
    )
    about_business: String
    address: MerchantDetailsAddress
}

structure MerchantDetailsAddress with [AddressDetails] {}

structure MerchantRecipientDataOneOfAlt0 {
    @required
    connector_recipient_id: String
}

structure MerchantRecipientDataOneOfAlt1 {
    @required
    wallet_id: String
}

structure MerchantRecipientDataOneOfAlt2 {
    @required
    account_data: MerchantAccountData
}

structure MerchantRoutingAlgorithm with [MerchantRoutingAlgorithmMixin] {}

/// Routing Algorithm specific to merchants
@mixin
structure MerchantRoutingAlgorithmMixin {
    @required
    id: String
    @required
    profile_id: String
    @required
    name: String
    @required
    description: String
    @required
    algorithm: RoutingAlgorithmWrapper
    @required
    created_at: Long
    @required
    modified_at: Long
    @required
    algorithm_for: TransactionType
}

structure MetadataValue {
    @required
    key: String
    @required
    value: String
}

structure MifinityData {
    @dateFormat
    @required
    date_of_birth: String
    language_preference: String
}

structure MinimumAmount {}

structure MobilePaymentDataOneOfAlt0 {
    @required
    direct_carrier_billing: DirectCarrierBilling
}

structure MobilePaymentNextStepData {
    @required
    consent_data_required: MobilePaymentConsent
}

structure MobilePaymentResponse {}

structure MultibancoBankTransfer {
    billing_details: BankTransferDataOneOfAlt3MultibancoBankTransferBillingDetails
}

@mixin
structure MultibancoBillingDetails {
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
}

structure MultibancoTransferInstructions {
    @dataExamples([
        {
            json: "122385736258"
        }
    ])
    @required
    reference: String
    @dataExamples([
        {
            json: "12345"
        }
    ])
    @required
    entity: String
}

structure MultiUse with [MandateAmountDataMixin] {}

structure NetworkTransactionIdAndCardDetails {
    /// The card number
    @dataExamples([
        {
            json: "4242424242424242"
        }
    ])
    @required
    card_number: String
    /// The card's expiry month
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_month: String
    /// The card's expiry year
    @dataExamples([
        {
            json: "24"
        }
    ])
    @required
    card_exp_year: String
    /// The card holder's name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @required
    card_holder_name: String
    /// The name of the issuer of card
    @dataExamples([
        {
            json: "chase"
        }
    ])
    card_issuer: String
    card_network: NetworkTransactionIdAndCardDetailsCardNetwork
    @dataExamples([
        {
            json: "CREDIT"
        }
    ])
    card_type: String
    @dataExamples([
        {
            json: "INDIA"
        }
    ])
    card_issuing_country: String
    @dataExamples([
        {
            json: "JP_AMEX"
        }
    ])
    bank_code: String
    /// The card holder's nick name
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    nick_name: String
    /// The network transaction ID provided by the card network during a CIT (Customer Initiated Transaction),
    /// where `setup_future_usage` is set to `off_session`.
    @required
    network_transaction_id: String
}

structure NetworkTransactionIdAndCardDetailsCardNetwork {}

/// Contains the url for redirection flow
structure NextActionDataOneOfAlt0 {
    @required
    redirect_to_url: String
}

structure NextActionDataOneOfAlt1 {
    @required
    popup_url: String
    @required
    redirect_response_url: String
}

/// Contains consent to collect otp for mobile payment
structure NextActionDataOneOfAlt10 {
    @required
    consent_data_required: MobilePaymentConsent
}

/// Contains data required to invoke hidden iframe
structure NextActionDataOneOfAlt11 {
    @required
    iframe_data: IframeData
}

/// Informs the next steps for bank transfer and also contains the charges details (ex: amount received, amount charged etc)
structure NextActionDataOneOfAlt2 {
    @required
    bank_transfer_steps_and_charges_details: BankTransferNextStepsData
}

/// Contains third party sdk session token response
structure NextActionDataOneOfAlt3 {
    session_token: NextActionDataOneOfAlt3SessionToken
}

structure NextActionDataOneOfAlt3SessionToken {}

/// Contains url for Qr code image, this qr code has to be shown in sdk
structure NextActionDataOneOfAlt4 {
    /// Hyperswitch generated image data source url
    @required
    image_data_url: String
    display_to_timestamp: Long
    /// The url for Qr code given by the connector
    @required
    qr_code_url: String
    display_text: String
    border_color: String
}

/// Contains url to fetch Qr code data
structure NextActionDataOneOfAlt5 {
    @required
    qr_code_fetch_url: String
}

/// Contains the download url and the reference number for transaction
structure NextActionDataOneOfAlt6 {
    @required
    voucher_details: String
}

/// Contains duration for displaying a wait screen, wait screen with timer is displayed by sdk
structure NextActionDataOneOfAlt7 {
    @required
    display_from_timestamp: Integer
    display_to_timestamp: Integer
    poll_config: PollConfig
}

/// Contains the information regarding three_ds_method_data submission, three_ds authentication, and authorization flows
structure NextActionDataOneOfAlt8 {
    @required
    three_ds_data: ThreeDsData
}

structure NextActionDataOneOfAlt9 {
    @required
    next_action_data: SdkNextActionData
}

structure Noon with [NoonData] {}

@mixin
structure NoonData {
    /// Information about the order category that merchant wants to specify at connector level. (e.g. In Noon Payments it can take values like "pay", "food", or any other custom string set by the merchant in Noon's Dashboard)
    order_category: String
}

structure NoThirdPartySdkSessionResponse {
    /// Timestamp at which session is requested
    @range(
        min: 0
    )
    @required
    epoch_timestamp: Long
    /// Timestamp at which session expires
    @range(
        min: 0
    )
    @required
    expires_at: Long
    /// The identifier for the merchant session
    @required
    merchant_session_identifier: String
    /// Apple pay generated unique ID (UUID) value
    @required
    nonce: String
    /// The identifier for the merchant
    @required
    merchant_identifier: String
    /// The domain name of the merchant which is registered in Apple Pay
    @required
    domain_name: String
    /// The name to be displayed on Apple Pay button
    @required
    display_name: String
    /// A string which represents the properties of a payment
    @required
    signature: String
    /// The identifier for the operational analytics
    @required
    operational_analytics_identifier: String
    /// The number of retries to get the session response
    @range(
        min: 0
    )
    @required
    retries: Integer
    /// The identifier for the connector transaction
    @required
    psp_id: String
}

/// Represents a number comparison for "NumberComparisonArrayValue"
structure NumberComparison {
    @required
    comparisonType: ComparisonType
    @required
    number: MinorUnit
}

structure Online with [OnlineMandate] {}

structure OnlineBankingCzechRepublic {
    @required
    issuer: BankNames
}

structure OnlineBankingFinland {
    email: String
}

structure OnlineBankingFpx {
    @required
    issuer: BankNames
}

structure OnlineBankingPoland {
    @required
    issuer: BankNames
}

structure OnlineBankingSlovakia {
    @required
    issuer: BankNames
}

structure OnlineBankingThailand {
    @required
    issuer: BankNames
}

/// Details of online mandate
@mixin
structure OnlineMandate {
    /// Ip address of the customer machine from which the mandate was created
    @dataExamples([
        {
            json: "123.32.25.123"
        }
    ])
    @required
    ip_address: String
    /// The user-agent of the customer's browser
    @required
    user_agent: String
}

structure OpenBankingDataOneOfAlt0 {
    @required
    open_banking_pis: Document
}

structure OpenBankingResponse {}

@mixin
structure OpenBankingSessionToken {
    /// The session token for OpenBanking Connectors
    @required
    open_banking_session_token: String
}

structure OpenBankingUk {
    @required
    issuer: BankNames
    @required
    country: CountryAlpha2
}

structure OrderDetailsWithAmount {
    /// Name of the product that is being purchased
    @dataExamples([
        {
            json: "shirt"
        }
    ])
    @length(
        max: 255
    )
    @required
    product_name: String
    /// The quantity of the product to be purchased
    @dataExamples([
        {
            json: 1
        }
    ])
    @range(
        min: 0
    )
    @required
    quantity: Integer
    /// the amount per quantity of product
    @required
    amount: Long
    /// tax rate applicable to the product
    tax_rate: Double
    /// total tax amount applicable to the product
    total_tax_amount: Long
    requires_shipping: Boolean
    /// The image URL of the product
    product_img_link: String
    /// ID of the product that is being purchased
    product_id: String
    /// Category of the product that is being purchased
    category: String
    /// Sub category of the product that is being purchased
    sub_category: String
    /// Brand of the product that is being purchased
    brand: String
    product_type: OrderDetailsWithAmountProductType
    /// The tax code for the product
    product_tax_code: String
}

structure OrderDetailsWithAmountProductType {}

structure OrganizationCreateRequest {
    /// Name of the organization
    @required
    organization_name: String
    /// Details about the organization
    organization_details: Document
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
}

structure OrganizationResponse {
    /// The unique identifier for the Organization
    @dataExamples([
        {
            json: "org_q98uSGAYbjEwqs0mJwnz"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    @required
    organization_id: String
    /// Name of the Organization
    organization_name: String
    /// Details about the organization
    organization_details: Document
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    @required
    @timestampFormat("date-time")
    modified_at: Timestamp
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
}

structure OrganizationUpdateRequest {
    /// Name of the organization
    organization_name: String
    /// Details about the organization
    organization_details: Document
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// Platform merchant id is unique distiguisher for special merchant in the platform org
    @required
    platform_merchant_id: String
}

structure OutgoingWebhook {
    /// The merchant id of the merchant
    @required
    merchant_id: String
    /// The unique event id for each webhook
    @required
    event_id: String
    @required
    event_type: EventType
    @required
    content: OutgoingWebhookContent
    /// The time at which webhook was sent
    @timestampFormat("date-time")
    timestamp: Timestamp
}

structure OutgoingWebhookContentOneOfAlt0 {
    @required
    object: PaymentsResponse
}

structure OutgoingWebhookContentOneOfAlt1 {
    @required
    object: RefundResponse
}

structure OutgoingWebhookContentOneOfAlt2 {
    @required
    object: DisputeResponse
}

structure OutgoingWebhookContentOneOfAlt3 {
    @required
    object: MandateResponse
}

structure OutgoingWebhookContentOneOfAlt4 {
    @required
    object: PayoutCreateResponse
}

/// The request information (headers and body) sent in the webhook.
structure OutgoingWebhookRequestContent {
    /// The request body sent in the webhook.
    @required
    body: String
    @required
    headers: OutgoingWebhookRequestContentHeaders
}

structure OutgoingWebhookRequestContentHeadersItemItem {}

/// The response information (headers, body and status code) received for the webhook sent.
structure OutgoingWebhookResponseContent {
    /// The response body received for the webhook sent.
    body: String
    headers: OutgoingWebhookResponseContentHeaders
    /// The HTTP status code for the webhook sent.
    @dataExamples([
        {
            json: 200
        }
    ])
    @range(
        min: 0
    )
    status_code: Integer
    /// Error message in case any error occurred when trying to deliver the webhook.
    @dataExamples([
        {
            json: "200"
        }
    ])
    error_message: String
}

structure OutgoingWebhookResponseContentHeadersItemItem {}

structure PayLaterDataOneOfAlt0 {
    @required
    klarna_redirect: KlarnaRedirect
}

structure PayLaterDataOneOfAlt1 {
    @required
    klarna_sdk: PayLaterDataOneOfAlt1KlarnaSdk
}

/// For Klarna Sdk as PayLater Option
structure PayLaterDataOneOfAlt1KlarnaSdk {
    /// The token for the sdk workflow
    @required
    token: String
}

structure PayLaterDataOneOfAlt2 {
    /// For Affirm redirect as PayLater Option
    @required
    affirm_redirect: Document
}

structure PayLaterDataOneOfAlt3 {
    @required
    afterpay_clearpay_redirect: AfterpayClearpayRedirect
}

structure PayLaterDataOneOfAlt4 {
    /// For PayBright Redirect as PayLater Option
    @required
    pay_bright_redirect: Document
}

structure PayLaterDataOneOfAlt5 {
    /// For WalleyRedirect as PayLater Option
    @required
    walley_redirect: Document
}

structure PayLaterDataOneOfAlt6 {
    /// For Alma Redirection as PayLater Option
    @required
    alma_redirect: Document
}

structure PayLaterDataOneOfAlt7 {
    @required
    atome_redirect: Document
}

structure PaylaterResponse {
    klarna_sdk: PaylaterResponseKlarnaSdk
}

structure PaylaterResponseKlarnaSdk with [KlarnaSdkPaymentMethodResponse] {}

structure PaymentAttemptResponse {
    /// A unique identifier for this specific payment attempt.
    @required
    attempt_id: String
    @required
    status: AttemptStatus
    /// The payment attempt amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    /// The payment attempt tax_amount.
    @dataExamples([
        {
            json: 6540
        }
    ])
    order_tax_amount: Long
    currency: PaymentAttemptResponseCurrency
    /// The name of the payment connector (e.g., 'stripe', 'adyen') used for this attempt.
    connector: String
    /// A human-readable message from the connector explaining the error, if one occurred during this payment attempt.
    error_message: String
    payment_method: PaymentAttemptResponsePaymentMethod
    /// A unique identifier for a payment provided by the connector
    connector_transaction_id: String
    capture_method: PaymentAttemptResponseCaptureMethod
    authentication_type: PaymentAttemptResponseAuthenticationType
    /// Time at which the payment attempt was created
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
    /// Time at which the payment attempt was last modified
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    modified_at: Timestamp
    /// If the payment was cancelled the reason will be provided here
    cancellation_reason: String
    /// If this payment attempt is associated with a mandate (e.g., for a recurring or subsequent payment), this field will contain the ID of that mandate.
    mandate_id: String
    /// The error code returned by the connector if this payment attempt failed. This code is specific to the connector.
    error_code: String
    /// If a tokenized (saved) payment method was used for this attempt, this field contains the payment token representing that payment method.
    payment_token: String
    connector_metadata: PaymentAttemptResponseConnectorMetadata
    payment_experience: PaymentAttemptResponsePaymentExperience
    payment_method_type: PaymentAttemptResponsePaymentMethodType
    /// The connector's own reference or transaction ID for this specific payment attempt. Useful for reconciliation with the connector.
    @dataExamples([
        {
            json: "993672945374576J"
        }
    ])
    reference_id: String
    /// (This field is not live yet)Error code unified across the connectors is received here if there was an error while calling connector
    unified_code: String
    /// (This field is not live yet)Error message unified across the connectors is received here if there was an error while calling connector
    unified_message: String
    /// Value passed in X-CLIENT-SOURCE header during payments confirm request by the client
    client_source: String
    /// Value passed in X-CLIENT-VERSION header during payments confirm request by the client
    client_version: String
}

structure PaymentAttemptResponseAuthenticationType {}

structure PaymentAttemptResponseCaptureMethod {}

structure PaymentAttemptResponseCurrency {}

structure PaymentAttemptResponsePaymentExperience {}

structure PaymentAttemptResponsePaymentMethod {}

structure PaymentAttemptResponsePaymentMethodType {}

structure PaymentChargeTypeOneOfAlt0 {
    @required
    Stripe: StripeChargeType
}

/// Configure a custom payment link for the particular payment
@mixin
structure PaymentCreatePaymentLinkConfig {}

/// Represents the payment data used in the 3DS decision rule.
structure PaymentData {
    /// The amount of the payment in minor units (e.g., cents for USD).
    @required
    amount: Long
    @required
    currency: Currency
}

structure PaymentExperienceType {}

structure PaymentExperienceTypes {
    payment_experience_type: PaymentExperienceType
    @required
    eligible_connectors: PaymentExperienceTypesEligibleConnectors
}

@mixin
structure PaymentLinkBackgroundImageConfig {
    /// URL of the image
    @dataExamples([
        {
            json: "https://hyperswitch.io/favicon.ico"
        }
    ])
    @required
    url: String
    position: Position
    size: Size
}

structure PaymentLinkConfig {
    /// custom theme for the payment link
    @required
    theme: String
    /// merchant display logo
    @required
    logo: String
    /// Custom merchant name for payment link
    @required
    seller_name: String
    /// Custom layout for sdk
    @required
    sdk_layout: String
    /// Display only the sdk for payment link
    @required
    display_sdk_only: Boolean
    /// Enable saved payment method option for payment link
    @required
    enabled_saved_payment_method: Boolean
    /// Hide card nickname field option for payment link
    @required
    hide_card_nickname_field: Boolean
    /// Show card form by default for payment link
    @required
    show_card_form_by_default: Boolean
    allowed_domains: PaymentLinkConfigAllowedDomains
    transaction_details: PaymentLinkConfigTransactionDetails
    background_image: PaymentLinkConfigBackgroundImage
    details_layout: PaymentLinkConfigDetailsLayout
    /// Toggle for HyperSwitch branding visibility
    branding_visibility: Boolean
    /// Text for payment link's handle confirm button
    payment_button_text: String
    /// Text for customizing message for card terms
    custom_message_for_card_terms: String
    /// Custom background colour for payment link's handle confirm button
    payment_button_colour: String
    /// Skip the status screen after payment completion
    skip_status_screen: Boolean
    /// Custom text colour for payment link's handle confirm button
    payment_button_text_colour: String
    /// Custom background colour for the payment link
    background_colour: String
    sdk_ui_rules: PaymentLinkConfigSdkUiRules
    payment_link_ui_rules: PaymentLinkConfigPaymentLinkUiRules
    /// Flag to enable the button only when the payment form is ready for submission
    @required
    enable_button_only_on_form_ready: Boolean
    /// Optional header for the SDK's payment form
    payment_form_header_text: String
    payment_form_label_type: PaymentLinkConfigPaymentFormLabelType
    show_card_terms: PaymentLinkConfigShowCardTerms
    /// Boolean to control payment button text for setup mandate calls
    is_setup_mandate_flow: Boolean
    /// Hex color for the CVC icon during error state
    color_icon_card_cvc_error: String
}

structure PaymentLinkConfigBackgroundImage with [PaymentLinkBackgroundImageConfig] {}

structure PaymentLinkConfigDetailsLayout {}

structure PaymentLinkConfigPaymentFormLabelType {}

structure PaymentLinkConfigRequestBackgroundImage with [PaymentLinkBackgroundImageConfig] {}

structure PaymentLinkConfigRequestDetailsLayout {}

structure PaymentLinkConfigRequestPaymentFormLabelType {}

structure PaymentLinkConfigRequestShowCardTerms {}

structure PaymentLinkConfigShowCardTerms {}

structure PaymentLinkInitiateRequest {
    @required
    merchant_id: String
    @required
    payment_id: String
}

@mixin
structure PaymentLinkResponse {
    /// URL for rendering the open payment link
    @required
    link: String
    /// URL for rendering the secure payment link
    secure_link: String
    /// Identifier for the payment link
    @required
    payment_link_id: String
}

structure PaymentLinkTransactionDetails {
    /// Key for the transaction details
    @dataExamples([
        {
            json: "Policy-Number"
        }
    ])
    @length(
        max: 255
    )
    @required
    key: String
    /// Value for the transaction details
    @dataExamples([
        {
            json: "297472368473924"
        }
    ])
    @length(
        max: 255
    )
    @required
    value: String
    ui_configuration: UiConfiguration
}

structure PaymentListConstraints {
    /// The identifier for customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// A cursor for use in pagination, fetch the next list after some object
    @dataExamples([
        {
            json: "pay_fafa124123"
        }
    ])
    starting_after: String
    /// A cursor for use in pagination, fetch the previous list before some object
    @dataExamples([
        {
            json: "pay_fafa124123"
        }
    ])
    ending_before: String
    /// limit on the number of objects to return
    @range(
        min: 0
        max: 100
    )
    limit: Integer
    /// The time at which payment is created
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
    /// Time less than the payment created time
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @jsonName("created.lt")
    @timestampFormat("date-time")
    createdlt: Timestamp
    /// Time greater than the payment created time
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @jsonName("created.gt")
    @timestampFormat("date-time")
    createdgt: Timestamp
    /// Time less than or equals to the payment created time
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @jsonName("created.lte")
    @timestampFormat("date-time")
    createdlte: Timestamp
    /// Time greater than or equals to the payment created time
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @jsonName("created.gte")
    @timestampFormat("date-time")
    createdgte: Timestamp
}

structure PaymentListResponse {
    /// The number of payments included in the list
    @range(
        min: 0
    )
    @required
    size: Integer
    @required
    data: PaymentListResponseData
}

structure PaymentMethodCollectLinkRequest {
    /// The unique identifier for the collect link.
    @dataExamples([
        {
            json: "pm_collect_link_2bdacf398vwzq5n422S1"
        }
    ])
    pm_collect_link_id: String
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "cus_92dnwed8s32bV9D8Snbiasd8v"
        }
    ])
    @required
    customer_id: String
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Redirect to this URL post completion
    @dataExamples([
        {
            json: "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status"
        }
    ])
    return_url: String
    enabled_payment_methods: PaymentMethodCollectLinkRequestAllOf1EnabledPaymentMethods
}

structure PaymentMethodCollectLinkResponse with [GenericLinkUiConfig] {
    /// The unique identifier for the collect link.
    @dataExamples([
        {
            json: "pm_collect_link_2bdacf398vwzq5n422S1"
        }
    ])
    @required
    pm_collect_link_id: String
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "cus_92dnwed8s32bV9D8Snbiasd8v"
        }
    ])
    @required
    customer_id: String
    /// Time when this link will be expired in ISO8601 format
    @dataExamples([
        {
            json: "2025-01-18T11:04:09.922Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    expiry: Timestamp
    /// URL to the form's link generated for collecting payment method details.
    @dataExamples([
        {
            json: "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1"
        }
    ])
    @required
    link: String
    /// Redirect to this URL post completion
    @dataExamples([
        {
            json: "https://sandbox.hyperswitch.io/payment_method/collect/pm_collect_link_2bdacf398vwzq5n422S1/status"
        }
    ])
    return_url: String
    enabled_payment_methods: PaymentMethodCollectLinkResponseAllOf1EnabledPaymentMethods
}

structure PaymentMethodCreate {
    @required
    payment_method: PaymentMethod
    payment_method_type: PaymentMethodCreatePaymentMethodType
    /// The name of the bank/ provider issuing the payment method to the end user
    @dataExamples([
        {
            json: "Citibank"
        }
    ])
    payment_method_issuer: String
    payment_method_issuer_code: PaymentMethodCreatePaymentMethodIssuerCode
    card: PaymentMethodCreateCard
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// The card network
    @dataExamples([
        {
            json: "Visa"
        }
    ])
    card_network: String
    bank_transfer: PaymentMethodCreateBankTransfer
    wallet: PaymentMethodCreateWallet
    /// For Client based calls, SDK will use the client_secret
    /// in order to call /payment_methods
    /// Client secret will be generated whenever a new
    /// payment method is created
    client_secret: String
    payment_method_data: PaymentMethodCreatePaymentMethodData
    billing: PaymentMethodCreateBilling
}

structure PaymentMethodCreateBankTransfer {}

structure PaymentMethodCreateCard with [CardDetailMixin] {}

structure PaymentMethodCreateDataOneOfAlt0 {
    @required
    card: CardDetail
}

structure PaymentMethodCreatePaymentMethodData {}

structure PaymentMethodCreatePaymentMethodIssuerCode {}

structure PaymentMethodCreatePaymentMethodType {}

structure PaymentMethodCreateWallet {}

structure PaymentMethodDataOneOfAlt0 {
    @required
    card: Card
}

structure PaymentMethodDataOneOfAlt1 {
    @required
    card_redirect: CardRedirectData
}

structure PaymentMethodDataOneOfAlt11 {
    @required
    upi: UpiData
}

structure PaymentMethodDataOneOfAlt12 {
    @required
    voucher: VoucherData
}

structure PaymentMethodDataOneOfAlt13 {
    @required
    gift_card: GiftCardData
}

structure PaymentMethodDataOneOfAlt14 {
    @required
    card_token: CardToken
}

structure PaymentMethodDataOneOfAlt15 {
    @required
    open_banking: OpenBankingData
}

structure PaymentMethodDataOneOfAlt16 {
    @required
    mobile_payment: MobilePaymentData
}

structure PaymentMethodDataOneOfAlt2 {
    @required
    wallet: WalletData
}

structure PaymentMethodDataOneOfAlt3 {
    @required
    pay_later: PayLaterData
}

structure PaymentMethodDataOneOfAlt4 {
    @required
    bank_redirect: BankRedirectData
}

structure PaymentMethodDataOneOfAlt5 {
    @required
    bank_debit: BankDebitData
}

structure PaymentMethodDataOneOfAlt6 {
    @required
    bank_transfer: BankTransferData
}

structure PaymentMethodDataOneOfAlt7 {
    @required
    real_time_payment: RealTimePaymentData
}

structure PaymentMethodDataOneOfAlt8 {
    @required
    crypto: CryptoData
}

/// The payment method information provided for making a payment
@mixin
structure PaymentMethodDataRequest {
    billing: PaymentMethodDataRequestAllOf1Billing
}

structure PaymentMethodDataResponseOneOfAlt0 {
    @required
    card: CardResponse
}

structure PaymentMethodDataResponseOneOfAlt1 {
    @required
    bank_transfer: BankTransferResponse
}

structure PaymentMethodDataResponseOneOfAlt10 {
    @required
    upi: UpiResponse
}

structure PaymentMethodDataResponseOneOfAlt11 {
    @required
    voucher: VoucherResponse
}

structure PaymentMethodDataResponseOneOfAlt12 {
    @required
    gift_card: GiftCardResponse
}

structure PaymentMethodDataResponseOneOfAlt13 {
    @required
    card_redirect: CardRedirectResponse
}

structure PaymentMethodDataResponseOneOfAlt14 {
    @required
    card_token: CardTokenResponse
}

structure PaymentMethodDataResponseOneOfAlt15 {
    @required
    open_banking: OpenBankingResponse
}

structure PaymentMethodDataResponseOneOfAlt16 {
    @required
    mobile_payment: MobilePaymentResponse
}

structure PaymentMethodDataResponseOneOfAlt2 {
    @required
    wallet: WalletResponse
}

structure PaymentMethodDataResponseOneOfAlt3 {
    @required
    pay_later: PaylaterResponse
}

structure PaymentMethodDataResponseOneOfAlt4 {
    @required
    bank_redirect: BankRedirectResponse
}

structure PaymentMethodDataResponseOneOfAlt5 {
    @required
    crypto: CryptoResponse
}

structure PaymentMethodDataResponseOneOfAlt6 {
    @required
    bank_debit: BankDebitResponse
}

structure PaymentMethodDataResponseOneOfAlt7 {
    @required
    mandate_payment: Document
}

structure PaymentMethodDataResponseOneOfAlt8 {
    @required
    reward: Document
}

structure PaymentMethodDataResponseOneOfAlt9 {
    @required
    real_time_payment: RealTimePaymentDataResponse
}

@mixin
structure PaymentMethodDataResponseWithBilling {
    billing: PaymentMethodDataResponseWithBillingAllOf1Billing
}

structure PaymentMethodDeleteResponse {
    /// The unique identifier of the Payment method
    @dataExamples([
        {
            json: "card_rGK4Vi5iSW70MY7J2mIg"
        }
    ])
    @required
    payment_method_id: String
    /// Whether payment method was deleted or not
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    deleted: Boolean
}

structure PaymentMethodListResponse {
    /// Redirect URL of the merchant
    @dataExamples([
        {
            json: "https://www.google.com"
        }
    ])
    redirect_url: String
    @required
    currency: Currency
    @required
    payment_methods: PaymentMethodListResponsePaymentMethods
    @required
    mandate_payment: MandateType
    merchant_name: String
    /// flag to indicate if surcharge and tax breakup screen should be shown or not
    @required
    show_surcharge_breakup_screen: Boolean
    payment_type: PaymentMethodListResponsePaymentType
    /// flag to indicate whether to perform external 3ds authentication
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    request_external_three_ds_authentication: Boolean
    /// flag that indicates whether to collect shipping details from wallets or from the customer
    collect_shipping_details_from_wallets: Boolean
    /// flag that indicates whether to collect billing details from wallets or from the customer
    collect_billing_details_from_wallets: Boolean
    /// flag that indicates whether to calculate tax on the order amount
    @required
    is_tax_calculation_enabled: Boolean
}

structure PaymentMethodListResponsePaymentType {}

/// Represents metadata about the payment method used in the 3DS decision rule.
@mixin
structure PaymentMethodMetaData {
    @required
    card_network: CardNetwork
}

structure PaymentMethodResponse with [PaymentMethodResponseMixin] {}

structure PaymentMethodResponseBankTransfer {}

structure PaymentMethodResponseCard with [CardDetailFromLocker] {}

@mixin
structure PaymentMethodResponseMixin {
    /// Unique identifier for a merchant
    @dataExamples([
        {
            json: "merchant_1671528864"
        }
    ])
    @required
    merchant_id: String
    /// The unique identifier of the customer.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// The unique identifier of the Payment method
    @dataExamples([
        {
            json: "card_rGK4Vi5iSW70MY7J2mIg"
        }
    ])
    @required
    payment_method_id: String
    @required
    payment_method: PaymentMethod
    payment_method_type: PaymentMethodResponsePaymentMethodType
    card: PaymentMethodResponseCard
    /// Indicates whether the payment method supports recurring payments. Optional.
    @dataExamples([
        {
            json: true
        }
    ])
    recurring_enabled: Boolean
    /// Indicates whether the payment method is eligible for installment payments (e.g., EMI, BNPL). Optional.
    @dataExamples([
        {
            json: true
        }
    ])
    installment_payment_enabled: Boolean
    payment_experience: PaymentMethodResponsePaymentExperience
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// A timestamp (ISO 8601 code) that determines when the payment method was created
    @dataExamples([
        {
            json: "2023-01-18T11:04:09.922Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
    bank_transfer: PaymentMethodResponseBankTransfer
    @dataExamples([
        {
            json: "2024-02-24T11:04:09.922Z"
        }
    ])
    @timestampFormat("date-time")
    last_used_at: Timestamp
    /// For Client based calls
    client_secret: String
}

structure PaymentMethodResponsePaymentMethodType {}

/// Details of all the payment methods enabled for the connector for the given merchant account
structure PaymentMethodsEnabled {
    @required
    payment_method: PaymentMethod
    payment_method_types: PaymentMethodsEnabledPaymentMethodTypes
}

structure PaymentMethodUpdate {
    card: PaymentMethodUpdateCard
    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    @dataExamples([
        {
            json: "secret_k2uj3he2893eiu2d"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    client_secret: String
}

structure PaymentMethodUpdateCard with [CardDetailUpdate] {}

@mixin
structure PaymentProcessingDetails {
    @required
    payment_processing_certificate: String
    @required
    payment_processing_certificate_key: String
}

structure PaymentProcessingDetailsAtOneOfAlt0 with [PaymentProcessingDetails] {}

structure PaymentRequestData with [ApplePayPaymentRequest] {}

structure PaymentRetrieveBody {
    /// The identifier for the Merchant Account.
    merchant_id: String
    /// Decider to enable or disable the connector call for retrieve request
    force_sync: Boolean
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    client_secret: String
    /// If enabled provides list of captures linked to latest attempt
    expand_captures: Boolean
    /// If enabled provides list of attempts linked to payment intent
    expand_attempts: Boolean
    /// If enabled, provides whole connector response
    all_keys_required: Boolean
}

structure PaymentsCancelRequest {
    /// The reason for the payment cancel
    cancellation_reason: String
    merchant_connector_details: PaymentsCancelRequestMerchantConnectorDetails
}

structure PaymentsCancelRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsCaptureRequest {
    /// The unique identifier for the merchant. This is usually inferred from the API key.
    merchant_id: String
    /// The amount to capture, in the lowest denomination of the currency. If omitted, the entire `amount_capturable` of the payment will be captured. Must be less than or equal to the current `amount_capturable`.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_to_capture: Long
    /// Decider to refund the uncaptured amount. (Currently not fully supported or behavior may vary by connector).
    refund_uncaptured_amount: Boolean
    /// A dynamic suffix that appears on your customer's credit card statement. This is concatenated with the (shortened) descriptor prefix set on your account to form the complete statement descriptor. The combined length should not exceed connector-specific limits (typically 22 characters).
    statement_descriptor_suffix: String
    /// An optional prefix for the statement descriptor that appears on your customer's credit card statement. This can override the default prefix set on your merchant account. The combined length of prefix and suffix should not exceed connector-specific limits (typically 22 characters).
    statement_descriptor_prefix: String
    merchant_connector_details: PaymentsCaptureRequestMerchantConnectorDetails
}

structure PaymentsCaptureRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsCompleteAuthorizeRequest {
    shipping: PaymentsCompleteAuthorizeRequestShipping
    /// Client Secret
    @required
    client_secret: String
    threeds_method_comp_ind: PaymentsCompleteAuthorizeRequestThreedsMethodCompInd
}

structure PaymentsCompleteAuthorizeRequestThreedsMethodCompInd {}

structure PaymentsConfirmRequest {
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 0
    )
    amount: Long
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    @dataExamples([
        {
            json: 6540
        }
    ])
    order_tax_amount: Long
    currency: PaymentsConfirmRequestCurrency
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_to_capture: Long
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    @dataExamples([
        {
            json: 6540
        }
    ])
    shipping_cost: Long
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    payment_id: String
    routing: PaymentsConfirmRequestRouting
    connector: PaymentsConfirmRequestConnector
    capture_method: PaymentsConfirmRequestCaptureMethod
    authentication_type: PaymentsConfirmRequestAuthenticationType
    billing: PaymentsConfirmRequestBilling
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    @dataExamples([
        {
            json: true
        }
    ])
    confirm: Boolean
    customer: PaymentsConfirmRequestCustomer
    /// The identifier for the customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    @dataExamples([
        {
            json: true
        }
    ])
    off_session: Boolean
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    @dataExamples([
        {
            json: "It's my first payment request"
        }
    ])
    description: String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    @length(
        max: 2048
    )
    return_url: String
    setup_future_usage: PaymentsConfirmRequestSetupFutureUsage
    payment_method_data: PaymentsConfirmRequestPaymentMethodData
    payment_method: PaymentsConfirmRequestPaymentMethod
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payment_token: String
    shipping: PaymentsConfirmRequestShipping
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    @dataExamples([
        {
            json: "Hyperswitch Router"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_name: String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    @dataExamples([
        {
            json: "Payment for shoes purchase"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_suffix: String
    order_details: PaymentsConfirmRequestOrderDetails
    /// It's a token used for client side verification.
    @dataExamples([
        {
            json: "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    client_secret: String
    mandate_data: PaymentsConfirmRequestMandateData
    customer_acceptance: PaymentsConfirmRequestCustomerAcceptance
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    @dataExamples([
        {
            json: "mandate_iwer89rnjef349dni3"
        }
    ])
    @length(
        max: 64
    )
    mandate_id: String
    browser_info: PaymentsConfirmRequestBrowserInfo
    payment_experience: PaymentsConfirmRequestPaymentExperience
    payment_method_type: PaymentsConfirmRequestPaymentMethodType
    merchant_connector_details: PaymentsConfirmRequestMerchantConnectorDetails
    allowed_payment_method_types: PaymentsConfirmRequestAllowedPaymentMethodTypes
    retry_action: PaymentsConfirmRequestRetryAction
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    connector_metadata: PaymentsConfirmRequestConnectorMetadata
    /// Whether to generate the payment link for this payment or not (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    payment_link: Boolean
    payment_link_config: PaymentsConfirmRequestPaymentLinkConfig
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: String
    payment_type: PaymentsConfirmRequestPaymentType
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Additional data related to some frm(Fraud Risk Management) connectors
    frm_metadata: Document
    /// Whether to perform external authentication (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    request_external_three_ds_authentication: Boolean
    recurring_details: PaymentsConfirmRequestRecurringDetails
    split_payments: PaymentsConfirmRequestSplitPayments
    /// Optional boolean value to extent authorization period of this payment
    /// 
    /// capture method must be manual or manual_multiple
    request_extended_authorization: Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "Custom_Order_id_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: Boolean
    psd2_sca_exemption_type: PaymentsConfirmRequestPsd2ScaExemptionType
    ctp_service_details: PaymentsConfirmRequestCtpServiceDetails
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    threeds_method_comp_ind: PaymentsConfirmRequestThreedsMethodCompInd
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: Boolean
    /// If enabled, provides whole connector response
    all_keys_required: Boolean
}

structure PaymentsConfirmRequestAuthenticationType {}

structure PaymentsConfirmRequestBrowserInfo with [BrowserInformation] {}

structure PaymentsConfirmRequestCaptureMethod {}

structure PaymentsConfirmRequestConnectorMetadata with [ConnectorMetadata] {}

structure PaymentsConfirmRequestCtpServiceDetails with [CtpServiceDetails] {}

structure PaymentsConfirmRequestCurrency {}

structure PaymentsConfirmRequestCustomerAcceptance with [CustomerAcceptance] {}

structure PaymentsConfirmRequestMandateData with [MandateData] {}

structure PaymentsConfirmRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsConfirmRequestPaymentExperience {}

structure PaymentsConfirmRequestPaymentLinkConfig with [PaymentCreatePaymentLinkConfig] {}

structure PaymentsConfirmRequestPaymentMethod {}

structure PaymentsConfirmRequestPaymentMethodData with [PaymentMethodDataRequest] {}

structure PaymentsConfirmRequestPaymentMethodType {}

structure PaymentsConfirmRequestPaymentType {}

structure PaymentsConfirmRequestPsd2ScaExemptionType {}

structure PaymentsConfirmRequestRecurringDetails {}

structure PaymentsConfirmRequestRetryAction {}

structure PaymentsConfirmRequestRouting {}

structure PaymentsConfirmRequestSetupFutureUsage {}

structure PaymentsConfirmRequestSplitPayments {}

structure PaymentsConfirmRequestThreedsMethodCompInd {}

structure PaymentsCreateRequest {
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    @range(
        min: 0
    )
    @required
    amount: Long
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    @dataExamples([
        {
            json: 6540
        }
    ])
    order_tax_amount: Long
    @required
    currency: Currency
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_to_capture: Long
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    @dataExamples([
        {
            json: 6540
        }
    ])
    shipping_cost: Long
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    payment_id: String
    routing: PaymentsCreateRequestRouting
    connector: PaymentsCreateRequestConnector
    capture_method: PaymentsCreateRequestCaptureMethod
    authentication_type: PaymentsCreateRequestAuthenticationType
    billing: PaymentsCreateRequestBilling
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    @dataExamples([
        {
            json: true
        }
    ])
    confirm: Boolean
    customer: PaymentsCreateRequestCustomer
    /// The identifier for the customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    @dataExamples([
        {
            json: true
        }
    ])
    off_session: Boolean
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    @dataExamples([
        {
            json: "It's my first payment request"
        }
    ])
    description: String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    @length(
        max: 2048
    )
    return_url: String
    setup_future_usage: PaymentsCreateRequestSetupFutureUsage
    payment_method_data: PaymentsCreateRequestPaymentMethodData
    payment_method: PaymentsCreateRequestPaymentMethod
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payment_token: String
    shipping: PaymentsCreateRequestShipping
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    @dataExamples([
        {
            json: "Hyperswitch Router"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_name: String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    @dataExamples([
        {
            json: "Payment for shoes purchase"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_suffix: String
    order_details: PaymentsCreateRequestOrderDetails
    mandate_data: PaymentsCreateRequestMandateData
    customer_acceptance: PaymentsCreateRequestCustomerAcceptance
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    @dataExamples([
        {
            json: "mandate_iwer89rnjef349dni3"
        }
    ])
    @length(
        max: 64
    )
    mandate_id: String
    browser_info: PaymentsCreateRequestBrowserInfo
    payment_experience: PaymentsCreateRequestPaymentExperience
    payment_method_type: PaymentsCreateRequestPaymentMethodType
    business_country: PaymentsCreateRequestBusinessCountry
    /// Business label of the merchant for this payment.
    /// To be deprecated soon. Pass the profile_id instead
    @dataExamples([
        {
            json: "food"
        }
    ])
    business_label: String
    merchant_connector_details: PaymentsCreateRequestMerchantConnectorDetails
    allowed_payment_method_types: PaymentsCreateRequestAllowedPaymentMethodTypes
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    connector_metadata: PaymentsCreateRequestConnectorMetadata
    /// Whether to generate the payment link for this payment or not (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    payment_link: Boolean
    payment_link_config: PaymentsCreateRequestPaymentLinkConfig
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: String
    surcharge_details: PaymentsCreateRequestSurchargeDetails
    payment_type: PaymentsCreateRequestPaymentType
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Additional data related to some frm(Fraud Risk Management) connectors
    frm_metadata: Document
    /// Whether to perform external authentication (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    request_external_three_ds_authentication: Boolean
    recurring_details: PaymentsCreateRequestRecurringDetails
    split_payments: PaymentsCreateRequestSplitPayments
    /// Optional boolean value to extent authorization period of this payment
    /// 
    /// capture method must be manual or manual_multiple
    request_extended_authorization: Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "Custom_Order_id_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: Boolean
    psd2_sca_exemption_type: PaymentsCreateRequestPsd2ScaExemptionType
    ctp_service_details: PaymentsCreateRequestCtpServiceDetails
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    threeds_method_comp_ind: PaymentsCreateRequestThreedsMethodCompInd
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: Boolean
    /// If enabled, provides whole connector response
    all_keys_required: Boolean
}

structure PaymentsCreateRequestAuthenticationType {}

structure PaymentsCreateRequestBrowserInfo with [BrowserInformation] {}

structure PaymentsCreateRequestBusinessCountry {}

structure PaymentsCreateRequestCaptureMethod {}

structure PaymentsCreateRequestConnectorMetadata with [ConnectorMetadata] {}

structure PaymentsCreateRequestCtpServiceDetails with [CtpServiceDetails] {}

structure PaymentsCreateRequestCustomerAcceptance with [CustomerAcceptance] {}

structure PaymentsCreateRequestMandateData with [MandateData] {}

structure PaymentsCreateRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsCreateRequestPaymentExperience {}

structure PaymentsCreateRequestPaymentLinkConfig with [PaymentCreatePaymentLinkConfig] {}

structure PaymentsCreateRequestPaymentMethod {}

structure PaymentsCreateRequestPaymentMethodData with [PaymentMethodDataRequest] {}

structure PaymentsCreateRequestPaymentMethodType {}

structure PaymentsCreateRequestPaymentType {}

structure PaymentsCreateRequestPsd2ScaExemptionType {}

structure PaymentsCreateRequestRecurringDetails {}

structure PaymentsCreateRequestRouting {}

structure PaymentsCreateRequestSetupFutureUsage {}

structure PaymentsCreateRequestSplitPayments {}

structure PaymentsCreateRequestSurchargeDetails with [RequestSurchargeDetails] {}

structure PaymentsCreateRequestThreedsMethodCompInd {}

structure PaymentsCreateResponseOpenApi {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payment_id: String
    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    @dataExamples([
        {
            json: "merchant_1668273825"
        }
    ])
    @length(
        max: 255
    )
    @required
    merchant_id: String
    @required
    status: PaymentsCreateResponseOpenApiStatus
    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    /// The payment net amount. net_amount = amount + surcharge_details.surcharge_amount + surcharge_details.tax_amount + shipping_cost + order_tax_amount,
    /// If no surcharge_details, shipping_cost, order_tax_amount, net_amount = amount
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    net_amount: Long
    /// The shipping cost for the payment.
    @dataExamples([
        {
            json: 6540
        }
    ])
    shipping_cost: Long
    /// The amount (in minor units) that can still be captured for this payment. This is relevant when `capture_method` is `manual`. Once fully captured, or if `capture_method` is `automatic` and payment succeeded, this will be 0.
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 100
    )
    @required
    amount_capturable: Long
    /// The total amount (in minor units) that has been captured for this payment. For `fauxpay` sandbox connector, this might reflect the authorized amount if `status` is `succeeded` even if `capture_method` was `manual`.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_received: Long
    /// The name of the payment connector (e.g., 'stripe', 'adyen') that processed or is processing this payment.
    @dataExamples([
        {
            json: "stripe"
        }
    ])
    connector: String
    /// A secret token unique to this payment intent. It is primarily used by client-side applications (e.g., Hyperswitch SDKs) to authenticate actions like confirming the payment or handling next actions. This secret should be handled carefully and not exposed publicly beyond its intended client-side use.
    @dataExamples([
        {
            json: "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    client_secret: String
    /// Timestamp indicating when this payment intent was created, in ISO 8601 format.
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
    @required
    currency: Currency
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    /// This field will be deprecated soon. Please refer to `customer.id`
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// An arbitrary string providing a description for the payment, often useful for display or internal record-keeping.
    @dataExamples([
        {
            json: "It's my first payment request"
        }
    ])
    description: String
    refunds: PaymentsCreateResponseOpenApiRefunds
    disputes: PaymentsCreateResponseOpenApiDisputes
    attempts: PaymentsCreateResponseOpenApiAttempts
    captures: PaymentsCreateResponseOpenApiCaptures
    /// A unique identifier to link the payment to a mandate, can be used instead of payment_method_data, in case of setting up recurring payments
    @dataExamples([
        {
            json: "mandate_iwer89rnjef349dni3"
        }
    ])
    @length(
        max: 255
    )
    mandate_id: String
    mandate_data: PaymentsCreateResponseOpenApiMandateData
    setup_future_usage: PaymentsCreateResponseOpenApiSetupFutureUsage
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with confirm=true.
    @dataExamples([
        {
            json: true
        }
    ])
    off_session: Boolean
    capture_method: PaymentsCreateResponseOpenApiCaptureMethod
    @required
    payment_method: PaymentMethod
    payment_method_data: PaymentsCreateResponseOpenApiPaymentMethodData
    /// Provide a reference to a stored payment method
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payment_token: String
    shipping: PaymentsCreateResponseOpenApiShipping
    billing: PaymentsCreateResponseOpenApiBilling
    order_details: PaymentsCreateResponseOpenApiOrderDetails
    /// description: The customer's email address
    /// This field will be deprecated soon. Please refer to `customer.email` object
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// description: The customer's name
    /// This field will be deprecated soon. Please refer to `customer.name` object
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's phone number
    /// This field will be deprecated soon. Please refer to `customer.phone` object
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    return_url: String
    authentication_type: PaymentsCreateResponseOpenApiAuthenticationType
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    @dataExamples([
        {
            json: "Hyperswitch Router"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_name: String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 255 characters for the concatenated descriptor.
    @dataExamples([
        {
            json: "Payment for shoes purchase"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_suffix: String
    next_action: PaymentsCreateResponseOpenApiNextAction
    /// If the payment intent was cancelled, this field provides a textual reason for the cancellation (e.g., "requested_by_customer", "abandoned").
    cancellation_reason: String
    /// The connector-specific error code from the last failed payment attempt associated with this payment intent.
    @dataExamples([
        {
            json: "E0001"
        }
    ])
    error_code: String
    /// A human-readable error message from the last failed payment attempt associated with this payment intent.
    @dataExamples([
        {
            json: "Failed while verifying the card"
        }
    ])
    error_message: String
    payment_experience: PaymentsCreateResponseOpenApiPaymentExperience
    payment_method_type: PaymentsCreateResponseOpenApiPaymentMethodType
    /// A label identifying the specific merchant connector account (MCA) used for this payment. This often combines the connector name, business country, and a custom label (e.g., "stripe_US_primary").
    @dataExamples([
        {
            json: "stripe_US_food"
        }
    ])
    connector_label: String
    business_country: PaymentsCreateResponseOpenApiBusinessCountry
    /// The label identifying the specific business unit or profile under which this payment was processed by the merchant.
    business_label: String
    /// An optional sub-label for further categorization of the business unit or profile used for this payment.
    business_sub_label: String
    allowed_payment_method_types: PaymentsCreateResponseOpenApiAllowedPaymentMethodTypes
    ephemeral_key: PaymentsCreateResponseOpenApiEphemeralKey
    /// If true the payment can be retried with same or different payment method which means the confirm call can be made again.
    manual_retry_allowed: Boolean
    /// A unique identifier for a payment provided by the connector
    @dataExamples([
        {
            json: "993672945374576J"
        }
    ])
    connector_transaction_id: String
    frm_message: PaymentsCreateResponseOpenApiFrmMessage
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    connector_metadata: PaymentsCreateResponseOpenApiConnectorMetadata
    feature_metadata: PaymentsCreateResponseOpenApiFeatureMetadata
    /// reference(Identifier) to the payment at connector side
    @dataExamples([
        {
            json: "993672945374576J"
        }
    ])
    reference_id: String
    payment_link: PaymentsCreateResponseOpenApiPaymentLink
    /// The business profile that is associated with this payment
    profile_id: String
    surcharge_details: PaymentsCreateResponseOpenApiSurchargeDetails
    /// Total number of attempts associated with this payment
    @required
    attempt_count: Integer
    /// Denotes the action(approve or reject) taken by merchant in case of manual review. Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    merchant_decision: String
    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    merchant_connector_id: String
    /// If true, incremental authorization can be performed on this payment, in case the funds authorized initially fall short.
    incremental_authorization_allowed: Boolean
    /// Total number of authorizations happened in an incremental_authorization payment
    authorization_count: Integer
    incremental_authorizations: PaymentsCreateResponseOpenApiIncrementalAuthorizations
    external_authentication_details: PaymentsCreateResponseOpenApiExternalAuthenticationDetails
    /// Flag indicating if external 3ds authentication is made or not
    external_3ds_authentication_attempted: Boolean
    /// Date Time for expiry of the payment
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    expires_on: Timestamp
    /// Payment Fingerprint, to identify a particular card.
    /// It is a 20 character long alphanumeric code.
    fingerprint: String
    browser_info: PaymentsCreateResponseOpenApiBrowserInfo
    /// A unique identifier for the payment method used in this payment. If the payment method was saved or tokenized, this ID can be used to reference it for future transactions or recurring payments.
    payment_method_id: String
    payment_method_status: PaymentsCreateResponseOpenApiPaymentMethodStatus
    /// Date time at which payment was updated
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    updated: Timestamp
    split_payments: PaymentsCreateResponseOpenApiSplitPayments
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. FRM Metadata is useful for storing additional, structured information on an object related to FRM.
    frm_metadata: Document
    /// flag that indicates if extended authorization is applied on this payment or not
    extended_authorization_applied: Boolean
    /// date and time after which this payment cannot be captured
    @timestampFormat("date-time")
    capture_before: Timestamp
    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    @dataExamples([
        {
            json: "Custom_Order_id_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    order_tax_amount: PaymentsCreateResponseOpenApiOrderTaxAmount
    /// Connector Identifier for the payment method
    connector_mandate_id: String
    card_discovery: PaymentsCreateResponseOpenApiCardDiscovery
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    /// Indicates if 3ds challenge is triggered
    force_3ds_challenge_trigger: Boolean
    /// Error code received from the issuer in case of failed payments
    issuer_error_code: String
    /// Error message received from the issuer in case of failed payments
    issuer_error_message: String
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: Boolean
    /// Contains whole connector response
    whole_connector_response: String
}

structure PaymentsCreateResponseOpenApiAuthenticationType {}

structure PaymentsCreateResponseOpenApiBrowserInfo with [BrowserInformation] {}

structure PaymentsCreateResponseOpenApiBusinessCountry {}

structure PaymentsCreateResponseOpenApiCaptureMethod {}

structure PaymentsCreateResponseOpenApiCardDiscovery {}

structure PaymentsCreateResponseOpenApiConnectorMetadata with [ConnectorMetadata] {}

structure PaymentsCreateResponseOpenApiEphemeralKey with [EphemeralKeyCreateResponse] {}

structure PaymentsCreateResponseOpenApiExternalAuthenticationDetails with [ExternalAuthenticationDetailsResponse] {}

structure PaymentsCreateResponseOpenApiFeatureMetadata with [FeatureMetadata] {}

structure PaymentsCreateResponseOpenApiFrmMessage with [FrmMessage] {}

structure PaymentsCreateResponseOpenApiMandateData with [MandateData] {}

structure PaymentsCreateResponseOpenApiNextAction {}

structure PaymentsCreateResponseOpenApiOrderTaxAmount {}

structure PaymentsCreateResponseOpenApiPaymentExperience {}

structure PaymentsCreateResponseOpenApiPaymentLink with [PaymentLinkResponse] {}

structure PaymentsCreateResponseOpenApiPaymentMethodData with [PaymentMethodDataResponseWithBilling] {}

structure PaymentsCreateResponseOpenApiPaymentMethodStatus {}

structure PaymentsCreateResponseOpenApiPaymentMethodType {}

structure PaymentsCreateResponseOpenApiSetupFutureUsage {}

structure PaymentsCreateResponseOpenApiSplitPayments {}

structure PaymentsCreateResponseOpenApiStatus {}

structure PaymentsCreateResponseOpenApiSurchargeDetails with [RequestSurchargeDetails] {}

structure PaymentsDynamicTaxCalculationRequest {
    @required
    shipping: Address
    /// Client Secret
    @required
    client_secret: String
    @required
    payment_method_type: PaymentMethodType
    /// Session Id
    session_id: String
}

structure PaymentsDynamicTaxCalculationResponse {
    /// The identifier for the payment
    @required
    payment_id: String
    @required
    net_amount: MinorUnit
    order_tax_amount: PaymentsDynamicTaxCalculationResponseOrderTaxAmount
    shipping_cost: ShippingCost
    @required
    display_amount: DisplayAmountOnSdk
}

structure PaymentsDynamicTaxCalculationResponseOrderTaxAmount {}

structure PaymentsExternalAuthenticationRequest {
    /// Client Secret
    @required
    client_secret: String
    sdk_information: SdkInformation
    @required
    device_channel: DeviceChannel
    @required
    threeds_method_comp_ind: ThreeDsCompletionIndicator
}

structure PaymentsExternalAuthenticationResponse {
    @required
    trans_status: TransactionStatus
    /// Access Server URL to be used for challenge submission
    acs_url: String
    /// Challenge request which should be sent to acs_url
    challenge_request: String
    /// Unique identifier assigned by the EMVCo(Europay, Mastercard and Visa)
    acs_reference_number: String
    /// Unique identifier assigned by the ACS to identify a single transaction
    acs_trans_id: String
    /// Unique identifier assigned by the 3DS Server to identify a single transaction
    three_dsserver_trans_id: String
    /// Contains the JWS object created by the ACS for the ARes(Authentication Response) message
    acs_signed_content: String
    /// Three DS Requestor URL
    @required
    three_ds_requestor_url: String
    /// Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred
    three_ds_requestor_app_url: String
}

structure PaymentsIncrementalAuthorizationRequest {
    /// The total amount including previously authorized amount and additional amount
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    /// Reason for incremental authorization
    reason: String
}

structure PaymentsPostSessionTokensRequest {
    /// It's a token used for client side verification.
    @required
    client_secret: String
    @required
    payment_method_type: PaymentMethodType
    @required
    payment_method: PaymentMethod
}

structure PaymentsPostSessionTokensResponse {
    /// The identifier for the payment
    @required
    payment_id: String
    next_action: PaymentsPostSessionTokensResponseNextAction
    @required
    status: PaymentsPostSessionTokensResponseStatus
}

structure PaymentsPostSessionTokensResponseNextAction {}

structure PaymentsPostSessionTokensResponseStatus {}

structure PaymentsRequest {
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 0
    )
    amount: Long
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    @dataExamples([
        {
            json: 6540
        }
    ])
    order_tax_amount: Long
    currency: PaymentsRequestCurrency
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_to_capture: Long
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    @dataExamples([
        {
            json: 6540
        }
    ])
    shipping_cost: Long
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    payment_id: String
    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    @dataExamples([
        {
            json: "merchant_1668273825"
        }
    ])
    @length(
        max: 255
    )
    merchant_id: String
    routing: PaymentsRequestRouting
    connector: PaymentsRequestConnector
    capture_method: PaymentsRequestCaptureMethod
    authentication_type: PaymentsRequestAuthenticationType
    billing: PaymentsRequestBilling
    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    capture_on: Timestamp
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    @dataExamples([
        {
            json: true
        }
    ])
    confirm: Boolean
    customer: PaymentsRequestCustomer
    /// The identifier for the customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// The customer's email address.
    /// This field will be deprecated soon, use the customer object instead
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// The customer's name.
    /// This field will be deprecated soon, use the customer object instead.
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's phone number
    /// This field will be deprecated soon, use the customer object instead
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// The country code for the customer phone number
    /// This field will be deprecated soon, use the customer object instead
    @dataExamples([
        {
            json: "+1"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    @dataExamples([
        {
            json: true
        }
    ])
    off_session: Boolean
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    @dataExamples([
        {
            json: "It's my first payment request"
        }
    ])
    description: String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    @length(
        max: 2048
    )
    return_url: String
    setup_future_usage: PaymentsRequestSetupFutureUsage
    payment_method_data: PaymentsRequestPaymentMethodData
    payment_method: PaymentsRequestPaymentMethod
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payment_token: String
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: String
    shipping: PaymentsRequestShipping
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    @dataExamples([
        {
            json: "Hyperswitch Router"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_name: String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    @dataExamples([
        {
            json: "Payment for shoes purchase"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_suffix: String
    order_details: PaymentsRequestOrderDetails
    /// It's a token used for client side verification.
    @dataExamples([
        {
            json: "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    client_secret: String
    mandate_data: PaymentsRequestMandateData
    customer_acceptance: PaymentsRequestCustomerAcceptance
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    @dataExamples([
        {
            json: "mandate_iwer89rnjef349dni3"
        }
    ])
    @length(
        max: 64
    )
    mandate_id: String
    browser_info: PaymentsRequestBrowserInfo
    payment_experience: PaymentsRequestPaymentExperience
    payment_method_type: PaymentsRequestPaymentMethodType
    business_country: PaymentsRequestBusinessCountry
    /// Business label of the merchant for this payment.
    /// To be deprecated soon. Pass the profile_id instead
    @dataExamples([
        {
            json: "food"
        }
    ])
    business_label: String
    merchant_connector_details: PaymentsRequestMerchantConnectorDetails
    allowed_payment_method_types: PaymentsRequestAllowedPaymentMethodTypes
    /// Business sub label for the payment
    business_sub_label: String
    retry_action: PaymentsRequestRetryAction
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    connector_metadata: PaymentsRequestConnectorMetadata
    feature_metadata: PaymentsRequestFeatureMetadata
    /// Whether to generate the payment link for this payment or not (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    payment_link: Boolean
    payment_link_config: PaymentsRequestPaymentLinkConfig
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: String
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: String
    surcharge_details: PaymentsRequestSurchargeDetails
    payment_type: PaymentsRequestPaymentType
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Additional data related to some frm(Fraud Risk Management) connectors
    frm_metadata: Document
    /// Whether to perform external authentication (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    request_external_three_ds_authentication: Boolean
    recurring_details: PaymentsRequestRecurringDetails
    split_payments: PaymentsRequestSplitPayments
    /// Optional boolean value to extent authorization period of this payment
    /// 
    /// capture method must be manual or manual_multiple
    request_extended_authorization: Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "Custom_Order_id_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: Boolean
    psd2_sca_exemption_type: PaymentsRequestPsd2ScaExemptionType
    ctp_service_details: PaymentsRequestCtpServiceDetails
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    threeds_method_comp_ind: PaymentsRequestThreedsMethodCompInd
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: Boolean
    /// If enabled, provides whole connector response
    all_keys_required: Boolean
}

structure PaymentsRequestAuthenticationType {}

structure PaymentsRequestBrowserInfo with [BrowserInformation] {}

structure PaymentsRequestBusinessCountry {}

structure PaymentsRequestCaptureMethod {}

structure PaymentsRequestConnectorMetadata with [ConnectorMetadata] {}

structure PaymentsRequestCtpServiceDetails with [CtpServiceDetails] {}

structure PaymentsRequestCurrency {}

structure PaymentsRequestCustomerAcceptance with [CustomerAcceptance] {}

structure PaymentsRequestFeatureMetadata with [FeatureMetadata] {}

structure PaymentsRequestMandateData with [MandateData] {}

structure PaymentsRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsRequestPaymentExperience {}

structure PaymentsRequestPaymentLinkConfig with [PaymentCreatePaymentLinkConfig] {}

structure PaymentsRequestPaymentMethod {}

structure PaymentsRequestPaymentMethodData with [PaymentMethodDataRequest] {}

structure PaymentsRequestPaymentMethodType {}

structure PaymentsRequestPaymentType {}

structure PaymentsRequestPsd2ScaExemptionType {}

structure PaymentsRequestRecurringDetails {}

structure PaymentsRequestRetryAction {}

structure PaymentsRequestRouting {}

structure PaymentsRequestSetupFutureUsage {}

structure PaymentsRequestSplitPayments {}

structure PaymentsRequestSurchargeDetails with [RequestSurchargeDetails] {}

structure PaymentsRequestThreedsMethodCompInd {}

structure PaymentsResponse {
    /// Unique identifier for the payment. This ensures idempotency for multiple payments
    /// that have been done by a single merchant.
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payment_id: String
    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    @dataExamples([
        {
            json: "merchant_1668273825"
        }
    ])
    @length(
        max: 255
    )
    @required
    merchant_id: String
    @required
    status: PaymentsResponseStatus
    /// The payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    /// The payment net amount. net_amount = amount + surcharge_details.surcharge_amount + surcharge_details.tax_amount + shipping_cost + order_tax_amount,
    /// If no surcharge_details, shipping_cost, order_tax_amount, net_amount = amount
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    net_amount: Long
    /// The shipping cost for the payment.
    @dataExamples([
        {
            json: 6540
        }
    ])
    shipping_cost: Long
    /// The amount (in minor units) that can still be captured for this payment. This is relevant when `capture_method` is `manual`. Once fully captured, or if `capture_method` is `automatic` and payment succeeded, this will be 0.
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 100
    )
    @required
    amount_capturable: Long
    /// The total amount (in minor units) that has been captured for this payment. For `fauxpay` sandbox connector, this might reflect the authorized amount if `status` is `succeeded` even if `capture_method` was `manual`.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_received: Long
    /// The name of the payment connector (e.g., 'stripe', 'adyen') that processed or is processing this payment.
    @dataExamples([
        {
            json: "stripe"
        }
    ])
    connector: String
    /// A secret token unique to this payment intent. It is primarily used by client-side applications (e.g., Hyperswitch SDKs) to authenticate actions like confirming the payment or handling next actions. This secret should be handled carefully and not exposed publicly beyond its intended client-side use.
    @dataExamples([
        {
            json: "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    client_secret: String
    /// Timestamp indicating when this payment intent was created, in ISO 8601 format.
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
    @required
    currency: Currency
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    /// This field will be deprecated soon. Please refer to `customer.id`
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    customer: PaymentsResponseCustomer
    /// An arbitrary string providing a description for the payment, often useful for display or internal record-keeping.
    @dataExamples([
        {
            json: "It's my first payment request"
        }
    ])
    description: String
    refunds: PaymentsResponseRefunds
    disputes: PaymentsResponseDisputes
    attempts: PaymentsResponseAttempts
    captures: PaymentsResponseCaptures
    /// A unique identifier to link the payment to a mandate, can be used instead of payment_method_data, in case of setting up recurring payments
    @dataExamples([
        {
            json: "mandate_iwer89rnjef349dni3"
        }
    ])
    @length(
        max: 255
    )
    mandate_id: String
    mandate_data: PaymentsResponseMandateData
    setup_future_usage: PaymentsResponseSetupFutureUsage
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. This parameter can only be used with confirm=true.
    @dataExamples([
        {
            json: true
        }
    ])
    off_session: Boolean
    /// A timestamp (ISO 8601 code) that determines when the payment should be captured.
    /// Providing this field will automatically set `capture` to true
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    capture_on: Timestamp
    capture_method: PaymentsResponseCaptureMethod
    @required
    payment_method: PaymentMethod
    payment_method_data: PaymentsResponsePaymentMethodData
    /// Provide a reference to a stored payment method
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payment_token: String
    shipping: PaymentsResponseShipping
    billing: PaymentsResponseBilling
    order_details: PaymentsResponseOrderDetails
    /// description: The customer's email address
    /// This field will be deprecated soon. Please refer to `customer.email` object
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// description: The customer's name
    /// This field will be deprecated soon. Please refer to `customer.name` object
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// The customer's phone number
    /// This field will be deprecated soon. Please refer to `customer.phone` object
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    return_url: String
    authentication_type: PaymentsResponseAuthenticationType
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    @dataExamples([
        {
            json: "Hyperswitch Router"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_name: String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 255 characters for the concatenated descriptor.
    @dataExamples([
        {
            json: "Payment for shoes purchase"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_suffix: String
    next_action: PaymentsResponseNextAction
    /// If the payment intent was cancelled, this field provides a textual reason for the cancellation (e.g., "requested_by_customer", "abandoned").
    cancellation_reason: String
    /// The connector-specific error code from the last failed payment attempt associated with this payment intent.
    @dataExamples([
        {
            json: "E0001"
        }
    ])
    error_code: String
    /// A human-readable error message from the last failed payment attempt associated with this payment intent.
    @dataExamples([
        {
            json: "Failed while verifying the card"
        }
    ])
    error_message: String
    /// error code unified across the connectors is received here if there was an error while calling connector
    unified_code: String
    /// error message unified across the connectors is received here if there was an error while calling connector
    unified_message: String
    payment_experience: PaymentsResponsePaymentExperience
    payment_method_type: PaymentsResponsePaymentMethodType
    /// A label identifying the specific merchant connector account (MCA) used for this payment. This often combines the connector name, business country, and a custom label (e.g., "stripe_US_primary").
    @dataExamples([
        {
            json: "stripe_US_food"
        }
    ])
    connector_label: String
    business_country: PaymentsResponseBusinessCountry
    /// The label identifying the specific business unit or profile under which this payment was processed by the merchant.
    business_label: String
    /// An optional sub-label for further categorization of the business unit or profile used for this payment.
    business_sub_label: String
    allowed_payment_method_types: PaymentsResponseAllowedPaymentMethodTypes
    ephemeral_key: PaymentsResponseEphemeralKey
    /// If true the payment can be retried with same or different payment method which means the confirm call can be made again.
    manual_retry_allowed: Boolean
    /// A unique identifier for a payment provided by the connector
    @dataExamples([
        {
            json: "993672945374576J"
        }
    ])
    connector_transaction_id: String
    frm_message: PaymentsResponseFrmMessage
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    connector_metadata: PaymentsResponseConnectorMetadata
    feature_metadata: PaymentsResponseFeatureMetadata
    /// reference(Identifier) to the payment at connector side
    @dataExamples([
        {
            json: "993672945374576J"
        }
    ])
    reference_id: String
    payment_link: PaymentsResponsePaymentLink
    /// The business profile that is associated with this payment
    profile_id: String
    surcharge_details: PaymentsResponseSurchargeDetails
    /// Total number of attempts associated with this payment
    @required
    attempt_count: Integer
    /// Denotes the action(approve or reject) taken by merchant in case of manual review. Manual review can occur when the transaction is marked as risky by the frm_processor, payment processor or when there is underpayment/over payment incase of crypto payment
    merchant_decision: String
    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    merchant_connector_id: String
    /// If true, incremental authorization can be performed on this payment, in case the funds authorized initially fall short.
    incremental_authorization_allowed: Boolean
    /// Total number of authorizations happened in an incremental_authorization payment
    authorization_count: Integer
    incremental_authorizations: PaymentsResponseIncrementalAuthorizations
    external_authentication_details: PaymentsResponseExternalAuthenticationDetails
    /// Flag indicating if external 3ds authentication is made or not
    external_3ds_authentication_attempted: Boolean
    /// Date Time for expiry of the payment
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    expires_on: Timestamp
    /// Payment Fingerprint, to identify a particular card.
    /// It is a 20 character long alphanumeric code.
    fingerprint: String
    browser_info: PaymentsResponseBrowserInfo
    /// A unique identifier for the payment method used in this payment. If the payment method was saved or tokenized, this ID can be used to reference it for future transactions or recurring payments.
    payment_method_id: String
    payment_method_status: PaymentsResponsePaymentMethodStatus
    /// Date time at which payment was updated
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    updated: Timestamp
    split_payments: PaymentsResponseSplitPayments
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. FRM Metadata is useful for storing additional, structured information on an object related to FRM.
    frm_metadata: Document
    /// flag that indicates if extended authorization is applied on this payment or not
    extended_authorization_applied: Boolean
    /// date and time after which this payment cannot be captured
    @timestampFormat("date-time")
    capture_before: Timestamp
    /// Merchant's identifier for the payment/invoice. This will be sent to the connector
    /// if the connector provides support to accept multiple reference ids.
    /// In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
    @dataExamples([
        {
            json: "Custom_Order_id_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    order_tax_amount: PaymentsResponseOrderTaxAmount
    /// Connector Identifier for the payment method
    connector_mandate_id: String
    card_discovery: PaymentsResponseCardDiscovery
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    /// Indicates if 3ds challenge is triggered
    force_3ds_challenge_trigger: Boolean
    /// Error code received from the issuer in case of failed payments
    issuer_error_code: String
    /// Error message received from the issuer in case of failed payments
    issuer_error_message: String
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: Boolean
    /// Contains whole connector response
    whole_connector_response: String
}

structure PaymentsResponseAuthenticationType {}

structure PaymentsResponseBrowserInfo with [BrowserInformation] {}

structure PaymentsResponseBusinessCountry {}

structure PaymentsResponseCaptureMethod {}

structure PaymentsResponseCardDiscovery {}

structure PaymentsResponseConnectorMetadata with [ConnectorMetadata] {}

structure PaymentsResponseCustomer with [CustomerDetailsResponse] {}

structure PaymentsResponseEphemeralKey with [EphemeralKeyCreateResponse] {}

structure PaymentsResponseExternalAuthenticationDetails with [ExternalAuthenticationDetailsResponse] {}

structure PaymentsResponseFeatureMetadata with [FeatureMetadata] {}

structure PaymentsResponseFrmMessage with [FrmMessage] {}

structure PaymentsResponseMandateData with [MandateData] {}

structure PaymentsResponseNextAction {}

structure PaymentsResponseOrderTaxAmount {}

structure PaymentsResponsePaymentExperience {}

structure PaymentsResponsePaymentLink with [PaymentLinkResponse] {}

structure PaymentsResponsePaymentMethodData with [PaymentMethodDataResponseWithBilling] {}

structure PaymentsResponsePaymentMethodStatus {}

structure PaymentsResponsePaymentMethodType {}

structure PaymentsResponseSetupFutureUsage {}

structure PaymentsResponseSplitPayments {}

structure PaymentsResponseStatus {}

structure PaymentsResponseSurchargeDetails with [RequestSurchargeDetails] {}

structure PaymentsRetrieveRequest {
    /// The type of ID (ex: payment intent id, payment attempt id or connector txn id)
    @required
    resource_id: String
    /// The identifier for the Merchant Account.
    merchant_id: String
    /// Decider to enable or disable the connector call for retrieve request
    @required
    force_sync: Boolean
    /// Optional query parameters that might be specific to a connector or flow, passed through during the retrieve operation. Use with caution and refer to specific connector documentation if applicable.
    param: String
    /// Optionally specifies the connector to be used for a 'force_sync' retrieve operation. If provided, Hyperswitch will attempt to sync the payment status from this specific connector.
    connector: String
    merchant_connector_details: PaymentsRetrieveRequestMerchantConnectorDetails
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    client_secret: String
    /// If enabled provides list of captures linked to latest attempt
    expand_captures: Boolean
    /// If enabled provides list of attempts linked to payment intent
    expand_attempts: Boolean
    /// If enabled, provides whole connector response
    all_keys_required: Boolean
}

structure PaymentsRetrieveRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsSessionRequest {
    /// The identifier for the payment
    @required
    payment_id: String
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @required
    client_secret: String
    @required
    wallets: Wallets
    merchant_connector_details: PaymentsSessionRequestMerchantConnectorDetails
}

structure PaymentsSessionRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsSessionResponse {
    /// The identifier for the payment
    @required
    payment_id: String
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @required
    client_secret: String
    @required
    session_token: PaymentsSessionResponseSessionToken
}

structure PaymentsUpdateMetadataRequest {
    /// Metadata is useful for storing additional, unstructured information on an object.
    @required
    metadata: Document
}

structure PaymentsUpdateMetadataResponse {
    /// The identifier for the payment
    @required
    payment_id: String
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
}

structure PaymentsUpdateRequest {
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 0
    )
    amount: Long
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    @dataExamples([
        {
            json: 6540
        }
    ])
    order_tax_amount: Long
    currency: PaymentsUpdateRequestCurrency
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    @dataExamples([
        {
            json: 6540
        }
    ])
    amount_to_capture: Long
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    @dataExamples([
        {
            json: 6540
        }
    ])
    shipping_cost: Long
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    payment_id: String
    routing: PaymentsUpdateRequestRouting
    connector: PaymentsUpdateRequestConnector
    capture_method: PaymentsUpdateRequestCaptureMethod
    authentication_type: PaymentsUpdateRequestAuthenticationType
    billing: PaymentsUpdateRequestBilling
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    @dataExamples([
        {
            json: true
        }
    ])
    confirm: Boolean
    customer: PaymentsUpdateRequestCustomer
    /// The identifier for the customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        min: 1
        max: 64
    )
    customer_id: String
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    @dataExamples([
        {
            json: true
        }
    ])
    off_session: Boolean
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    @dataExamples([
        {
            json: "It's my first payment request"
        }
    ])
    description: String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    @length(
        max: 2048
    )
    return_url: String
    setup_future_usage: PaymentsUpdateRequestSetupFutureUsage
    payment_method_data: PaymentsUpdateRequestPaymentMethodData
    payment_method: PaymentsUpdateRequestPaymentMethod
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payment_token: String
    shipping: PaymentsUpdateRequestShipping
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    @dataExamples([
        {
            json: "Hyperswitch Router"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_name: String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    @dataExamples([
        {
            json: "Payment for shoes purchase"
        }
    ])
    @length(
        max: 255
    )
    statement_descriptor_suffix: String
    order_details: PaymentsUpdateRequestOrderDetails
    mandate_data: PaymentsUpdateRequestMandateData
    customer_acceptance: PaymentsUpdateRequestCustomerAcceptance
    browser_info: PaymentsUpdateRequestBrowserInfo
    payment_experience: PaymentsUpdateRequestPaymentExperience
    payment_method_type: PaymentsUpdateRequestPaymentMethodType
    merchant_connector_details: PaymentsUpdateRequestMerchantConnectorDetails
    allowed_payment_method_types: PaymentsUpdateRequestAllowedPaymentMethodTypes
    retry_action: PaymentsUpdateRequestRetryAction
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    connector_metadata: PaymentsUpdateRequestConnectorMetadata
    /// Whether to generate the payment link for this payment or not (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    payment_link: Boolean
    payment_link_config: PaymentsUpdateRequestPaymentLinkConfig
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: String
    surcharge_details: PaymentsUpdateRequestSurchargeDetails
    payment_type: PaymentsUpdateRequestPaymentType
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: Boolean
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Additional data related to some frm(Fraud Risk Management) connectors
    frm_metadata: Document
    /// Whether to perform external authentication (if applicable)
    @dataExamples([
        {
            json: true
        }
    ])
    request_external_three_ds_authentication: Boolean
    recurring_details: PaymentsUpdateRequestRecurringDetails
    split_payments: PaymentsUpdateRequestSplitPayments
    /// Optional boolean value to extent authorization period of this payment
    /// 
    /// capture method must be manual or manual_multiple
    request_extended_authorization: Boolean
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "Custom_Order_id_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: Boolean
    psd2_sca_exemption_type: PaymentsUpdateRequestPsd2ScaExemptionType
    ctp_service_details: PaymentsUpdateRequestCtpServiceDetails
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    threeds_method_comp_ind: PaymentsUpdateRequestThreedsMethodCompInd
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: Boolean
    /// If enabled, provides whole connector response
    all_keys_required: Boolean
}

structure PaymentsUpdateRequestAuthenticationType {}

structure PaymentsUpdateRequestBrowserInfo with [BrowserInformation] {}

structure PaymentsUpdateRequestCaptureMethod {}

structure PaymentsUpdateRequestConnectorMetadata with [ConnectorMetadata] {}

structure PaymentsUpdateRequestCtpServiceDetails with [CtpServiceDetails] {}

structure PaymentsUpdateRequestCurrency {}

structure PaymentsUpdateRequestCustomerAcceptance with [CustomerAcceptance] {}

structure PaymentsUpdateRequestMandateData with [MandateData] {}

structure PaymentsUpdateRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure PaymentsUpdateRequestPaymentExperience {}

structure PaymentsUpdateRequestPaymentLinkConfig with [PaymentCreatePaymentLinkConfig] {}

structure PaymentsUpdateRequestPaymentMethod {}

structure PaymentsUpdateRequestPaymentMethodData with [PaymentMethodDataRequest] {}

structure PaymentsUpdateRequestPaymentMethodType {}

structure PaymentsUpdateRequestPaymentType {}

structure PaymentsUpdateRequestPsd2ScaExemptionType {}

structure PaymentsUpdateRequestRecurringDetails {}

structure PaymentsUpdateRequestRetryAction {}

structure PaymentsUpdateRequestRouting {}

structure PaymentsUpdateRequestSetupFutureUsage {}

structure PaymentsUpdateRequestSplitPayments {}

structure PaymentsUpdateRequestSurchargeDetails with [RequestSurchargeDetails] {}

structure PaymentsUpdateRequestThreedsMethodCompInd {}

structure PayoutAttemptResponse {
    /// Unique identifier for the attempt
    @required
    attempt_id: String
    @required
    status: PayoutStatus
    /// The payout attempt amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 6583
        }
    ])
    @required
    amount: Long
    currency: PayoutAttemptResponseCurrency
    /// The connector used for the payout
    connector: String
    /// Connector's error code in case of failures
    error_code: String
    /// Connector's error message in case of failures
    error_message: String
    payment_method: PayoutAttemptResponsePaymentMethod
    payout_method_type: PayoutMethodType
    /// A unique identifier for a payout provided by the connector
    connector_transaction_id: String
    /// If the payout was cancelled the reason provided here
    cancellation_reason: String
    /// (This field is not live yet)
    /// Error code unified across the connectors is received here in case of errors while calling the underlying connector
    @dataExamples([
        {
            json: "UE_000"
        }
    ])
    @length(
        max: 255
    )
    unified_code: String
    /// (This field is not live yet)
    /// Error message unified across the connectors is received here in case of errors while calling the underlying connector
    @dataExamples([
        {
            json: "Invalid card details"
        }
    ])
    @length(
        max: 1024
    )
    unified_message: String
}

structure PayoutAttemptResponseCurrency {}

structure PayoutAttemptResponsePaymentMethod {}

structure PayoutCancelRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payout_id: String
}

structure PayoutConfirmRequest {
    /// Your unique identifier for this payout or order. This ID helps you reconcile payouts on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "merchant_order_ref_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 1000
        }
    ])
    @range(
        min: 0
    )
    amount: Long
    currency: PayoutConfirmRequestCurrency
    routing: PayoutConfirmRequestRouting
    connector: PayoutConfirmRequestConnector
    payout_type: PayoutConfirmRequestPayoutType
    payout_method_data: PayoutConfirmRequestPayoutMethodData
    billing: PayoutConfirmRequestBilling
    /// Set to true to confirm the payout without review, no further action required
    @dataExamples([
        {
            json: true
        }
    ])
    auto_fulfill: Boolean
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated. _Deprecated: Use customer_id instead._
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    customer_id: String
    customer: PayoutConfirmRequestCustomer
    /// It's a token used for client side verification.
    @required
    client_secret: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    return_url: String
    business_country: PayoutConfirmRequestBusinessCountry
    /// Business label of the merchant for this payout. _Deprecated: Use profile_id instead._
    @dataExamples([
        {
            json: "food"
        }
    ])
    business_label: String
    /// A description of the payout
    @dataExamples([
        {
            json: "It's my first payout request"
        }
    ])
    description: String
    entity_type: PayoutConfirmRequestEntityType
    /// Specifies whether or not the payout request is recurring
    recurring: Boolean
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// Provide a reference to a stored payout method, used to process the payout.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payout_token: String
    /// The business profile to use for this payout, especially if there are multiple business profiles associated with the account, otherwise default business profile associated with the merchant account will be used.
    profile_id: String
    priority: PayoutConfirmRequestPriority
    /// Whether to get the payout link (if applicable). Merchant need to specify this during the Payout _Create_, this field can not be updated during Payout _Update_.
    @dataExamples([
        {
            json: true
        }
    ])
    payout_link: Boolean
    payout_link_config: PayoutConfirmRequestPayoutLinkConfig
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Customer's email. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// Customer's name. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// Customer's phone. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// Customer's phone country code. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "+1"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    /// Identifier for payout method
    payout_method_id: String
}

structure PayoutConfirmRequestBusinessCountry {}

structure PayoutConfirmRequestCurrency {}

structure PayoutConfirmRequestEntityType {}

structure PayoutConfirmRequestPayoutLinkConfig with [PayoutCreatePayoutLinkConfig] {}

structure PayoutConfirmRequestPayoutMethodData {}

structure PayoutConfirmRequestPayoutType {}

structure PayoutConfirmRequestPriority {}

structure PayoutConfirmRequestRouting {}

/// Custom payout link config for the particular payout, if payout link is to be generated.
@mixin
structure PayoutCreatePayoutLinkConfig {
    /// The unique identifier for the collect link.
    @dataExamples([
        {
            json: "pm_collect_link_2bdacf398vwzq5n422S1"
        }
    ])
    payout_link_id: String
    enabled_payment_methods: PayoutCreatePayoutLinkConfigAllOf1EnabledPaymentMethods
    form_layout: PayoutCreatePayoutLinkConfigAllOf1FormLayout
    /// `test_mode` allows for opening payout links without any restrictions. This removes
    /// - domain name validations
    /// - check for making sure link is accessed within an iframe
    @dataExamples([
        {
            json: false
        }
    ])
    test_mode: Boolean
}

structure PayoutCreatePayoutLinkConfigAllOf1FormLayout {}

structure PayoutCreateResponse {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payout_id: String
    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    @dataExamples([
        {
            json: "merchant_1668273825"
        }
    ])
    @length(
        max: 255
    )
    @required
    merchant_id: String
    /// Your unique identifier for this payout or order. This ID helps you reconcile payouts on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "merchant_order_ref_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 1000
        }
    ])
    @required
    amount: Long
    @required
    currency: Currency
    /// The connector used for the payout
    @dataExamples([
        {
            json: "wise"
        }
    ])
    connector: String
    payout_type: PayoutCreateResponsePayoutType
    payout_method_data: PayoutCreateResponsePayoutMethodData
    billing: PayoutCreateResponseBilling
    /// Set to true to confirm the payout without review, no further action required
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    auto_fulfill: Boolean
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated.
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    @required
    customer_id: String
    customer: PayoutCreateResponseCustomer
    /// It's a token used for client side verification.
    @dataExamples([
        {
            json: "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    @required
    client_secret: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    @required
    return_url: String
    @required
    business_country: CountryAlpha2
    /// Business label of the merchant for this payout
    @dataExamples([
        {
            json: "food"
        }
    ])
    business_label: String
    /// A description of the payout
    @dataExamples([
        {
            json: "It's my first payout request"
        }
    ])
    description: String
    @required
    entity_type: PayoutEntityType
    /// Specifies whether or not the payout request is recurring
    @required
    recurring: Boolean
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// Unique identifier of the merchant connector account
    @dataExamples([
        {
            json: "mca_sAD3OZLATetvjLOYhUSy"
        }
    ])
    merchant_connector_id: String
    @required
    status: PayoutStatus
    /// If there was an error while calling the connector the error message is received here
    @dataExamples([
        {
            json: "Failed while verifying the card"
        }
    ])
    error_message: String
    /// If there was an error while calling the connectors the code is received here
    @dataExamples([
        {
            json: "E0001"
        }
    ])
    error_code: String
    /// The business profile that is associated with this payout
    @required
    profile_id: String
    /// Time when the payout was created
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
    /// Underlying processor's payout resource ID
    @dataExamples([
        {
            json: "S3FC9G9M2MVFDXT5"
        }
    ])
    connector_transaction_id: String
    priority: PayoutCreateResponsePriority
    attempts: PayoutCreateResponseAttempts
    payout_link: PayoutLink
    /// Customer's email. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// Customer's name. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// Customer's phone. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// Customer's phone country code. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "+1"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    /// (This field is not live yet)
    /// Error code unified across the connectors is received here in case of errors while calling the underlying connector
    @dataExamples([
        {
            json: "UE_000"
        }
    ])
    @length(
        max: 255
    )
    unified_code: String
    /// (This field is not live yet)
    /// Error message unified across the connectors is received here in case of errors while calling the underlying connector
    @dataExamples([
        {
            json: "Invalid card details"
        }
    ])
    @length(
        max: 1024
    )
    unified_message: String
    /// Identifier for payout method
    payout_method_id: String
}

structure PayoutCreateResponseCustomer with [CustomerDetailsResponse] {}

structure PayoutCreateResponsePayoutMethodData {}

structure PayoutCreateResponsePayoutType {}

structure PayoutCreateResponsePriority {}

structure PayoutFulfillRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payout_id: String
}

structure PayoutLink with [PayoutLinkResponse] {}

structure PayoutLinkInitiateRequest {
    @required
    merchant_id: String
    @required
    payout_id: String
}

@mixin
structure PayoutLinkResponse {
    @required
    payout_link_id: String
    @required
    link: String
}

structure PayoutListConstraints {
    /// The identifier for customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    customer_id: String
    /// A cursor for use in pagination, fetch the next list after some object
    @dataExamples([
        {
            json: "payout_fafa124123"
        }
    ])
    starting_after: String
    /// A cursor for use in pagination, fetch the previous list before some object
    @dataExamples([
        {
            json: "payout_fafa124123"
        }
    ])
    ending_before: String
    /// limit on the number of objects to return
    @range(
        min: 0
        max: 100
    )
    limit: Integer
    /// The time at which payout is created
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @timestampFormat("date-time")
    created: Timestamp
}

structure PayoutListFilterConstraints {
    /// The identifier for payout
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    payout_id: String
    /// The merchant order reference ID for payout
    @dataExamples([
        {
            json: "merchant_order_ref_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// The identifier for business profile
    profile_id: String
    /// The identifier for customer
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    customer_id: String
    /// The limit on the number of objects. The default limit is 10 and max limit is 20
    @range(
        min: 0
    )
    limit: Integer
    /// The starting point within a list of objects
    @range(
        min: 0
    )
    offset: Integer
    connector: PayoutListFilterConstraintsAllOf1Connector
    @required
    currency: Currency
    status: PayoutListFilterConstraintsAllOf1Status
    payout_method: PayoutListFilterConstraintsAllOf1PayoutMethod
    @required
    entity_type: PayoutEntityType
}

structure PayoutListFilters {
    @required
    connector: PayoutListFiltersConnector
    @required
    currency: PayoutListFiltersCurrency
    @required
    status: PayoutListFiltersStatus
    @required
    payout_method: PayoutListFiltersPayoutMethod
}

structure PayoutListResponse {
    /// The number of payouts included in the list
    @range(
        min: 0
    )
    @required
    size: Integer
    @required
    data: PayoutListResponseData
    /// The total number of available payouts for given constraints
    total_count: Long
}

structure PayoutMethodDataOneOfAlt0 {
    @required
    card: CardPayout
}

structure PayoutMethodDataOneOfAlt1 {
    @required
    bank: Bank
}

structure PayoutMethodDataOneOfAlt2 {
    @required
    wallet: Wallet
}

structure PayoutMethodDataResponseOneOfAlt0 {
    @required
    card: CardAdditionalData
}

structure PayoutMethodDataResponseOneOfAlt1 {
    @required
    bank: BankAdditionalData
}

structure PayoutMethodDataResponseOneOfAlt2 {
    @required
    wallet: WalletAdditionalData
}

structure PayoutMethodType {}

structure PayoutRetrieveBody {
    force_sync: Boolean
    merchant_id: String
}

structure PayoutRetrieveRequest {
    /// Unique identifier for the payout. This ensures idempotency for multiple payouts
    /// that have been done by a single merchant. This field is auto generated and is returned in the API response.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payout_id: String
    /// `force_sync` with the connector to get payout details
    /// (defaults to false)
    @dataExamples([
        {
            json: true
        }
    ])
    force_sync: Boolean
    /// The identifier for the Merchant Account.
    merchant_id: String
}

structure PayoutsCreateRequest {
    /// Your unique identifier for this payout or order. This ID helps you reconcile payouts on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "merchant_order_ref_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @range(
        min: 0
    )
    @required
    amount: Long
    @required
    currency: Currency
    routing: PayoutsCreateRequestRouting
    connector: PayoutsCreateRequestConnector
    /// This field is used when merchant wants to confirm the payout, thus useful for the payout _Confirm_ request. Ideally merchants should _Create_ a payout, _Update_ it (if required), then _Confirm_ it.
    @dataExamples([
        {
            json: true
        }
    ])
    confirm: Boolean
    payout_type: PayoutsCreateRequestPayoutType
    payout_method_data: PayoutsCreateRequestPayoutMethodData
    billing: PayoutsCreateRequestBilling
    /// Set to true to confirm the payout without review, no further action required
    @dataExamples([
        {
            json: true
        }
    ])
    auto_fulfill: Boolean
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated. _Deprecated: Use customer_id instead._
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    customer_id: String
    customer: PayoutsCreateRequestCustomer
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    return_url: String
    business_country: PayoutsCreateRequestBusinessCountry
    /// Business label of the merchant for this payout. _Deprecated: Use profile_id instead._
    @dataExamples([
        {
            json: "food"
        }
    ])
    business_label: String
    /// A description of the payout
    @dataExamples([
        {
            json: "It's my first payout request"
        }
    ])
    description: String
    entity_type: PayoutsCreateRequestEntityType
    /// Specifies whether or not the payout request is recurring
    recurring: Boolean
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// Provide a reference to a stored payout method, used to process the payout.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payout_token: String
    /// The business profile to use for this payout, especially if there are multiple business profiles associated with the account, otherwise default business profile associated with the merchant account will be used.
    profile_id: String
    priority: PayoutsCreateRequestPriority
    /// Whether to get the payout link (if applicable). Merchant need to specify this during the Payout _Create_, this field can not be updated during Payout _Update_.
    @dataExamples([
        {
            json: true
        }
    ])
    payout_link: Boolean
    payout_link_config: PayoutsCreateRequestPayoutLinkConfig
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Customer's email. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// Customer's name. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// Customer's phone. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// Customer's phone country code. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "+1"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    /// Identifier for payout method
    payout_method_id: String
}

structure PayoutsCreateRequestBusinessCountry {}

structure PayoutsCreateRequestEntityType {}

structure PayoutsCreateRequestPayoutLinkConfig with [PayoutCreatePayoutLinkConfig] {}

structure PayoutsCreateRequestPayoutMethodData {}

structure PayoutsCreateRequestPayoutType {}

structure PayoutsCreateRequestPriority {}

structure PayoutsCreateRequestRouting {}

structure PayoutUpdateRequest {
    /// Your unique identifier for this payout or order. This ID helps you reconcile payouts on your system. If provided, it is passed to the connector if supported.
    @dataExamples([
        {
            json: "merchant_order_ref_123"
        }
    ])
    @length(
        max: 255
    )
    merchant_order_reference_id: String
    /// The payout amount. Amount for the payout in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc.,
    @dataExamples([
        {
            json: 1000
        }
    ])
    @range(
        min: 0
    )
    amount: Long
    currency: PayoutUpdateRequestCurrency
    routing: PayoutUpdateRequestRouting
    connector: PayoutUpdateRequestConnector
    /// This field is used when merchant wants to confirm the payout, thus useful for the payout _Confirm_ request. Ideally merchants should _Create_ a payout, _Update_ it (if required), then _Confirm_ it.
    @dataExamples([
        {
            json: true
        }
    ])
    confirm: Boolean
    payout_type: PayoutUpdateRequestPayoutType
    payout_method_data: PayoutUpdateRequestPayoutMethodData
    billing: PayoutUpdateRequestBilling
    /// Set to true to confirm the payout without review, no further action required
    @dataExamples([
        {
            json: true
        }
    ])
    auto_fulfill: Boolean
    /// The identifier for the customer object. If not provided the customer ID will be autogenerated. _Deprecated: Use customer_id instead._
    @dataExamples([
        {
            json: "cus_y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    customer_id: String
    customer: PayoutUpdateRequestCustomer
    /// It's a token used for client side verification.
    @dataExamples([
        {
            json: "pay_U42c409qyHwOkWo3vK60_secret_el9ksDkiB8hi6j9N78yo"
        }
    ])
    client_secret: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://hyperswitch.io"
        }
    ])
    return_url: String
    business_country: PayoutUpdateRequestBusinessCountry
    /// Business label of the merchant for this payout. _Deprecated: Use profile_id instead._
    @dataExamples([
        {
            json: "food"
        }
    ])
    business_label: String
    /// A description of the payout
    @dataExamples([
        {
            json: "It's my first payout request"
        }
    ])
    description: String
    entity_type: PayoutUpdateRequestEntityType
    /// Specifies whether or not the payout request is recurring
    recurring: Boolean
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// Provide a reference to a stored payout method, used to process the payout.
    @dataExamples([
        {
            json: "187282ab-40ef-47a9-9206-5099ba31e432"
        }
    ])
    payout_token: String
    /// The business profile to use for this payout, especially if there are multiple business profiles associated with the account, otherwise default business profile associated with the merchant account will be used.
    profile_id: String
    priority: PayoutUpdateRequestPriority
    /// Whether to get the payout link (if applicable). Merchant need to specify this during the Payout _Create_, this field can not be updated during Payout _Update_.
    @dataExamples([
        {
            json: true
        }
    ])
    payout_link: Boolean
    payout_link_config: PayoutUpdateRequestPayoutLinkConfig
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds
    /// (900) for 15 mins
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    /// Customer's email. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
    /// Customer's name. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "John Test"
        }
    ])
    @length(
        max: 255
    )
    name: String
    /// Customer's phone. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @length(
        max: 255
    )
    phone: String
    /// Customer's phone country code. _Deprecated: Use customer object instead._
    @dataExamples([
        {
            json: "+1"
        }
    ])
    @length(
        max: 255
    )
    phone_country_code: String
    /// Identifier for payout method
    payout_method_id: String
}

structure PayoutUpdateRequestBusinessCountry {}

structure PayoutUpdateRequestCurrency {}

structure PayoutUpdateRequestEntityType {}

structure PayoutUpdateRequestPayoutLinkConfig with [PayoutCreatePayoutLinkConfig] {}

structure PayoutUpdateRequestPayoutMethodData {}

structure PayoutUpdateRequestPayoutType {}

structure PayoutUpdateRequestPriority {}

structure PayoutUpdateRequestRouting {}

structure Paypal {
    /// Email linked with paypal account
    @dataExamples([
        {
            json: "john.doe@example.com"
        }
    ])
    @required
    email: String
    /// mobile number linked to paypal account
    @dataExamples([
        {
            json: "16608213349"
        }
    ])
    @required
    telephone_number: String
    /// id of the paypal account
    @dataExamples([
        {
            json: "G83KXTJ5EHCQ2"
        }
    ])
    @required
    paypal_id: String
}

/// Masked payout method details for paypal wallet payout method
structure PaypalAdditionalData {
    /// Email linked with paypal account
    @dataExamples([
        {
            json: "john.doe@example.com"
        }
    ])
    email: String
    /// mobile number linked to paypal account
    @dataExamples([
        {
            json: "******* 3349"
        }
    ])
    telephone_number: String
    /// id of the paypal account
    @dataExamples([
        {
            json: "G83K ***** HCQ2"
        }
    ])
    paypal_id: String
}

structure PaypalRedirection {
    /// paypal's email address
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email: String
}

@mixin
structure PaypalSessionTokenResponse {
    /// Name of the connector
    @required
    connector: String
    /// The session token for PayPal
    @required
    session_token: String
    @required
    sdk_next_action: SdkNextAction
}

structure PayPalWalletData {
    /// Token generated for the Apple pay
    @required
    token: String
}

@mixin
structure PazeSessionTokenResponse {
    /// Paze Client ID
    @required
    client_id: String
    /// Client Name to be displayed on the Paze screen
    @required
    client_name: String
    /// Paze Client Profile ID
    @required
    client_profile_id: String
    @required
    transaction_currency_code: Currency
    /// The transaction amount
    @dataExamples([
        {
            json: "38.02"
        }
    ])
    @required
    transaction_amount: String
    /// Email Address
    @dataExamples([
        {
            json: "johntest@test.com"
        }
    ])
    @length(
        max: 255
    )
    email_address: String
}

structure PazeWalletData {
    @required
    complete_response: String
}

structure PermataBankTransfer {
    billing_details: BankTransferDataOneOfAlt4PermataBankTransferBillingDetails
}

structure Phone with [PhoneDetails] {}

@mixin
structure PhoneDetails {
    /// The contact number
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    number: String
    /// The country code attached to the number
    @dataExamples([
        {
            json: "+1"
        }
    ])
    country_code: String
}

structure Pix {
    /// Unique key for pix transfer
    @dataExamples([
        {
            json: "a1f4102e-a446-4a57-bcce-6fa48899c1d1"
        }
    ])
    pix_key: String
    /// CPF is a Brazilian tax identification number
    @dataExamples([
        {
            json: "10599054689"
        }
    ])
    cpf: String
    /// CNPJ is a Brazilian company tax identification number
    @dataExamples([
        {
            json: "74469027417312"
        }
    ])
    cnpj: String
    /// Source bank account number
    @dataExamples([
        {
            json: "8b2812f0-d6c8-4073-97bb-9fa964d08bc5"
        }
    ])
    source_bank_account_id: String
    /// Destination bank account number
    @dataExamples([
        {
            json: "9b95f84e-de61-460b-a14b-f23b4e71c97b"
        }
    ])
    destination_bank_account_id: String
}

structure PixBankTransfer {
    /// Bank name
    @dataExamples([
        {
            json: "Deutsche Bank"
        }
    ])
    bank_name: String
    /// Bank branch
    @dataExamples([
        {
            json: "3707"
        }
    ])
    bank_branch: String
    /// Bank account number is an unique identifier assigned by a bank to a customer.
    @dataExamples([
        {
            json: "000123456"
        }
    ])
    @required
    bank_account_number: String
    /// Unique key for pix customer
    @dataExamples([
        {
            json: "000123456"
        }
    ])
    @required
    pix_key: String
    /// Individual taxpayer identification number
    @dataExamples([
        {
            json: "000123456"
        }
    ])
    tax_id: String
}

structure PixBankTransferAdditionalData {
    /// Partially masked unique key for pix transfer
    @dataExamples([
        {
            json: "a1f4102e ****** 6fa48899c1d1"
        }
    ])
    pix_key: String
    /// Partially masked CPF - CPF is a Brazilian tax identification number
    @dataExamples([
        {
            json: "**** 124689"
        }
    ])
    cpf: String
    /// Partially masked CNPJ - CNPJ is a Brazilian company tax identification number
    @dataExamples([
        {
            json: "**** 417312"
        }
    ])
    cnpj: String
    /// Partially masked source bank account number
    @dataExamples([
        {
            json: "********-****-4073-****-9fa964d08bc5"
        }
    ])
    source_bank_account_id: String
    /// Partially masked destination bank account number
    @dataExamples([
        {
            json: "********-****-460b-****-f23b4e71c97b"
        }
    ])
    destination_bank_account_id: String
}

structure Platform {}

/// Swedish Plusgiro system
structure Plusgiro {
    /// Plusgiro number (2-8 digits)
    @dataExamples([
        {
            json: "4789-2"
        }
    ])
    @required
    number: String
    /// Account holder name
    @dataExamples([
        {
            json: "Anna Larsson"
        }
    ])
    @required
    name: String
    connector_recipient_id: String
}

@mixin
structure PollConfig {
    /// Interval of the poll
    @range(
        min: 0
    )
    @required
    delay_in_secs: Integer
    /// Frequency of the poll
    @range(
        min: 0
    )
    @required
    frequency: Integer
}

structure PollConfigResponse {
    /// Poll Id
    @required
    poll_id: String
    /// Interval of the poll
    @required
    delay_in_secs: Integer
    /// Frequency of the poll
    @required
    frequency: Integer
}

structure PollResponse {
    /// The poll id
    @required
    poll_id: String
    @required
    status: PollStatus
}

structure Position {}

structure PrimaryBusinessDetails with [PrimaryBusinessDetailsMixin] {}

@mixin
structure PrimaryBusinessDetailsMixin {
    @required
    country: CountryAlpha2
    @dataExamples([
        {
            json: "food"
        }
    ])
    @required
    business: String
}

/// Processor payment token for MIT payments where payment_method_data is not available
structure ProcessorPaymentToken {
    @required
    processor_payment_token: String
    merchant_connector_id: String
}

structure ProfileAcquirerCreate {
    /// The merchant id assigned by the acquirer
    @dataExamples([
        {
            json: "M123456789"
        }
    ])
    @required
    acquirer_assigned_merchant_id: String
    /// merchant name
    @dataExamples([
        {
            json: "NewAge Retailer"
        }
    ])
    @required
    merchant_name: String
    /// Merchant country code assigned by acquirer
    @dataExamples([
        {
            json: "US"
        }
    ])
    @required
    merchant_country_code: String
    /// Network provider
    @dataExamples([
        {
            json: "VISA"
        }
    ])
    @required
    network: String
    /// Acquirer bin
    @dataExamples([
        {
            json: "456789"
        }
    ])
    @required
    acquirer_bin: String
    /// Acquirer ica provided by acquirer
    @dataExamples([
        {
            json: "401288"
        }
    ])
    acquirer_ica: String
    /// Fraud rate for the particular acquirer configuration
    @dataExamples([
        {
            json: 0.01
        }
    ])
    @required
    acquirer_fraud_rate: Double
    /// Parent profile id to link the acquirer account with
    @dataExamples([
        {
            json: "pro_ky0yNyOXXlA5hF8JzE5q"
        }
    ])
    @required
    profile_id: String
}

structure ProfileAcquirerResponse {
    /// The unique identifier of the profile acquirer
    @dataExamples([
        {
            json: "pro_acq_LCRdERuylQvNQ4qh3QE0"
        }
    ])
    @required
    profile_acquirer_id: String
    /// The merchant id assigned by the acquirer
    @dataExamples([
        {
            json: "M123456789"
        }
    ])
    @required
    acquirer_assigned_merchant_id: String
    /// Merchant name
    @dataExamples([
        {
            json: "NewAge Retailer"
        }
    ])
    @required
    merchant_name: String
    /// Merchant country code assigned by acquirer
    @dataExamples([
        {
            json: "US"
        }
    ])
    @required
    merchant_country_code: String
    /// Network provider
    @dataExamples([
        {
            json: "VISA"
        }
    ])
    @required
    network: String
    /// Acquirer bin
    @dataExamples([
        {
            json: "456789"
        }
    ])
    @required
    acquirer_bin: String
    /// Acquirer ica provided by acquirer
    @dataExamples([
        {
            json: "401288"
        }
    ])
    acquirer_ica: String
    /// Fraud rate for the particular acquirer configuration
    @dataExamples([
        {
            json: 0.01
        }
    ])
    @required
    acquirer_fraud_rate: Double
    /// Parent profile id to link the acquirer account with
    @dataExamples([
        {
            json: "pro_ky0yNyOXXlA5hF8JzE5q"
        }
    ])
    @required
    profile_id: String
}

structure ProfileAcquirerUpdate {
    @dataExamples([
        {
            json: "M987654321"
        }
    ])
    acquirer_assigned_merchant_id: String
    @dataExamples([
        {
            json: "Updated Retailer Name"
        }
    ])
    merchant_name: String
    @dataExamples([
        {
            json: "CA"
        }
    ])
    merchant_country_code: String
    @dataExamples([
        {
            json: "MASTERCARD"
        }
    ])
    network: String
    @dataExamples([
        {
            json: "987654"
        }
    ])
    acquirer_bin: String
    @dataExamples([
        {
            json: "501299"
        }
    ])
    acquirer_ica: String
    @dataExamples([
        {
            json: 0.02
        }
    ])
    acquirer_fraud_rate: Double
}

structure ProfileCreate {
    /// The name of profile
    @length(
        max: 64
    )
    profile_name: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://www.example.com/success"
        }
    ])
    @length(
        max: 255
    )
    return_url: String
    /// A boolean value to indicate if payment response hash needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    enable_payment_response_hash: Boolean
    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    payment_response_hash_key: String
    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    redirect_to_merchant_with_http_post: Boolean
    webhook_details: ProfileCreateWebhookDetails
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// The routing algorithm to be used for routing payments to desired connectors
    routing_algorithm: Document
    /// Will be used to determine the time till which your payment will be active once the payment session starts
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    intent_fulfillment_time: Integer
    /// The frm routing algorithm to be used for routing payments to desired FRM's
    frm_routing_algorithm: Document
    payout_routing_algorithm: ProfileCreatePayoutRoutingAlgorithm
    applepay_verified_domains: ProfileCreateApplepayVerifiedDomains
    /// Client Secret Default expiry for all payments created under this profile
    @dataExamples([
        {
            json: 900
        }
    ])
    @range(
        min: 0
    )
    session_expiry: Integer
    payment_link_config: ProfileCreatePaymentLinkConfig
    authentication_connector_details: ProfileCreateAuthenticationConnectorDetails
    /// Whether to use the billing details passed when creating the intent as payment method billing
    use_billing_as_payment_method_billing: Boolean
    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    collect_shipping_details_from_wallet_connector: Boolean
    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    collect_billing_details_from_wallet_connector: Boolean
    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    always_collect_shipping_details_from_wallet_connector: Boolean
    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    always_collect_billing_details_from_wallet_connector: Boolean
    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    is_connector_agnostic_mit_enabled: Boolean
    payout_link_config: ProfileCreatePayoutLinkConfig
    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended not to use more than four key-value pairs.
    outgoing_webhook_custom_http_headers: Document
    /// Merchant Connector id to be stored for tax_calculator connector
    tax_connector_id: String
    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    is_tax_connector_enabled: Boolean
    /// Indicates if network tokenization is enabled or not.
    is_network_tokenization_enabled: Boolean
    /// Indicates if is_auto_retries_enabled is enabled or not.
    is_auto_retries_enabled: Boolean
    /// Maximum number of auto retries allowed for a payment
    @range(
        min: 0
    )
    max_auto_retries_enabled: Integer
    /// Bool indicating if extended authentication must be requested for all payments
    always_request_extended_authorization: Boolean
    /// Indicates if click to pay is enabled or not.
    is_click_to_pay_enabled: Boolean
    /// Product authentication ids
    authentication_product_ids: Document
    card_testing_guard_config: ProfileCreateCardTestingGuardConfig
    /// Indicates if clear pan retries is enabled or not.
    is_clear_pan_retries_enabled: Boolean
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: Boolean
    /// Indicates if debit routing is enabled or not
    is_debit_routing_enabled: Boolean
    merchant_business_country: ProfileCreateMerchantBusinessCountry
    /// Indicates if the redirection has to open in the iframe
    @dataExamples([
        {
            json: false
        }
    ])
    is_iframe_redirection_enabled: Boolean
    /// Indicates if pre network tokenization is enabled or not
    is_pre_network_tokenization_enabled: Boolean
    merchant_category_code: ProfileCreateMerchantCategoryCode
}

structure ProfileCreateAuthenticationConnectorDetails with [AuthenticationConnectorDetails] {}

structure ProfileCreateCardTestingGuardConfig with [CardTestingGuardConfig] {}

structure ProfileCreateMerchantBusinessCountry {}

structure ProfileCreateMerchantCategoryCode {}

structure ProfileCreatePayoutLinkConfig with [BusinessPayoutLinkConfig] {}

structure ProfileCreatePayoutRoutingAlgorithm {}

structure ProfileCreateWebhookDetails with [WebhookDetails] {}

structure ProfileDefaultRoutingConfig {
    @required
    profile_id: String
    @required
    connectors: ProfileDefaultRoutingConfigConnectors
}

structure ProfileResponse {
    /// The identifier for Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// The identifier for profile. This must be used for creating merchant accounts, payments and payouts
    @dataExamples([
        {
            json: "pro_abcdefghijklmnopqrstuvwxyz"
        }
    ])
    @length(
        max: 64
    )
    @required
    profile_id: String
    /// Name of the profile
    @length(
        max: 64
    )
    @required
    profile_name: String
    /// The URL to redirect after the completion of the operation
    @dataExamples([
        {
            json: "https://www.example.com/success"
        }
    ])
    @length(
        max: 255
    )
    return_url: String
    /// A boolean value to indicate if payment response hash needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    enable_payment_response_hash: Boolean
    /// Refers to the hash key used for calculating the signature for webhooks and redirect response. If the value is not provided, a value is automatically generated.
    payment_response_hash_key: String
    /// A boolean value to indicate if redirect to merchant with http post needs to be enabled
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    redirect_to_merchant_with_http_post: Boolean
    webhook_details: ProfileResponseWebhookDetails
    /// Metadata is useful for storing additional, unstructured information on an object.
    metadata: Document
    /// The routing algorithm to be used for routing payments to desired connectors
    routing_algorithm: Document
    /// Will be used to determine the time till which your payment will be active once the payment session starts
    @dataExamples([
        {
            json: 900
        }
    ])
    intent_fulfillment_time: Long
    /// The routing algorithm to be used to process the incoming request from merchant to outgoing payment processor or payment method. The default is 'Custom'
    frm_routing_algorithm: Document
    payout_routing_algorithm: ProfileResponsePayoutRoutingAlgorithm
    applepay_verified_domains: ProfileResponseApplepayVerifiedDomains
    /// Client Secret Default expiry for all payments created under this profile
    @dataExamples([
        {
            json: 900
        }
    ])
    session_expiry: Long
    payment_link_config: ProfileResponsePaymentLinkConfig
    authentication_connector_details: ProfileResponseAuthenticationConnectorDetails
    use_billing_as_payment_method_billing: Boolean
    extended_card_info_config: ExtendedCardInfoConfig
    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    collect_shipping_details_from_wallet_connector: Boolean
    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    collect_billing_details_from_wallet_connector: Boolean
    /// A boolean value to indicate if customer shipping details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    always_collect_shipping_details_from_wallet_connector: Boolean
    /// A boolean value to indicate if customer billing details needs to be collected from wallet
    /// connector irrespective of connector required fields (Eg. Apple pay, Google pay etc)
    @dataExamples([
        {
            json: false
        }
    ])
    always_collect_billing_details_from_wallet_connector: Boolean
    /// Indicates if the MIT (merchant initiated transaction) payments can be made connector
    /// agnostic, i.e., MITs may be processed through different connector than CIT (customer
    /// initiated transaction) based on the routing rules.
    /// If set to `false`, MIT will go through the same connector as the CIT.
    is_connector_agnostic_mit_enabled: Boolean
    payout_link_config: ProfileResponsePayoutLinkConfig
    /// These key-value pairs are sent as additional custom headers in the outgoing webhook request.
    outgoing_webhook_custom_http_headers: Document
    /// Merchant Connector id to be stored for tax_calculator connector
    tax_connector_id: String
    /// Indicates if tax_calculator connector is enabled or not.
    /// If set to `true` tax_connector_id will be checked.
    @required
    is_tax_connector_enabled: Boolean
    /// Indicates if network tokenization is enabled or not.
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    is_network_tokenization_enabled: Boolean
    /// Indicates if is_auto_retries_enabled is enabled or not.
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    is_auto_retries_enabled: Boolean
    /// Maximum number of auto retries allowed for a payment
    max_auto_retries_enabled: Integer
    /// Bool indicating if extended authentication must be requested for all payments
    always_request_extended_authorization: Boolean
    /// Indicates if click to pay is enabled or not.
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    is_click_to_pay_enabled: Boolean
    /// Product authentication ids
    authentication_product_ids: Document
    card_testing_guard_config: ProfileResponseCardTestingGuardConfig
    /// Indicates if clear pan retries is enabled or not.
    @required
    is_clear_pan_retries_enabled: Boolean
    /// Indicates if 3ds challenge is forced
    @required
    force_3ds_challenge: Boolean
    /// Indicates if debit routing is enabled or not
    is_debit_routing_enabled: Boolean
    merchant_business_country: ProfileResponseMerchantBusinessCountry
    /// Indicates if pre network tokenization is enabled or not
    @dataExamples([
        {
            json: false
        }
    ])
    @required
    is_pre_network_tokenization_enabled: Boolean
    acquirer_configs: AcquirerConfigs
    /// Indicates if the redirection has to open in the iframe
    @dataExamples([
        {
            json: false
        }
    ])
    is_iframe_redirection_enabled: Boolean
    merchant_category_code: ProfileResponseMerchantCategoryCode
}

structure ProfileResponseAuthenticationConnectorDetails with [AuthenticationConnectorDetails] {}

structure ProfileResponseCardTestingGuardConfig with [CardTestingGuardConfig] {}

structure ProfileResponseMerchantBusinessCountry {}

structure ProfileResponseMerchantCategoryCode {}

structure ProfileResponsePayoutLinkConfig with [BusinessPayoutLinkConfig] {}

structure ProfileResponsePayoutRoutingAlgorithm {}

structure ProfileResponseWebhookDetails with [WebhookDetails] {}

/// The program, having a default connector selection and
/// a bunch of rules. Also can hold arbitrary metadata.
structure ProgramConnectorSelection {
    @required
    defaultSelection: ConnectorSelection
    @required
    rules: RuleConnectorSelection
    @required
    metadata: ProgramConnectorSelectionMetadata
}

structure ProgramThreeDsDecisionRule {
    @required
    defaultSelection: ThreeDSDecisionRule
    @required
    rules: RuleThreeDsDecisionRule
    @required
    metadata: ProgramThreeDsDecisionRuleMetadata
}

structure Przelewy24 {
    bank_name: BankRedirectDataOneOfAlt12Przelewy24BankName
    billing_details: BankRedirectDataOneOfAlt12Przelewy24BillingDetails
}

structure RealTimePaymentDataOneOfAlt0 {
    @required
    fps: Document
}

structure RealTimePaymentDataOneOfAlt1 {
    @required
    duit_now: Document
}

structure RealTimePaymentDataOneOfAlt2 {
    @required
    prompt_pay: Document
}

structure RealTimePaymentDataOneOfAlt3 {
    @required
    viet_qr: Document
}

structure RealTimePaymentDataResponse {}

structure Receiver with [ReceiverDetails] {}

@mixin
structure ReceiverDetails {
    /// The amount received by receiver
    @required
    amount_received: Long
    /// The amount charged by ACH
    amount_charged: Long
    /// The amount remaining to be sent via ACH
    amount_remaining: Long
}

structure RecurringDetailsOneOfAlt0 {
    @required
    data: String
}

structure RecurringDetailsOneOfAlt1 {
    @required
    data: String
}

structure RecurringDetailsOneOfAlt2 {
    @required
    data: ProcessorPaymentToken
}

structure RecurringDetailsOneOfAlt3 {
    @required
    data: NetworkTransactionIdAndCardDetails
}

structure RecurringPaymentRequest with [ApplePayRecurringPaymentRequest] {}

@mixin
structure RedirectResponse {
    param: String
    json_payload: Document
}

structure RefundListRequest {
    /// The identifier for the payment
    payment_id: String
    /// The identifier for the refund
    refund_id: String
    /// The identifier for business profile
    profile_id: String
    /// Limit on the number of objects to return
    limit: Long
    /// The starting point within a list of objects
    offset: Long
    amount_filter: AmountFilter
    connector: RefundListRequestAllOf1Connector
    merchant_connector_id: MerchantConnectorId
    currency: RefundListRequestAllOf1Currency
    refund_status: RefundStatus
}

structure RefundListResponse {
    /// The number of refunds included in the list
    @range(
        min: 0
    )
    @required
    count: Integer
    /// The total number of refunds in the list
    @required
    total_count: Long
    @required
    data: RefundListResponseData
}

structure RefundRequest {
    /// The payment id against which refund is to be initiated
    @dataExamples([
        {
            json: "pay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    @required
    payment_id: String
    /// Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refunds initiated against the same payment. If this is not passed by the merchant, this field shall be auto generated and provided in the API response. It is recommended to generate uuid(v4) as the refund_id.
    @dataExamples([
        {
            json: "ref_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @length(
        min: 30
        max: 30
    )
    refund_id: String
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    merchant_id: String
    /// Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, this will default to the full payment amount
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 100
    )
    amount: Long
    /// Reason for the refund. Often useful for displaying to users and your customer support executive. In case the payment went through Stripe, this field needs to be passed with one of these enums: `duplicate`, `fraudulent`, or `requested_by_customer`
    @dataExamples([
        {
            json: "Customer returned the product"
        }
    ])
    @length(
        max: 255
    )
    reason: String
    refund_type: RefundType
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    merchant_connector_details: RefundRequestMerchantConnectorDetails
    split_refunds: RefundRequestSplitRefunds
}

structure RefundRequestMerchantConnectorDetails with [MerchantConnectorDetailsWrap] {}

structure RefundRequestSplitRefunds {}

structure RefundResponse {
    /// Unique Identifier for the refund
    @required
    refund_id: String
    /// The payment id against which refund is initiated
    @required
    payment_id: String
    /// The refund amount, which should be less than or equal to the total payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc
    @dataExamples([
        {
            json: 6540
        }
    ])
    @range(
        min: 100
    )
    @required
    amount: Long
    /// The three-letter ISO currency code
    @required
    currency: String
    @required
    status: RefundStatus
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    reason: String
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object
    metadata: Document
    /// The error message
    error_message: String
    /// The code for the error
    error_code: String
    /// Error code unified across the connectors is received here if there was an error while calling connector
    unified_code: String
    /// Error message unified across the connectors is received here if there was an error while calling connector
    unified_message: String
    /// The timestamp at which refund is created
    @timestampFormat("date-time")
    created_at: Timestamp
    /// The timestamp at which refund is updated
    @timestampFormat("date-time")
    updated_at: Timestamp
    /// The connector used for the refund and the corresponding payment
    @dataExamples([
        {
            json: "stripe"
        }
    ])
    @required
    connector: String
    /// The id of business profile for this refund
    profile_id: String
    /// The merchant_connector_id of the processor through which this payment went through
    merchant_connector_id: String
    split_refunds: RefundResponseSplitRefunds
    /// Error code received from the issuer in case of failed refunds
    issuer_error_code: String
    /// Error message received from the issuer in case of failed refunds
    issuer_error_message: String
}

structure RefundResponseSplitRefunds {}

structure RefundUpdateRequest {
    /// An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive
    @dataExamples([
        {
            json: "Customer returned the product"
        }
    ])
    @length(
        max: 255
    )
    reason: String
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
}

structure RelayDataOneOfAlt0 {
    @required
    refund: RelayRefundRequestData
}

@mixin
structure RelayError {
    /// The error code
    @required
    code: String
    /// The error message
    @required
    message: String
}

structure RelayRefundRequestData {
    /// The amount that is being refunded
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    @required
    currency: Currency
    /// The reason for the refund
    @dataExamples([
        {
            json: "Customer returned the product"
        }
    ])
    @length(
        max: 255
    )
    reason: String
}

structure RelayRequest200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RelayResponse
}

@error("client")
@httpError(400)
structure RelayRequest400 {}

structure RelayRequestData {}

structure RelayRequestInput {
    /// Profile ID for authentication
    @httpHeader("X-Profile-Id")
    @required
    X_Profile_Id: String
    /// Idempotency Key for relay request
    @httpHeader("X-Idempotency-Key")
    @required
    X_Idempotency_Key: String
    @httpPayload
    @required
    @contentType("application/json")
    body: RelayRequest
}

structure RelayResponse {
    /// The unique identifier for the Relay
    @dataExamples([
        {
            json: "relay_mbabizu24mvu3mela5njyhpit4"
        }
    ])
    @required
    id: String
    @required
    status: RelayStatus
    /// The identifier that is associated to a resource at the connector reference to which the relay request is being made
    @dataExamples([
        {
            json: "pi_3MKEivSFNglxLpam0ZaL98q9"
        }
    ])
    @required
    connector_resource_id: String
    error: Error
    /// The identifier that is associated to a resource at the connector to which the relay request is being made
    @dataExamples([
        {
            json: "re_3QY4TnEOqOywnAIx1Mm1p7GQ"
        }
    ])
    connector_reference_id: String
    /// Identifier of the connector ( merchant connector account ) which was chosen to make the payment
    @dataExamples([
        {
            json: "mca_5apGeP94tMts6rg3U3kR"
        }
    ])
    @required
    connector_id: String
    /// The business profile that is associated with this relay request.
    @dataExamples([
        {
            json: "pro_abcdefghijklmnopqrstuvwxyz"
        }
    ])
    @required
    profile_id: String
    @required
    type: RelayType
    data: RelayResponseData
}

structure RelayResponseData {}

structure RequestPaymentMethodTypes {
    @required
    payment_method_type: PaymentMethodType
    payment_experience: RequestPaymentMethodTypesPaymentExperience
    card_networks: RequestPaymentMethodTypesCardNetworks
    accepted_currencies: RequestPaymentMethodTypesAcceptedCurrencies
    accepted_countries: RequestPaymentMethodTypesAcceptedCountries
    minimum_amount: MinimumAmount
    maximum_amount: MaximumAmount
    /// Indicates whether the payment method supports recurring payments. Optional.
    @dataExamples([
        {
            json: false
        }
    ])
    recurring_enabled: Boolean
    /// Indicates whether the payment method is eligible for installment payments (e.g., EMI, BNPL). Optional.
    @dataExamples([
        {
            json: true
        }
    ])
    installment_payment_enabled: Boolean
}

structure RequestPaymentMethodTypesAcceptedCountries {}

structure RequestPaymentMethodTypesAcceptedCurrencies {}

structure RequestPaymentMethodTypesPaymentExperience {}

/// Details of surcharge applied on this payment, if applicable
@mixin
structure RequestSurchargeDetails {
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    surcharge_amount: Long
    tax_amount: TaxAmount
}

structure RequiredBillingContactFields {}

/// Required fields info used while listing the payment_method_data
structure RequiredFieldInfo {
    /// Required field for a payment_method through a payment_method_type
    @required
    required_field: String
    /// Display name of the required field in the front-end
    @required
    display_name: String
    @required
    field_type: FieldType
    value: String
}

structure RequiredShippingContactFields {}

structure ResponsePaymentMethodsEnabled {
    @required
    payment_method: PaymentMethod
    @required
    payment_method_types: ResponsePaymentMethodsEnabledPaymentMethodTypes
}

structure ResponsePaymentMethodTypes {
    @required
    payment_method_type: PaymentMethodType
    payment_experience: ResponsePaymentMethodTypesPaymentExperience
    card_networks: ResponsePaymentMethodTypesCardNetworks
    bank_names: BankNames
    bank_debits: BankDebits
    bank_transfers: BankTransfers
    required_fields: RequiredFields
    surcharge_details: ResponsePaymentMethodTypesSurchargeDetails
    /// auth service connector label for this payment method type, if exists
    pm_auth_connector: String
}

structure ResponsePaymentMethodTypesSurchargeDetails with [SurchargeDetailsResponse] {}

structure RetrieveActiveConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: LinkedRoutingConfigRetrieveResponse
}

@error("client")
@httpError(403)
structure RetrieveActiveConfig403 {}

@error("client")
@httpError(404)
structure RetrieveActiveConfig404 {}

@error("server")
@httpError(500)
structure RetrieveActiveConfig500 {}

structure RetrieveActiveConfigInput {
    /// The unique identifier for a merchant profile
    @httpQuery("profile_id")
    profile_id: String
}

structure RetrieveACustomer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerResponse
}

@error("client")
@httpError(404)
structure RetrieveACustomer404 {}

structure RetrieveACustomerInput {
    /// The unique identifier for the Customer
    @httpLabel
    @required
    customer_id: String
}

structure RetrieveADispute200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: DisputeResponse
}

@error("client")
@httpError(404)
structure RetrieveADispute404 {}

structure RetrieveADisputeInput {
    /// The identifier for dispute
    @httpLabel
    @required
    dispute_id: String
}

structure RetrieveAMandate200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MandateResponse
}

@error("client")
@httpError(404)
structure RetrieveAMandate404 {}

structure RetrieveAMandateInput {
    /// The identifier for mandate
    @httpLabel
    @required
    mandate_id: String
}

structure RetrieveAMerchantAccount200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantAccountResponse
}

@error("client")
@httpError(404)
structure RetrieveAMerchantAccount404 {}

structure RetrieveAMerchantAccountInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
}

structure RetrieveAMerchantConnector200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantConnectorResponse
}

@error("client")
@httpError(401)
structure RetrieveAMerchantConnector401 {}

@error("client")
@httpError(404)
structure RetrieveAMerchantConnector404 {}

structure RetrieveAMerchantConnectorInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    /// The unique identifier for the Merchant Connector
    @httpLabel
    @required
    merchant_connector_id: String
}

structure RetrieveAnAPIKey200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RetrieveApiKeyResponse
}

@error("client")
@httpError(404)
structure RetrieveAnAPIKey404 {}

structure RetrieveAnAPIKeyInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    merchant_id: String
    /// The unique identifier for the API Key
    @httpLabel
    @required
    key_id: String
}

structure RetrieveAnOrganization200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: OrganizationResponse
}

@error("client")
@httpError(400)
structure RetrieveAnOrganization400 {}

structure RetrieveAnOrganizationInput {
    /// The unique identifier for the Organization
    @httpLabel
    @required
    id: String
}

structure RetrieveAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsResponse
}

@error("client")
@httpError(404)
structure RetrieveAPayment404 {}

structure RetrieveAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    /// Decider to enable or disable the connector call for retrieve request
    @httpQuery("force_sync")
    force_sync: Boolean
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @httpQuery("client_secret")
    client_secret: String
    /// If enabled provides list of attempts linked to payment intent
    @httpQuery("expand_attempts")
    expand_attempts: Boolean
    /// If enabled provides list of captures linked to latest attempt
    @httpQuery("expand_captures")
    expand_captures: Boolean
}

structure RetrieveAPaymentLink200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RetrievePaymentLinkResponse
}

@error("client")
@httpError(404)
structure RetrieveAPaymentLink404 {}

structure RetrieveAPaymentLinkInput {
    /// The identifier for payment link
    @httpLabel
    @required
    payment_link_id: String
    /// This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK
    @httpQuery("client_secret")
    client_secret: String
}

structure RetrieveAPaymentMethod200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodResponse
}

@error("client")
@httpError(404)
structure RetrieveAPaymentMethod404 {}

structure RetrieveAPaymentMethodInput {
    /// The unique identifier for the Payment Method
    @httpLabel
    @required
    method_id: String
}

structure RetrieveAPayout200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCreateResponse
}

@error("client")
@httpError(404)
structure RetrieveAPayout404 {}

structure RetrieveAPayoutInput {
    /// The identifier for payout
    @httpLabel
    @required
    payout_id: String
    /// Sync with the connector to get the payout details (defaults to false)
    @httpQuery("force_sync")
    force_sync: Boolean
}

/// The response body for retrieving an API Key.
structure RetrieveApiKeyResponse {
    /// The identifier for the API Key.
    @dataExamples([
        {
            json: "5hEEqkgJUyuxgSKGArHA4mWSnX"
        }
    ])
    @length(
        max: 64
    )
    @required
    key_id: String
    /// The identifier for the Merchant Account.
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// The unique name for the API Key to help you identify it.
    @dataExamples([
        {
            json: "Sandbox integration key"
        }
    ])
    @length(
        max: 64
    )
    @required
    name: String
    /// The description to provide more context about the API Key.
    @dataExamples([
        {
            json: "Key used by our developers to integrate with the sandbox environment"
        }
    ])
    @length(
        max: 256
    )
    description: String
    /// The first few characters of the plaintext API Key to help you identify it.
    @length(
        max: 64
    )
    @required
    prefix: String
    /// The time at which the API Key was created.
    @dataExamples([
        {
            json: "2022-09-10T10:11:12Z"
        }
    ])
    @required
    @timestampFormat("date-time")
    created: Timestamp
    @required
    expiration: ApiKeyExpiration
}

structure RetrieveAProfile200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileResponse
}

@error("client")
@httpError(400)
structure RetrieveAProfile400 {}

structure RetrieveAProfileInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    /// The unique identifier for the profile
    @httpLabel
    @required
    profile_id: String
}

structure RetrieveARefund200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundResponse
}

@error("client")
@httpError(404)
structure RetrieveARefund404 {}

structure RetrieveARefundInput {
    /// The identifier for refund
    @httpLabel
    @required
    refund_id: String
}

structure RetrieveARelayDetails200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RelayResponse
}

@error("client")
@httpError(404)
structure RetrieveARelayDetails404 {}

structure RetrieveARelayDetailsInput {
    /// The unique identifier for the Relay
    @httpLabel
    @required
    relay_id: String
    /// Profile ID for authentication
    @httpHeader("X-Profile-Id")
    @required
    X_Profile_Id: String
}

structure RetrieveARoutingConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantRoutingAlgorithm
}

@error("client")
@httpError(403)
structure RetrieveARoutingConfig403 {}

@error("client")
@httpError(404)
structure RetrieveARoutingConfig404 {}

@error("server")
@httpError(500)
structure RetrieveARoutingConfig500 {}

structure RetrieveARoutingConfigInput {
    /// The unique identifier for a config
    @httpLabel
    @required
    routing_algorithm_id: String
}

structure RetrieveDefaultConfigsForAllProfiles200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileDefaultRoutingConfig
}

@error("client")
@httpError(404)
structure RetrieveDefaultConfigsForAllProfiles404 {}

@error("server")
@httpError(500)
structure RetrieveDefaultConfigsForAllProfiles500 {}

structure RetrieveDefaultFallbackConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RetrieveDefaultFallbackConfig200Body
}

@error("server")
@httpError(500)
structure RetrieveDefaultFallbackConfig500 {}

structure RetrieveGsmRule200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmResponse
}

@error("client")
@httpError(400)
structure RetrieveGsmRule400 {}

structure RetrieveGsmRuleInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmRetrieveRequest
}

structure RetrievePaymentLinkRequest {
    /// It's a token used for client side verification.
    client_secret: String
}

structure RetrievePaymentLinkResponse {
    /// Identifier for Payment Link
    @required
    payment_link_id: String
    /// Identifier for Merchant
    @required
    merchant_id: String
    /// Open payment link (without any security checks and listing SPMs)
    @required
    link_to_pay: String
    /// The payment amount. Amount for the payment in the lowest denomination of the currency
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    amount: Long
    /// Date and time of Payment Link creation
    @required
    @timestampFormat("date-time")
    created_at: Timestamp
    /// Date and time of Expiration for Payment Link
    @timestampFormat("date-time")
    expiry: Timestamp
    /// Description for Payment Link
    description: String
    @required
    status: PaymentLinkStatus
    currency: RetrievePaymentLinkResponseCurrency
    /// Secure payment link (with security checks and listing saved payment methods)
    secure_link: String
}

structure RetrievePaymentLinkResponseCurrency {}

structure RetrievePollStatus200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PollResponse
}

@error("client")
@httpError(404)
structure RetrievePollStatus404 {}

structure RetrievePollStatusInput {
    /// The identifier for poll
    @httpLabel
    @required
    poll_id: String
}

structure RevokeAMandate200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MandateRevokedResponse
}

@error("client")
@httpError(400)
structure RevokeAMandate400 {}

structure RevokeAMandateInput {
    /// The identifier for a mandate
    @httpLabel
    @required
    mandate_id: String
}

structure RevokeAnAPIKey200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RevokeApiKeyResponse
}

@error("client")
@httpError(404)
structure RevokeAnAPIKey404 {}

structure RevokeAnAPIKeyInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    merchant_id: String
    /// The unique identifier for the API Key
    @httpLabel
    @required
    key_id: String
}

/// The response body for revoking an API Key.
structure RevokeApiKeyResponse {
    /// The identifier for the Merchant Account.
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 64
    )
    @required
    merchant_id: String
    /// The identifier for the API Key.
    @dataExamples([
        {
            json: "5hEEqkgJUyuxgSKGArHA4mWSnX"
        }
    ])
    @length(
        max: 64
    )
    @required
    key_id: String
    /// Indicates whether the API key was revoked or not.
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    revoked: Boolean
}

structure RewardData {
    /// The merchant ID with which we have to call the connector
    @required
    merchant_id: String
}

/// Routable Connector chosen for a payment
structure RoutableConnectorChoice {
    @required
    connector: RoutableConnectors
    merchant_connector_id: String
}

structure RoutingConfigRequest {
    name: String
    description: String
    algorithm: RoutingConfigRequestAlgorithm
    profile_id: String
    transaction_type: TransactionType
}

structure RoutingConfigRequestAlgorithm {}

structure RoutingDictionary {
    @required
    merchant_id: String
    active_id: String
    @required
    records: Records
}

structure RoutingDictionaryRecord {
    @required
    id: String
    @required
    profile_id: String
    @required
    name: String
    @required
    kind: RoutingAlgorithmKind
    @required
    description: String
    @required
    created_at: Long
    @required
    modified_at: Long
    algorithm_for: AlgorithmFor
    decision_engine_routing_id: String
}

/// Response of the retrieved routing configs for a merchant account
structure RoutingRetrieveResponse {
    algorithm: RoutingRetrieveResponseAlgorithm
}

structure RoutingRetrieveResponseAlgorithm with [MerchantRoutingAlgorithmMixin] {}

structure RoutingVolumeSplitResponse {
    @range(
        min: 0
    )
    @required
    split: Integer
}

/// Represents a rule
/// 
/// ```text
/// rule_name: [stripe, adyen, checkout]
/// {
/// payment.method = card {
/// payment.method.cardtype = (credit, debit) {
/// payment.method.network = (amex, rupay, diners)
/// }
/// 
/// payment.method.cardtype = credit
/// }
/// }
/// ```
structure RuleConnectorSelection {
    @required
    name: String
    @required
    connectorSelection: ConnectorSelection
    @required
    statements: RuleConnectorSelectionStatements
}

structure RuleThreeDsDecisionRule {
    @required
    name: String
    @required
    connectorSelection: ThreeDSDecision
    @required
    statements: RuleThreeDsDecisionRuleStatements
}

structure SamsungPayAmountDetails {
    @required
    option: SamsungPayAmountFormat
    @required
    currency_code: Currency
    /// The total amount of the transaction
    @dataExamples([
        {
            json: "38.02"
        }
    ])
    @required
    total: String
}

structure SamsungPayAppWalletData {
    @jsonName("3_d_s")
    @required
    n3_d_s: SamsungPayTokenData
    @required
    payment_card_brand: SamsungPayCardBrand
    /// Currency type of the payment
    @required
    payment_currency_type: String
    /// Last 4 digits of the device specific card number
    payment_last4_dpan: String
    /// Last 4 digits of the card number
    @required
    payment_last4_fpan: String
    /// Merchant reference id that was passed in the session call request
    merchant_ref: String
    /// Specifies authentication method used
    method: String
    /// Value if credential is enabled for recurring payment
    recurring_payment: Boolean
}

structure SamsungPayMerchantPaymentInformation {
    /// Merchant name, this will be displayed on the Samsung Pay screen
    @required
    name: String
    /// Merchant domain that process payments, required for web payments
    url: String
    @required
    country_code: CountryAlpha2
}

@mixin
structure SamsungPaySessionTokenResponse {
    /// Samsung Pay API version
    @required
    version: String
    /// Samsung Pay service ID to which session call needs to be made
    @required
    service_id: String
    /// Order number of the transaction
    @required
    order_number: String
    @required
    merchant: SamsungPayMerchantPaymentInformation
    @required
    amount: SamsungPayAmountDetails
    @required
    protocol: SamsungPayProtocolType
    @required
    allowed_brands: AllowedBrands
    /// Is billing address required to be collected from wallet
    @required
    billing_address_required: Boolean
    /// Is shipping address required to be collected from wallet
    @required
    shipping_address_required: Boolean
}

structure SamsungPayTokenData {
    /// 3DS type used by Samsung Pay
    type: String
    /// 3DS version used by Samsung Pay
    @required
    version: String
    /// Samsung Pay encrypted payment credential data
    @required
    data: String
}

structure SamsungPayWalletData {
    @required
    payment_credential: SamsungPayWalletCredentials
}

structure SamsungPayWebWalletData {
    /// Specifies authentication method used
    method: String
    /// Value if credential is enabled for recurring payment
    recurring_payment: Boolean
    @required
    card_brand: SamsungPayCardBrand
    /// Last 4 digits of the card number
    @required
    card_last4digits: String
    @jsonName("3_d_s")
    @required
    n3_d_s: SamsungPayTokenData
}

/// SDK Information if request is from SDK
@mixin
structure SdkInformation {
    /// Unique ID created on installations of the 3DS Requestor App on a Consumer Device
    @required
    sdk_app_id: String
    /// JWE Object containing data encrypted by the SDK for the DS to decrypt
    @required
    sdk_enc_data: String
    @required
    sdk_ephem_pub_key: SdkEphemPubKey
    /// Unique transaction identifier assigned by the 3DS SDK
    @required
    sdk_trans_id: String
    /// Identifies the vendor and version for the 3DS SDK that is integrated in a 3DS Requestor App
    @required
    sdk_reference_number: String
    /// Indicates maximum amount of time in minutes
    @range(
        min: 0
    )
    @required
    sdk_max_timeout: Integer
    sdk_type: SdkType
}

structure SdkNextAction {
    @required
    next_action: NextActionCall
}

structure SdkNextActionData {
    @required
    next_action: NextActionCall
    order_id: String
}

structure SecretInfoToInitiateSdk with [SecretInfoToInitiateSdkMixin] {}

@mixin
structure SecretInfoToInitiateSdkMixin {
    @required
    display: String
    @required
    payment: String
}

structure Secrets with [SecretInfoToInitiateSdkMixin] {}

/// SEPA payments (Euro zone)
structure Sepa {
    /// IBAN for SEPA transfers
    @dataExamples([
        {
            json: "FR1420041010050500013M02606"
        }
    ])
    @required
    iban: String
    /// Account holder name
    @required
    name: String
    connector_recipient_id: String
}

@mixin
structure SepaAndBacsBillingDetails {
    /// The Email ID for SEPA and BACS billing
    @dataExamples([
        {
            json: "example@me.com"
        }
    ])
    email: String
    /// The billing name for SEPA and BACS billing
    @dataExamples([
        {
            json: "Jane Doe"
        }
    ])
    name: String
}

structure SepaBankDebit {
    billing_details: BankDebitDataOneOfAlt1SepaBankDebitBillingDetails
    /// International bank account number (iban) for SEPA
    @dataExamples([
        {
            json: "DE89370400440532013000"
        }
    ])
    @required
    iban: String
    /// Owner name for bank debit
    @dataExamples([
        {
            json: "A. Schneider"
        }
    ])
    @required
    bank_account_holder_name: String
}

structure SepaBankDebitAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    @dataExamples([
        {
            json: "DE8937******013000"
        }
    ])
    @required
    iban: String
    /// Bank account's owner name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    bank_account_holder_name: String
}

structure SepaBankTransfer {
    /// Bank name
    @dataExamples([
        {
            json: "Deutsche Bank"
        }
    ])
    bank_name: String
    bank_country_code: SepaBankTransferBankCountryCode
    /// Bank city
    @dataExamples([
        {
            json: "California"
        }
    ])
    bank_city: String
    /// International Bank Account Number (iban) - used in many countries for identifying a bank along with it's customer.
    @dataExamples([
        {
            json: "DE89370400440532013000"
        }
    ])
    @required
    iban: String
    /// [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it's branches
    @dataExamples([
        {
            json: "HSBCGB2LXXX"
        }
    ])
    @required
    bic: String
}

/// Masked payout method details for sepa bank transfer payout method
structure SepaBankTransferAdditionalData {
    /// Partially masked international bank account number (iban) for SEPA
    @dataExamples([
        {
            json: "DE8937******013000"
        }
    ])
    @required
    iban: String
    /// Bank name
    @dataExamples([
        {
            json: "Deutsche Bank"
        }
    ])
    bank_name: String
    bank_country_code: SepaBankTransferAdditionalDataBankCountryCode
    /// Bank city
    @dataExamples([
        {
            json: "California"
        }
    ])
    bank_city: String
    /// [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it's branches
    @dataExamples([
        {
            json: "HSBCGB2LXXX"
        }
    ])
    bic: String
}

structure SepaBankTransferAdditionalDataBankCountryCode {}

structure SepaBankTransferBankCountryCode {}

structure SepaBankTransferInstructions {
    @dataExamples([
        {
            json: "Jane Doe"
        }
    ])
    @required
    account_holder_name: String
    @dataExamples([
        {
            json: "9123456789"
        }
    ])
    @required
    bic: String
    @required
    country: String
    @dataExamples([
        {
            json: "123456789"
        }
    ])
    @required
    iban: String
    @dataExamples([
        {
            json: "U2PVVSEV4V9Y"
        }
    ])
    @required
    reference: String
}

/// SEPA Instant payments (10-second transfers)
structure SepaInstant {
    /// IBAN for instant SEPA transfers
    @dataExamples([
        {
            json: "DE89370400440532013000"
        }
    ])
    @required
    iban: String
    /// Account holder name
    @required
    name: String
    connector_recipient_id: String
}

@mixin
structure SessionTokenInfo {
    @required
    certificate: String
    @required
    certificate_keys: String
    @required
    merchant_identifier: String
    @required
    display_name: String
    @required
    initiative: ApplepayInitiative
    initiative_context: String
    merchant_business_country: SessionTokenInfoAllOf1MerchantBusinessCountry
}

structure SessionTokenInfoAllOf1MerchantBusinessCountry {}

structure SessionTokenOneOfAlt0 {}

structure SessionTokenOneOfAlt1 with [SamsungPaySessionTokenResponse] {}

structure SessionTokenOneOfAlt2 with [KlarnaSessionTokenResponse] {}

structure SessionTokenOneOfAlt3 with [PaypalSessionTokenResponse] {}

structure SessionTokenOneOfAlt4 with [ApplepaySessionTokenResponse] {}

structure SessionTokenOneOfAlt5 with [OpenBankingSessionToken] {}

structure SessionTokenOneOfAlt6 with [PazeSessionTokenResponse] {}

structure SessionTokenOneOfAlt7 with [ClickToPaySessionResponse] {}

structure SetThePaymentMethodAsDefault200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerDefaultPaymentMethodResponse
}

@error("client")
@httpError(400)
structure SetThePaymentMethodAsDefault400 {}

@error("client")
@httpError(404)
structure SetThePaymentMethodAsDefault404 {}

structure SetThePaymentMethodAsDefaultInput {
    /// The unique identifier for the Customer
    @httpLabel
    @required
    customer_id: String
    /// The unique identifier for the Payment Method
    @httpLabel
    @required
    payment_method_id: String
}

structure ShippingCost {}

structure Size {}

structure Sofort {
    billing_details: BankRedirectDataOneOfAlt13SofortBillingDetails
    @required
    country: CountryAlpha2
    /// The preferred language
    @dataExamples([
        {
            json: "en"
        }
    ])
    preferred_language: String
}

structure SplitPaymentsRequestOneOfAlt0 {
    @required
    stripe_split_payment: StripeSplitPaymentRequest
}

structure SplitPaymentsRequestOneOfAlt1 {
    @required
    adyen_split_payment: AdyenSplitData
}

structure SplitPaymentsRequestOneOfAlt2 {
    @required
    xendit_split_payment: XenditSplitRequest
}

structure SplitRefundOneOfAlt0 {
    @required
    stripe_split_refund: StripeSplitRefundRequest
}

structure SplitRefundOneOfAlt1 {
    @required
    adyen_split_refund: AdyenSplitData
}

structure SplitRefundOneOfAlt2 {
    @required
    xendit_split_refund: XenditSplitSubMerchantData
}

structure StaticRoutingAlgorithmOneOfAlt0 {
    @required
    data: RoutableConnectorChoice
}

structure StaticRoutingAlgorithmOneOfAlt1 {
    @required
    data: StaticRoutingAlgorithmOneOfAlt1Data
}

structure StaticRoutingAlgorithmOneOfAlt2 {
    @required
    data: StaticRoutingAlgorithmOneOfAlt2Data
}

structure StaticRoutingAlgorithmOneOfAlt3 {
    @required
    data: ProgramConnectorSelection
}

structure StaticRoutingAlgorithmOneOfAlt4 {
    @required
    data: ProgramThreeDsDecisionRule
}

structure StraightThroughAlgorithmOneOfAlt0 {
    @required
    data: RoutableConnectorChoice
}

structure StraightThroughAlgorithmOneOfAlt1 {
    @required
    data: StraightThroughAlgorithmOneOfAlt1Data
}

structure StraightThroughAlgorithmOneOfAlt2 {
    @required
    data: StraightThroughAlgorithmOneOfAlt2Data
}

/// Fee information to be charged on the payment being collected via Stripe
structure StripeChargeResponseData {
    /// Identifier for charge created for the payment
    charge_id: String
    @required
    charge_type: PaymentChargeType
    /// Platform fees collected on the payment
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    application_fees: Long
    /// Identifier for the reseller's account where the funds were transferred
    @required
    transfer_account_id: String
}

/// Fee information for Split Payments to be charged on the payment being collected for Stripe
structure StripeSplitPaymentRequest {
    @required
    charge_type: PaymentChargeType
    /// Platform fees to be collected on the payment
    @dataExamples([
        {
            json: 6540
        }
    ])
    @required
    application_fees: Long
    /// Identifier for the reseller's account where the funds were transferred
    @required
    transfer_account_id: String
}

/// Charge specific fields for controlling the revert of funds from either platform or connected account for Stripe. Check sub-fields for more details.
structure StripeSplitRefundRequest {
    /// Toggle for reverting the application fee that was collected for the payment.
    /// If set to false, the funds are pulled from the destination account.
    revert_platform_fee: Boolean
    /// Toggle for reverting the transfer that was made during the charge.
    /// If set to false, the funds are pulled from the main platform's account.
    revert_transfer: Boolean
}

structure SuccessBasedRoutingConfig {
    params: SuccessBasedRoutingConfigParams
    config: SuccessBasedRoutingConfigConfig
    @required
    decision_engine_configs: DecisionEngineSuccessRateData
}

@mixin
structure SuccessBasedRoutingConfigBody {
    @range(
        min: 0
    )
    min_aggregates_size: Integer
    default_success_rate: Double
    @range(
        min: 0
    )
    max_aggregates_size: Integer
    current_block_threshold: CurrentBlockThreshold
    specificity_level: SuccessRateSpecificityLevel
    exploration_percent: Double
    shuffle_on_tie_during_exploitation: Boolean
}

structure SuccessBasedRoutingConfigConfig with [SuccessBasedRoutingConfigBody] {}

structure SupportedPaymentMethod {
    @required
    payment_method: PaymentMethod
    @required
    payment_method_type: PaymentMethodType
    /// The display name of the payment method type
    @required
    payment_method_type_display_name: String
    @required
    mandates: FeatureStatus
    @required
    refunds: FeatureStatus
    @required
    supported_capture_methods: SupportedCaptureMethods
    supported_countries: SupportedCountries
    supported_currencies: SupportedCurrencies
}

@mixin
structure SurchargeDetailsResponse {
    @required
    surcharge: SurchargeResponse
    tax_on_surcharge: TaxOnSurcharge
    /// surcharge amount for this payment
    @required
    display_surcharge_amount: Double
    /// tax on surcharge amount for this payment
    @required
    display_tax_on_surcharge_amount: Double
    /// sum of display_surcharge_amount and display_tax_on_surcharge_amount
    @required
    display_total_surcharge_amount: Double
}

structure SurchargePercentage with [SurchargePercentageMixin] {}

@mixin
structure SurchargePercentageMixin {
    @required
    percentage: Float
}

structure SurchargeResponseOneOfAlt0 {
    @required
    value: MinorUnit
}

structure SurchargeResponseOneOfAlt1 {
    @required
    value: SurchargePercentage
}

structure TaxAmount {}

structure TaxOnSurcharge with [SurchargePercentageMixin] {}

structure ThirdPartySdkSessionResponse {
    @required
    secrets: SecretInfoToInitiateSdk
}

structure ThreeDsData {
    /// ThreeDS authentication url - to initiate authentication
    @required
    three_ds_authentication_url: String
    /// ThreeDS authorize url - to complete the payment authorization after authentication
    @required
    three_ds_authorize_url: String
    @required
    three_ds_method_details: ThreeDsMethodData
    @required
    poll_config: PollConfigResponse
    /// Message Version
    message_version: String
    /// Directory Server ID
    directory_server_id: String
}

/// Struct representing the output configuration for the 3DS Decision Rule Engine.
structure ThreeDSDecisionRule {
    @required
    decision: ThreeDSDecision
}

/// Represents the request to execute a 3DS decision rule.
structure ThreeDsDecisionRuleExecuteRequest {
    /// The ID of the routing algorithm to be executed.
    @required
    routing_id: String
    @required
    payment: PaymentData
    payment_method: ThreeDsDecisionRuleExecuteRequestPaymentMethod
    customer_device: CustomerDevice
    issuer: Issuer
    acquirer: Acquirer
}

structure ThreeDsDecisionRuleExecuteRequestPaymentMethod with [PaymentMethodMetaData] {}

/// Represents the response from executing a 3DS decision rule.
structure ThreeDsDecisionRuleExecuteResponse {
    @required
    decision: ThreeDSDecision
}

structure ThreeDsMethodDataOneOfAlt0 {
    /// Whether ThreeDS method data submission is required
    @required
    three_ds_method_data_submission: Boolean
    /// ThreeDS method data
    three_ds_method_data: String
    /// ThreeDS method url
    three_ds_method_url: String
}

structure TimeScale {}

structure ToggleBlocklistGuardForAParticularMerchant200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ToggleBlocklistResponse
}

@error("client")
@httpError(400)
structure ToggleBlocklistGuardForAParticularMerchant400 {}

structure ToggleBlocklistGuardForAParticularMerchantInput {
    /// Boolean value to enable/disable blocklist
    @httpQuery("status")
    @required
    status: Boolean
}

structure ToggleBlocklistResponse {
    @required
    blocklist_guard_status: String
}

structure ToggleContractRoutingAlgorithm200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure ToggleContractRoutingAlgorithm400 {}

@error("client")
@httpError(403)
structure ToggleContractRoutingAlgorithm403 {}

@error("client")
@httpError(404)
structure ToggleContractRoutingAlgorithm404 {}

@error("client")
@httpError(422)
structure ToggleContractRoutingAlgorithm422 {}

@error("server")
@httpError(500)
structure ToggleContractRoutingAlgorithm500 {}

structure ToggleContractRoutingAlgorithmInput {
    /// Merchant id
    @httpLabel
    @required
    account_id: String
    /// Profile id under which Dynamic routing needs to be toggled
    @httpLabel
    @required
    profile_id: String
    /// Feature to enable for contract based routing
    @httpQuery("enable")
    @required
    enable: DynamicRoutingFeatures
    @httpPayload
    @required
    @contentType("application/json")
    body: ContractBasedRoutingConfig
}

structure ToggleDynamicRoutingPath {
    @required
    profile_id: String
}

structure ToggleDynamicRoutingQuery {
    @required
    enable: DynamicRoutingFeatures
}

structure ToggleEliminationRoutingAlgorithm200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure ToggleEliminationRoutingAlgorithm400 {}

@error("client")
@httpError(403)
structure ToggleEliminationRoutingAlgorithm403 {}

@error("client")
@httpError(404)
structure ToggleEliminationRoutingAlgorithm404 {}

@error("client")
@httpError(422)
structure ToggleEliminationRoutingAlgorithm422 {}

@error("server")
@httpError(500)
structure ToggleEliminationRoutingAlgorithm500 {}

structure ToggleEliminationRoutingAlgorithmInput {
    /// Merchant id
    @httpLabel
    @required
    account_id: String
    /// Profile id under which Dynamic routing needs to be toggled
    @httpLabel
    @required
    profile_id: String
    /// Feature to enable for elimination based routing
    @httpQuery("enable")
    @required
    enable: DynamicRoutingFeatures
}

structure ToggleKVRequest {
    /// Status of KV for the specific merchant
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    kv_enabled: Boolean
}

structure ToggleKVResponse {
    /// The identifier for the Merchant Account
    @dataExamples([
        {
            json: "y3oqhf46pyzuxjbcn2giaqnb44"
        }
    ])
    @length(
        max: 255
    )
    @required
    merchant_id: String
    /// Status of KV for the specific merchant
    @dataExamples([
        {
            json: true
        }
    ])
    @required
    kv_enabled: Boolean
}

structure ToggleSuccessBasedDynamicRoutingAlgorithm200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure ToggleSuccessBasedDynamicRoutingAlgorithm400 {}

@error("client")
@httpError(403)
structure ToggleSuccessBasedDynamicRoutingAlgorithm403 {}

@error("client")
@httpError(404)
structure ToggleSuccessBasedDynamicRoutingAlgorithm404 {}

@error("client")
@httpError(422)
structure ToggleSuccessBasedDynamicRoutingAlgorithm422 {}

@error("server")
@httpError(500)
structure ToggleSuccessBasedDynamicRoutingAlgorithm500 {}

structure ToggleSuccessBasedDynamicRoutingAlgorithmInput {
    /// Merchant id
    @httpLabel
    @required
    account_id: String
    /// Profile id under which Dynamic routing needs to be toggled
    @httpLabel
    @required
    profile_id: String
    /// Feature to enable for success based routing
    @httpQuery("enable")
    @required
    enable: DynamicRoutingFeatures
}

structure TokenizationData {}

structure TokenizeCardRequest {
    /// Card Number
    @dataExamples([
        {
            json: "4111111145551142"
        }
    ])
    @required
    raw_card_number: String
    /// Card Expiry Month
    @dataExamples([
        {
            json: "10"
        }
    ])
    @required
    card_expiry_month: String
    /// Card Expiry Year
    @dataExamples([
        {
            json: "25"
        }
    ])
    @required
    card_expiry_year: String
    /// The CVC number for the card
    @dataExamples([
        {
            json: "242"
        }
    ])
    card_cvc: String
    /// Card Holder Name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    card_holder_name: String
    /// Card Holder's Nick Name
    @dataExamples([
        {
            json: "John Doe"
        }
    ])
    nick_name: String
    /// Card Issuing Country
    card_issuing_country: String
    card_network: TokenizeCardRequestCardNetwork
    /// Issuer Bank for Card
    card_issuer: String
    card_type: CardType
}

structure TokenizeCardRequestCardNetwork {}

structure TokenizeDataRequestOneOfAlt0 {
    @required
    card: TokenizeCardRequest
}

structure TokenizeDataRequestOneOfAlt1 {
    @required
    existing_payment_method: TokenizePaymentMethodRequest
}

structure TokenizePaymentMethodRequest {
    /// The CVC number for the card
    @dataExamples([
        {
            json: "242"
        }
    ])
    card_cvc: String
}

/// The response body of list initial delivery attempts api call.
structure TotalEventsResponse {
    @required
    events: Events
    /// Count of total events
    @required
    total_count: Long
}

@mixin
structure TransactionDetailsUiConfiguration {
    /// Position of the key-value pair in the UI
    @dataExamples([
        {
            json: 5
        }
    ])
    position: Integer
    /// Whether the key should be bold
    @dataExamples([
        {
            json: true
        }
    ])
    is_key_bold: Boolean
    /// Whether the value should be bold
    @dataExamples([
        {
            json: true
        }
    ])
    is_value_bold: Boolean
}

structure Trustly {
    @required
    country: CountryAlpha2
}

structure UiConfiguration with [TransactionDetailsUiConfiguration] {}

structure UnblockAFingerprint200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: BlocklistResponse
}

@error("client")
@httpError(400)
structure UnblockAFingerprint400 {}

structure UnblockAFingerprintInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: BlocklistRequest
}

structure UpdateACustomer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerResponse
}

@error("client")
@httpError(404)
structure UpdateACustomer404 {}

structure UpdateACustomerInput {
    /// The unique identifier for the Customer
    @httpLabel
    @required
    customer_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: CustomerUpdateRequest
}

structure UpdateAMerchantAccount200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantAccountResponse
}

@error("client")
@httpError(404)
structure UpdateAMerchantAccount404 {}

structure UpdateAMerchantAccountInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantAccountUpdate
}

structure UpdateAMerchantConnector200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantConnectorResponse
}

@error("client")
@httpError(401)
structure UpdateAMerchantConnector401 {}

@error("client")
@httpError(404)
structure UpdateAMerchantConnector404 {}

structure UpdateAMerchantConnectorInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    /// The unique identifier for the Merchant Connector
    @httpLabel
    @required
    merchant_connector_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: MerchantConnectorUpdate
}

structure UpdateAnAPIKey200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RetrieveApiKeyResponse
}

@error("client")
@httpError(404)
structure UpdateAnAPIKey404 {}

structure UpdateAnAPIKeyInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    merchant_id: String
    /// The unique identifier for the API Key
    @httpLabel
    @required
    key_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: UpdateApiKeyRequest
}

structure UpdateAnOrganization200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: OrganizationResponse
}

@error("client")
@httpError(400)
structure UpdateAnOrganization400 {}

structure UpdateAnOrganizationInput {
    /// The unique identifier for the Organization
    @httpLabel
    @required
    id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: OrganizationUpdateRequest
}

structure UpdateAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsCreateResponseOpenApi
}

@error("client")
@httpError(400)
structure UpdateAPayment400 {}

structure UpdateAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsUpdateRequest
}

structure UpdateAPaymentMethod200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodResponse
}

@error("client")
@httpError(404)
structure UpdateAPaymentMethod404 {}

structure UpdateAPaymentMethodInput {
    /// The unique identifier for the Payment Method
    @httpLabel
    @required
    method_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentMethodUpdate
}

structure UpdateAPayout200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutCreateResponse
}

@error("client")
@httpError(400)
structure UpdateAPayout400 {}

structure UpdateAPayoutInput {
    /// The identifier for payout
    @httpLabel
    @required
    payout_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PayoutUpdateRequest
}

/// The request body for updating an API Key.
structure UpdateApiKeyRequest {
    /// A unique name for the API Key to help you identify it.
    @dataExamples([
        {
            json: "Sandbox integration key"
        }
    ])
    @length(
        max: 64
    )
    name: String
    /// A description to provide more context about the API Key.
    @dataExamples([
        {
            json: "Key used by our developers to integrate with the sandbox environment"
        }
    ])
    @length(
        max: 256
    )
    description: String
    expiration: Expiration
}

structure UpdateAProfile200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileResponse
}

@error("client")
@httpError(400)
structure UpdateAProfile400 {}

structure UpdateAProfileAcquirer200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileAcquirerResponse
}

@error("client")
@httpError(400)
structure UpdateAProfileAcquirer400 {}

structure UpdateAProfileAcquirerInput {
    /// The unique identifier for the Profile
    @httpLabel
    @required
    profile_id: String
    /// The unique identifier for the Profile Acquirer
    @httpLabel
    @required
    profile_acquirer_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileAcquirerUpdate
}

structure UpdateAProfileInput {
    /// The unique identifier for the merchant account
    @httpLabel
    @required
    account_id: String
    /// The unique identifier for the profile
    @httpLabel
    @required
    profile_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileCreate
}

structure UpdateARefund200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundResponse
}

@error("client")
@httpError(400)
structure UpdateARefund400 {}

structure UpdateARefundInput {
    /// The identifier for refund
    @httpLabel
    @required
    refund_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: RefundUpdateRequest
}

structure UpdateContractBasedDynamicRoutingConfigs200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure UpdateContractBasedDynamicRoutingConfigs400 {}

@error("client")
@httpError(403)
structure UpdateContractBasedDynamicRoutingConfigs403 {}

@error("client")
@httpError(404)
structure UpdateContractBasedDynamicRoutingConfigs404 {}

@error("client")
@httpError(422)
structure UpdateContractBasedDynamicRoutingConfigs422 {}

@error("server")
@httpError(500)
structure UpdateContractBasedDynamicRoutingConfigs500 {}

structure UpdateContractBasedDynamicRoutingConfigsInput {
    /// Merchant id
    @httpLabel
    @required
    account_id: String
    /// Profile id under which Dynamic routing needs to be toggled
    @httpLabel
    @required
    profile_id: String
    /// Contract based routing algorithm id which was last activated to update the config
    @httpLabel
    @required
    algorithm_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: ContractBasedRoutingConfig
}

structure UpdateDefaultConfigsForAllProfiles200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: ProfileDefaultRoutingConfig
}

@error("client")
@httpError(400)
structure UpdateDefaultConfigsForAllProfiles400 {}

@error("client")
@httpError(403)
structure UpdateDefaultConfigsForAllProfiles403 {}

@error("client")
@httpError(404)
structure UpdateDefaultConfigsForAllProfiles404 {}

@error("client")
@httpError(422)
structure UpdateDefaultConfigsForAllProfiles422 {}

@error("server")
@httpError(500)
structure UpdateDefaultConfigsForAllProfiles500 {}

structure UpdateDefaultConfigsForAllProfilesInput {
    /// The unique identifier for a profile
    @httpLabel
    @required
    profile_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: UpdateDefaultConfigsForAllProfilesInputBody
}

structure UpdateDefaultFallbackConfig200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: UpdateDefaultFallbackConfig200Body
}

@error("client")
@httpError(400)
structure UpdateDefaultFallbackConfig400 {}

@error("client")
@httpError(422)
structure UpdateDefaultFallbackConfig422 {}

@error("server")
@httpError(500)
structure UpdateDefaultFallbackConfig500 {}

structure UpdateDefaultFallbackConfigInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: UpdateDefaultFallbackConfigInputBody
}

structure UpdateGsmRule200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmResponse
}

@error("client")
@httpError(400)
structure UpdateGsmRule400 {}

structure UpdateGsmRuleInput {
    @httpPayload
    @required
    @contentType("application/json")
    body: GsmUpdateRequest
}

structure UpdateMetadataForAPayment200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsUpdateMetadataResponse
}

@error("client")
@httpError(400)
structure UpdateMetadataForAPayment400 {}

structure UpdateMetadataForAPaymentInput {
    /// The identifier for payment
    @httpLabel
    @required
    payment_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: PaymentsUpdateMetadataRequest
}

structure UpdateSuccessBasedDynamicRoutingConfigs200 {
    @httpPayload
    @required
    @contentType("application/json")
    body: RoutingDictionaryRecord
}

@error("client")
@httpError(400)
structure UpdateSuccessBasedDynamicRoutingConfigs400 {}

@error("client")
@httpError(403)
structure UpdateSuccessBasedDynamicRoutingConfigs403 {}

@error("client")
@httpError(404)
structure UpdateSuccessBasedDynamicRoutingConfigs404 {}

@error("client")
@httpError(422)
structure UpdateSuccessBasedDynamicRoutingConfigs422 {}

@error("server")
@httpError(500)
structure UpdateSuccessBasedDynamicRoutingConfigs500 {}

structure UpdateSuccessBasedDynamicRoutingConfigsInput {
    /// Merchant id
    @httpLabel
    @required
    account_id: String
    /// Profile id under which Dynamic routing needs to be toggled
    @httpLabel
    @required
    profile_id: String
    /// Success based routing algorithm id which was last activated to update the config
    @httpLabel
    @required
    algorithm_id: String
    @httpPayload
    @required
    @contentType("application/json")
    body: SuccessBasedRoutingConfig
}

structure UpiAdditionalDataOneOfAlt0 {
    @required
    upi_collect: UpiCollectAdditionalData
}

structure UpiAdditionalDataOneOfAlt1 {
    @required
    upi_intent: UpiIntentData
}

structure UpiCollectAdditionalData {
    /// Masked VPA ID
    @dataExamples([
        {
            json: "ab********@okhdfcbank"
        }
    ])
    vpa_id: String
}

structure UpiCollectData {
    @dataExamples([
        {
            json: "successtest@iata"
        }
    ])
    vpa_id: String
}

structure UpiDataOneOfAlt0 {
    @required
    upi_collect: UpiCollectData
}

structure UpiDataOneOfAlt1 {
    @required
    upi_intent: UpiIntentData
}

structure UpiResponse {}

structure UserAddressCountry {
    @required
    options: FieldTypeOneOfAlt18UserAddressCountryOptions
}

structure UserCountry {
    @required
    options: FieldTypeOneOfAlt9UserCountryOptions
}

structure UserCurrency {
    @required
    options: FieldTypeOneOfAlt10UserCurrencyOptions
}

structure UserShippingAddressCountry {
    @required
    options: FieldTypeOneOfAlt25UserShippingAddressCountryOptions
}

structure ValueTypeOneOfAlt0 {
    @required
    value: MinorUnit
}

structure ValueTypeOneOfAlt1 {
    /// Represents an enum variant
    @required
    value: String
}

structure ValueTypeOneOfAlt2 {
    @required
    value: MetadataValue
}

structure ValueTypeOneOfAlt3 {
    /// Represents a arbitrary String value
    @required
    value: String
}

structure ValueTypeOneOfAlt4 {
    @required
    value: ValueTypeOneOfAlt4Value
}

structure ValueTypeOneOfAlt5 {
    @required
    value: ValueTypeOneOfAlt5Value
}

structure ValueTypeOneOfAlt6 {
    @required
    value: ValueTypeOneOfAlt6Value
}

structure Venmo {
    /// mobile number linked to venmo account
    @dataExamples([
        {
            json: "16608213349"
        }
    ])
    @required
    telephone_number: String
}

/// Masked payout method details for venmo wallet payout method
structure VenmoAdditionalData {
    /// mobile number linked to venmo account
    @dataExamples([
        {
            json: "******* 3349"
        }
    ])
    telephone_number: String
}

structure VoucherDataOneOfAlt0 {
    @required
    boleto: BoletoVoucherData
}

structure VoucherDataOneOfAlt10 {
    @required
    mini_stop: JCSVoucherData
}

structure VoucherDataOneOfAlt11 {
    @required
    family_mart: JCSVoucherData
}

structure VoucherDataOneOfAlt12 {
    @required
    seicomart: JCSVoucherData
}

structure VoucherDataOneOfAlt13 {
    @required
    pay_easy: JCSVoucherData
}

structure VoucherDataOneOfAlt5 {
    @required
    alfamart: AlfamartVoucherData
}

structure VoucherDataOneOfAlt6 {
    @required
    indomaret: IndomaretVoucherData
}

structure VoucherDataOneOfAlt8 {
    @required
    seven_eleven: JCSVoucherData
}

structure VoucherDataOneOfAlt9 {
    @required
    lawson: JCSVoucherData
}

structure VoucherResponse {}

structure WalletAdditionalDataForCard {
    /// Last 4 digits of the card number
    @required
    last4: String
    /// The information of the payment method
    @required
    card_network: String
    /// The type of payment method
    type: String
}

structure WalletDataOneOfAlt0 {
    @required
    ali_pay_qr: AliPayQr
}

structure WalletDataOneOfAlt1 {
    @required
    ali_pay_redirect: AliPayRedirection
}

structure WalletDataOneOfAlt10 {
    @required
    apple_pay_third_party_sdk: ApplePayThirdPartySdkData
}

structure WalletDataOneOfAlt11 {
    /// Wallet data for DANA redirect flow
    @required
    dana_redirect: Document
}

structure WalletDataOneOfAlt12 {
    @required
    google_pay: GooglePayWalletData
}

structure WalletDataOneOfAlt13 {
    @required
    google_pay_redirect: GooglePayRedirectData
}

structure WalletDataOneOfAlt14 {
    @required
    google_pay_third_party_sdk: GooglePayThirdPartySdkData
}

structure WalletDataOneOfAlt15 {
    @required
    mb_way_redirect: MbWayRedirection
}

structure WalletDataOneOfAlt16 {
    @required
    mobile_pay_redirect: MobilePayRedirection
}

structure WalletDataOneOfAlt17 {
    @required
    paypal_redirect: PaypalRedirection
}

structure WalletDataOneOfAlt18 {
    @required
    paypal_sdk: PayPalWalletData
}

structure WalletDataOneOfAlt19 {
    @required
    paze: PazeWalletData
}

structure WalletDataOneOfAlt2 {
    @required
    ali_pay_hk_redirect: AliPayHkRedirection
}

structure WalletDataOneOfAlt20 {
    @required
    samsung_pay: SamsungPayWalletData
}

structure WalletDataOneOfAlt21 {
    /// Wallet data for Twint Redirection
    @required
    twint_redirect: Document
}

structure WalletDataOneOfAlt22 {
    /// Wallet data for Vipps Redirection
    @required
    vipps_redirect: Document
}

structure WalletDataOneOfAlt23 {
    @required
    touch_n_go_redirect: TouchNGoRedirection
}

structure WalletDataOneOfAlt24 {
    @required
    we_chat_pay_redirect: WeChatPayRedirection
}

structure WalletDataOneOfAlt25 {
    @required
    we_chat_pay_qr: WeChatPayQr
}

structure WalletDataOneOfAlt26 {
    @required
    cashapp_qr: CashappQr
}

structure WalletDataOneOfAlt27 {
    @required
    swish_qr: SwishQrData
}

structure WalletDataOneOfAlt28 {
    @required
    mifinity: MifinityData
}

structure WalletDataOneOfAlt29 {
    @required
    revolut_pay: RevolutPayData
}

structure WalletDataOneOfAlt3 {
    @required
    amazon_pay_redirect: AmazonPayRedirectData
}

structure WalletDataOneOfAlt4 {
    @required
    momo_redirect: MomoRedirection
}

structure WalletDataOneOfAlt5 {
    @required
    kakao_pay_redirect: KakaoPayRedirection
}

structure WalletDataOneOfAlt6 {
    @required
    go_pay_redirect: GoPayRedirection
}

structure WalletDataOneOfAlt7 {
    @required
    gcash_redirect: GcashRedirection
}

structure WalletDataOneOfAlt8 {
    @required
    apple_pay: ApplePayWalletData
}

structure WalletDataOneOfAlt9 {
    @required
    apple_pay_redirect: ApplePayRedirectData
}

structure WalletOneOfAlt0 {
    @required
    paypal: Paypal
}

structure WalletOneOfAlt1 {
    @required
    venmo: Venmo
}

structure WalletResponse {}

structure WalletResponseDataOneOfAlt0 {
    @required
    apple_pay: WalletAdditionalDataForCard
}

structure WalletResponseDataOneOfAlt1 {
    @required
    google_pay: WalletAdditionalDataForCard
}

structure WalletResponseDataOneOfAlt2 {
    @required
    samsung_pay: WalletAdditionalDataForCard
}

@mixin
structure WebhookDetails {
    /// The version for Webhook
    @dataExamples([
        {
            json: "1.0.2"
        }
    ])
    @length(
        max: 255
    )
    webhook_version: String
    /// The user name for Webhook login
    @dataExamples([
        {
            json: "ekart_retail"
        }
    ])
    @length(
        max: 255
    )
    webhook_username: String
    /// The password for Webhook login
    @dataExamples([
        {
            json: "ekart@123"
        }
    ])
    @length(
        max: 255
    )
    webhook_password: String
    /// The url for the webhook endpoint
    @dataExamples([
        {
            json: "www.ekart.com/webhooks"
        }
    ])
    webhook_url: String
    /// If this property is true, a webhook message is posted whenever a new payment is created
    @dataExamples([
        {
            json: true
        }
    ])
    payment_created_enabled: Boolean
    /// If this property is true, a webhook message is posted whenever a payment is successful
    @dataExamples([
        {
            json: true
        }
    ])
    payment_succeeded_enabled: Boolean
    /// If this property is true, a webhook message is posted whenever a payment fails
    @dataExamples([
        {
            json: true
        }
    ])
    payment_failed_enabled: Boolean
    @required
    payment_statuses_enabled: PaymentStatusesEnabled
    @required
    refund_statuses_enabled: RefundStatusesEnabled
    payout_statuses_enabled: PayoutStatusesEnabled
}

structure XenditChargeResponseDataOneOfAlt0 {
    @required
    multiple_splits: XenditMultipleSplitResponse
}

structure XenditChargeResponseDataOneOfAlt1 {
    @required
    single_split: XenditSplitSubMerchantData
}

/// Fee information to be charged on the payment being collected via xendit
structure XenditMultipleSplitRequest {
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    @required
    name: String
    /// Description to identify fee rule
    @required
    description: String
    /// The sub-account user-id that you want to make this transaction for.
    for_user_id: String
    @required
    routes: XenditMultipleSplitRequestRoutes
}

/// Fee information charged on the payment being collected via xendit
structure XenditMultipleSplitResponse {
    /// Identifier for split rule created for the payment
    @required
    split_rule_id: String
    /// The sub-account user-id that you want to make this transaction for.
    for_user_id: String
    /// Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-merchant types.
    @required
    name: String
    /// Description to identify fee rule
    @required
    description: String
    @required
    routes: XenditMultipleSplitResponseRoutes
}

structure XenditSplitRequestOneOfAlt0 {
    @required
    multiple_splits: XenditMultipleSplitRequest
}

structure XenditSplitRequestOneOfAlt1 {
    @required
    single_split: XenditSplitSubMerchantData
}

/// Fee information to be charged on the payment being collected via xendit
structure XenditSplitRoute {
    flat_amount: FlatAmount
    /// Amount of payments to be split, using a percent rate as unit
    percent_amount: Long
    @required
    currency: Currency
    /// ID of the destination account where the amount will be routed to
    @required
    destination_account_id: String
    /// Reference ID which acts as an identifier of the route itself
    @required
    reference_id: String
}

/// Fee information to be charged on the payment being collected for sub-merchant via xendit
structure XenditSplitSubMerchantData {
    /// The sub-account user-id that you want to make this transaction for.
    @required
    for_user_id: String
}

/// Object to filter the customer countries for which the payment method is displayed
@discriminated("type")
union AcceptedCountries {
    alt0: AcceptedCountriesOneOfAlt0
    alt1: AcceptedCountriesOneOfAlt1
    alt2: ComponentsSchemasAcceptedCountriesOneOfAlt2
}

@discriminated("type")
union AcceptedCurrencies {
    alt0: AcceptedCurrenciesOneOfAlt0
    alt1: AcceptedCurrenciesOneOfAlt1
    alt2: ComponentsSchemasAcceptedCurrenciesOneOfAlt2
}

union AdditionalMerchantData {
    open_banking_recipient_data: MerchantRecipientData
}

/// Masked payout method details for storing in db
union AdditionalPayoutMethodData {
    Card: CardAdditionalData
    Bank: BankAdditionalData
    Wallet: WalletAdditionalData
}

@untagged
union ApiKeyExpiration {
    alt0: ApiKeyExpirationOneOfAlt0
    Timestamp: Timestamp
}

@untagged
union ApplePaySessionResponse {
    ThirdPartySdkSessionResponse: ThirdPartySdkSessionResponse
    NoThirdPartySdkSessionResponse: NoThirdPartySdkSessionResponse
    NullObject: NullObject
}

@untagged
union Bank {
    AchBankTransfer: AchBankTransfer
    BacsBankTransfer: BacsBankTransfer
    SepaBankTransfer: SepaBankTransfer
    PixBankTransfer: PixBankTransfer
}

/// Masked payout method details for bank payout method
@untagged
union BankAdditionalData {
    AchBankTransferAdditionalData: AchBankTransferAdditionalData
    BacsBankTransferAdditionalData: BacsBankTransferAdditionalData
    SepaBankTransferAdditionalData: SepaBankTransferAdditionalData
    PixBankTransferAdditionalData: PixBankTransferAdditionalData
}

union BankDebitAdditionalData {
    ach: AchBankDebitAdditionalData
    bacs: BacsBankDebitAdditionalData
    becs: BecsBankDebitAdditionalData
    sepa: SepaBankDebitAdditionalData
}

union BankDebitData {
    ach_bank_debit: AchBankDebit
    sepa_bank_debit: SepaBankDebit
    becs_bank_debit: BecsBankDebit
    bacs_bank_debit: BacsBankDebit
}

union BankRedirectData {
    bancontact_card: BancontactCard
    bizum: Document
    blik: Blik
    eps: Eps
    giropay: Giropay
    ideal: Ideal
    interac: Interac
    online_banking_czech_republic: OnlineBankingCzechRepublic
    online_banking_finland: OnlineBankingFinland
    online_banking_poland: OnlineBankingPoland
    online_banking_slovakia: OnlineBankingSlovakia
    open_banking_uk: OpenBankingUk
    przelewy24: Przelewy24
    sofort: Sofort
    trustly: Trustly
    online_banking_fpx: OnlineBankingFpx
    online_banking_thailand: OnlineBankingThailand
    local_bank_redirect: Document
    eft: Eft
}

union BankRedirectDetails {
    BancontactCard: BancontactBankRedirectAdditionalData
    Blik: BlikBankRedirectAdditionalData
    Giropay: GiropayBankRedirectAdditionalData
}

union BankTransferAdditionalData {
    ach: Document
    sepa: Document
    bacs: Document
    multibanco: Document
    permata: Document
    bca: Document
    bni_va: Document
    bri_va: Document
    cimb_va: Document
    danamon_va: Document
    mandiri_va: Document
    pix: PixBankTransferAdditionalData
    pse: Document
    local_bank_transfer: LocalBankTransferAdditionalData
    instant_bank_transfer: Document
    instant_bank_transfer_finland: Document
    instant_bank_transfer_poland: Document
}

union BankTransferData {
    ach_bank_transfer: AchBankTransfer
    sepa_bank_transfer: SepaBankTransfer
    bacs_bank_transfer: BacsBankTransfer
    multibanco_bank_transfer: MultibancoBankTransfer
    permata_bank_transfer: PermataBankTransfer
    bca_bank_transfer: BcaBankTransfer
    bni_va_bank_transfer: BniVaBankTransfer
    bri_va_bank_transfer: BriVaBankTransfer
    cimb_va_bank_transfer: CimbVaBankTransfer
    danamon_va_bank_transfer: DanamonVaBankTransfer
    mandiri_va_bank_transfer: MandiriVaBankTransfer
    pix: Pix
    pse: Document
    local_bank_transfer: LocalBankTransfer
    instant_bank_transfer: Document
    instant_bank_transfer_finland: Document
    instant_bank_transfer_poland: Document
}

union BankTransferInstructions {
    doku_bank_transfer_instructions: DokuBankTransferInstructions
    ach_credit_transfer: AchTransfer
    sepa_bank_instructions: SepaBankTransferInstructions
    bacs_bank_instructions: BacsBankTransferInstructions
    multibanco: MultibancoTransferInstructions
}

@discriminated("type")
union BlocklistRequest {
    alt0: BlocklistRequestOneOfAlt0
    alt1: BlocklistRequestOneOfAlt1
    alt2: BlocklistRequestOneOfAlt2
}

union CardRedirectData {
    knet: Document
    benefit: Document
    momo_atm: Document
    card_redirect: Document
}

/// Charge Information
union ConnectorChargeResponseData {
    stripe_split_payment: StripeChargeResponseData
    adyen_split_payment: AdyenSplitData
    xendit_split_payment: XenditChargeResponseData
}

@discriminated("type")
union ConnectorSelection {
    alt0: ConnectorSelectionOneOfAlt0
    alt1: ConnectorSelectionOneOfAlt1
}

@untagged
union DynamicRoutingAlgorithm {
    EliminationRoutingConfig: EliminationRoutingConfig
    SuccessBasedRoutingConfig: SuccessBasedRoutingConfig
    ContractBasedRoutingConfig: ContractBasedRoutingConfig
}

union ElementSize {
    Variants: SizeVariants
    Percentage: Integer
    Pixels: Integer
}

/// Possible field type of required fields in payment_method_data
@untagged
union FieldType {
    alt0: FieldTypeOneOfAlt0
    alt1: FieldTypeOneOfAlt1
    alt2: FieldTypeOneOfAlt2
    alt3: FieldTypeOneOfAlt3
    alt4: FieldTypeOneOfAlt4
    alt5: FieldTypeOneOfAlt5
    alt6: FieldTypeOneOfAlt6
    alt7: FieldTypeOneOfAlt7
    alt8: FieldTypeOneOfAlt8
    alt9: FieldTypeOneOfAlt9
    alt10: FieldTypeOneOfAlt10
    alt11: FieldTypeOneOfAlt11
    alt12: FieldTypeOneOfAlt12
    alt13: FieldTypeOneOfAlt13
    alt14: FieldTypeOneOfAlt14
    alt15: FieldTypeOneOfAlt15
    alt16: FieldTypeOneOfAlt16
    alt17: FieldTypeOneOfAlt17
    alt18: FieldTypeOneOfAlt18
    alt19: FieldTypeOneOfAlt19
    alt20: FieldTypeOneOfAlt20
    alt21: FieldTypeOneOfAlt21
    alt22: FieldTypeOneOfAlt22
    alt23: FieldTypeOneOfAlt23
    alt24: FieldTypeOneOfAlt24
    alt25: FieldTypeOneOfAlt25
    alt26: FieldTypeOneOfAlt26
    alt27: FieldTypeOneOfAlt27
    alt28: FieldTypeOneOfAlt28
    alt29: FieldTypeOneOfAlt29
    alt30: FieldTypeOneOfAlt30
    alt31: FieldTypeOneOfAlt31
    alt32: FieldTypeOneOfAlt32
    alt33: FieldTypeOneOfAlt33
    alt34: FieldTypeOneOfAlt34
    alt35: FieldTypeOneOfAlt35
    alt36: FieldTypeOneOfAlt36
    alt37: FieldTypeOneOfAlt37
    alt38: FieldTypeOneOfAlt38
    alt39: FieldTypeOneOfAlt39
    alt40: FieldTypeOneOfAlt40
    alt41: FieldTypeOneOfAlt41
    alt42: FieldTypeOneOfAlt42
    alt43: FieldTypeOneOfAlt43
    alt44: FieldTypeOneOfAlt44
    alt45: FieldTypeOneOfAlt45
    alt46: FieldTypeOneOfAlt46
}

union GiftCardAdditionalData {
    givex: GivexGiftCardAdditionalData
    pay_safe_card: Document
}

union GiftCardData {
    givex: GiftCardDetails
    pay_safe_card: Document
}

@untagged
union GpaySessionTokenResponse {
    GooglePayThirdPartySdk: GooglePayThirdPartySdk
    GooglePaySessionResponse: GooglePaySessionResponse
}

@discriminated("method_key")
union IframeData {
    alt0: IframeDataOneOfAlt0
}

@untagged
union LinkedRoutingConfigRetrieveResponse {
    RoutingRetrieveResponse: RoutingRetrieveResponse
    alt1: LinkedRoutingConfigRetrieveResponseOneOfAlt1
}

union MandateType {
    single_use: MandateAmountData
    multi_use: MultiUse
}

union MerchantAccountData {
    iban: Iban
    bacs: Bacs
    faster_payments: FasterPayments
    sepa: Sepa
    sepa_instant: SepaInstant
    elixir: Elixir
    bankgiro: Bankgiro
    plusgiro: Plusgiro
}

union MerchantRecipientData {
    connector_recipient_id: String
    wallet_id: String
    account_data: MerchantAccountData
}

union MobilePaymentData {
    direct_carrier_billing: DirectCarrierBilling
}

@discriminated("type")
union NextActionData {
    alt0: NextActionDataOneOfAlt0
    alt1: NextActionDataOneOfAlt1
    alt2: NextActionDataOneOfAlt2
    alt3: NextActionDataOneOfAlt3
    alt4: NextActionDataOneOfAlt4
    alt5: NextActionDataOneOfAlt5
    alt6: NextActionDataOneOfAlt6
    alt7: NextActionDataOneOfAlt7
    alt8: NextActionDataOneOfAlt8
    alt9: NextActionDataOneOfAlt9
    alt10: NextActionDataOneOfAlt10
    alt11: NextActionDataOneOfAlt11
}

union OpenBankingData {
    open_banking_pis: Document
}

@discriminated("type")
union OutgoingWebhookContent {
    alt0: OutgoingWebhookContentOneOfAlt0
    alt1: OutgoingWebhookContentOneOfAlt1
    alt2: OutgoingWebhookContentOneOfAlt2
    alt3: OutgoingWebhookContentOneOfAlt3
    alt4: OutgoingWebhookContentOneOfAlt4
}

union PayLaterData {
    klarna_redirect: KlarnaRedirect
    klarna_sdk: PayLaterDataOneOfAlt1KlarnaSdk
    affirm_redirect: Document
    afterpay_clearpay_redirect: AfterpayClearpayRedirect
    pay_bright_redirect: Document
    walley_redirect: Document
    alma_redirect: Document
    atome_redirect: Document
}

union PaymentChargeType {
    Stripe: StripeChargeType
}

union PaymentMethodCreateData {
    card: CardDetail
}

@untagged
union PaymentMethodData {
    alt0: PaymentMethodDataOneOfAlt0
    alt1: PaymentMethodDataOneOfAlt1
    alt2: PaymentMethodDataOneOfAlt2
    alt3: PaymentMethodDataOneOfAlt3
    alt4: PaymentMethodDataOneOfAlt4
    alt5: PaymentMethodDataOneOfAlt5
    alt6: PaymentMethodDataOneOfAlt6
    alt7: PaymentMethodDataOneOfAlt7
    alt8: PaymentMethodDataOneOfAlt8
    alt9: PaymentMethodDataOneOfAlt9
    alt10: PaymentMethodDataOneOfAlt10
    alt11: PaymentMethodDataOneOfAlt11
    alt12: PaymentMethodDataOneOfAlt12
    alt13: PaymentMethodDataOneOfAlt13
    alt14: PaymentMethodDataOneOfAlt14
    alt15: PaymentMethodDataOneOfAlt15
    alt16: PaymentMethodDataOneOfAlt16
}

union PaymentMethodDataResponse {
    card: CardResponse
    bank_transfer: BankTransferResponse
    wallet: WalletResponse
    pay_later: PaylaterResponse
    bank_redirect: BankRedirectResponse
    crypto: CryptoResponse
    bank_debit: BankDebitResponse
    mandate_payment: Document
    reward: Document
    real_time_payment: RealTimePaymentDataResponse
    upi: UpiResponse
    voucher: VoucherResponse
    gift_card: GiftCardResponse
    card_redirect: CardRedirectResponse
    card_token: CardTokenResponse
    open_banking: OpenBankingResponse
    mobile_payment: MobilePaymentResponse
}

@untagged
union PaymentMethodSpecificFeatures {
    CardSpecificFeatures: CardSpecificFeatures
}

@discriminated("payment_processing_details_at")
union PaymentProcessingDetailsAt {
    alt0: PaymentProcessingDetailsAtOneOfAlt0
    alt1: ComponentsSchemasPaymentProcessingDetailsAtOneOfAlt1
}

/// The payout method information required for carrying out a payout
union PayoutMethodData {
    card: CardPayout
    bank: Bank
    wallet: Wallet
}

/// The payout method information for response
union PayoutMethodDataResponse {
    card: CardAdditionalData
    bank: BankAdditionalData
    wallet: WalletAdditionalData
}

union RealTimePaymentData {
    fps: Document
    duit_now: Document
    prompt_pay: Document
    viet_qr: Document
}

/// Details required for recurring payment
@discriminated("type")
union RecurringDetails {
    alt0: RecurringDetailsOneOfAlt0
    alt1: RecurringDetailsOneOfAlt1
    alt2: RecurringDetailsOneOfAlt2
    alt3: RecurringDetailsOneOfAlt3
}

union RelayData {
    refund: RelayRefundRequestData
}

@untagged
union RoutingAlgorithmWrapper {
    StaticRoutingAlgorithm: StaticRoutingAlgorithm
    DynamicRoutingAlgorithm: DynamicRoutingAlgorithm
}

@untagged
union RoutingKind {
    RoutingDictionary: RoutingDictionary
    alt1: RoutingKindOneOfAlt1
}

@untagged
union SamsungPayWalletCredentials {
    SamsungPayWebWalletData: SamsungPayWebWalletData
    SamsungPayAppWalletData: SamsungPayAppWalletData
}

@discriminated("wallet_name")
union SessionToken {
    alt0: SessionTokenOneOfAlt0
    alt1: SessionTokenOneOfAlt1
    alt2: SessionTokenOneOfAlt2
    alt3: SessionTokenOneOfAlt3
    alt4: SessionTokenOneOfAlt4
    alt5: SessionTokenOneOfAlt5
    alt6: SessionTokenOneOfAlt6
    alt7: SessionTokenOneOfAlt7
    alt8: ComponentsSchemasSessionTokenOneOfAlt8
}

/// Fee information for Split Payments to be charged on the payment being collected
union SplitPaymentsRequest {
    stripe_split_payment: StripeSplitPaymentRequest
    adyen_split_payment: AdyenSplitData
    xendit_split_payment: XenditSplitRequest
}

/// Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.
union SplitRefund {
    stripe_split_refund: StripeSplitRefundRequest
    adyen_split_refund: AdyenSplitData
    xendit_split_refund: XenditSplitSubMerchantData
}

@discriminated("type")
union StaticRoutingAlgorithm {
    alt0: StaticRoutingAlgorithmOneOfAlt0
    alt1: StaticRoutingAlgorithmOneOfAlt1
    alt2: StaticRoutingAlgorithmOneOfAlt2
    alt3: StaticRoutingAlgorithmOneOfAlt3
    alt4: StaticRoutingAlgorithmOneOfAlt4
}

@discriminated("type")
union StraightThroughAlgorithm {
    alt0: StraightThroughAlgorithmOneOfAlt0
    alt1: StraightThroughAlgorithmOneOfAlt1
    alt2: StraightThroughAlgorithmOneOfAlt2
}

@discriminated("type")
union SurchargeResponse {
    alt0: SurchargeResponseOneOfAlt0
    alt1: SurchargeResponseOneOfAlt1
}

@discriminated("three_ds_method_key")
union ThreeDsMethodData {
    alt0: ThreeDsMethodDataOneOfAlt0
}

union TokenizeDataRequest {
    card: TokenizeCardRequest
    existing_payment_method: TokenizePaymentMethodRequest
}

union UpiAdditionalData {
    upi_collect: UpiCollectAdditionalData
    upi_intent: UpiIntentData
}

union UpiData {
    upi_collect: UpiCollectData
    upi_intent: UpiIntentData
}

/// Represents a value in the DSL
@discriminated("type")
union ValueType {
    alt0: ValueTypeOneOfAlt0
    alt1: ValueTypeOneOfAlt1
    alt2: ValueTypeOneOfAlt2
    alt3: ValueTypeOneOfAlt3
    alt4: ValueTypeOneOfAlt4
    alt5: ValueTypeOneOfAlt5
    alt6: ValueTypeOneOfAlt6
}

@untagged
union VoucherData {
    alt0: VoucherDataOneOfAlt0
    alt1: VoucherDataOneOfAlt1
    alt2: VoucherDataOneOfAlt2
    alt3: VoucherDataOneOfAlt3
    alt4: VoucherDataOneOfAlt4
    alt5: VoucherDataOneOfAlt5
    alt6: VoucherDataOneOfAlt6
    alt7: VoucherDataOneOfAlt7
    alt8: VoucherDataOneOfAlt8
    alt9: VoucherDataOneOfAlt9
    alt10: VoucherDataOneOfAlt10
    alt11: VoucherDataOneOfAlt11
    alt12: VoucherDataOneOfAlt12
    alt13: VoucherDataOneOfAlt13
}

union Wallet {
    paypal: Paypal
    venmo: Venmo
}

/// Masked payout method details for wallet payout method
@untagged
union WalletAdditionalData {
    PaypalAdditionalData: PaypalAdditionalData
    VenmoAdditionalData: VenmoAdditionalData
}

union WalletData {
    ali_pay_qr: AliPayQr
    ali_pay_redirect: AliPayRedirection
    ali_pay_hk_redirect: AliPayHkRedirection
    amazon_pay_redirect: AmazonPayRedirectData
    momo_redirect: MomoRedirection
    kakao_pay_redirect: KakaoPayRedirection
    go_pay_redirect: GoPayRedirection
    gcash_redirect: GcashRedirection
    apple_pay: ApplePayWalletData
    apple_pay_redirect: ApplePayRedirectData
    apple_pay_third_party_sdk: ApplePayThirdPartySdkData
    dana_redirect: Document
    google_pay: GooglePayWalletData
    google_pay_redirect: GooglePayRedirectData
    google_pay_third_party_sdk: GooglePayThirdPartySdkData
    mb_way_redirect: MbWayRedirection
    mobile_pay_redirect: MobilePayRedirection
    paypal_redirect: PaypalRedirection
    paypal_sdk: PayPalWalletData
    paze: PazeWalletData
    samsung_pay: SamsungPayWalletData
    twint_redirect: Document
    vipps_redirect: Document
    touch_n_go_redirect: TouchNGoRedirection
    we_chat_pay_redirect: WeChatPayRedirection
    we_chat_pay_qr: WeChatPayQr
    cashapp_qr: CashappQr
    swish_qr: SwishQrData
    mifinity: MifinityData
    revolut_pay: RevolutPayData
}

/// Hyperswitch supports SDK integration with Apple Pay and Google Pay wallets. For other wallets, we integrate with their respective connectors, redirecting the customer to the connector for wallet payments. As a result, we don’t receive any payment method data in the confirm call for payments made through other wallets.
union WalletResponseData {
    apple_pay: WalletAdditionalDataForCard
    google_pay: WalletAdditionalDataForCard
    samsung_pay: WalletAdditionalDataForCard
}

/// Charge Information
union XenditChargeResponseData {
    multiple_splits: XenditMultipleSplitResponse
    single_split: XenditSplitSubMerchantData
}

/// Xendit Charge Request
union XenditSplitRequest {
    multiple_splits: XenditMultipleSplitRequest
    single_split: XenditSplitSubMerchantData
}

list AcceptedCountriesOneOfAlt0List {
    member: CountryAlpha2
}

list AcceptedCountriesOneOfAlt1List {
    member: CountryAlpha2
}

list AcceptedCurrenciesOneOfAlt0List {
    member: Currency
}

list AcceptedCurrenciesOneOfAlt1List {
    member: Currency
}

/// Acquirer configs
list AcquirerConfigs {
    member: ProfileAcquirerResponse
}

/// The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc)
list AllowedAuthMethods {
    member: String
}

/// List of supported card brands
list AllowedBrands {
    member: String
}

/// The list of allowed card networks (ex: AMEX,JCB etc)
list AllowedCardNetworks {
    member: String
}

/// List of the allowed payment meythods
list AllowedPaymentMethods {
    member: GpayAllowedPaymentMethods
}

list ApplePayBillingContactFields {
    member: ApplePayAddressParameters
}

list ApplePayShippingContactFields {
    member: ApplePayAddressParameters
}

list BankCodeResponseBankName {
    member: BankNames
}

list BankCodeResponseEligibleConnectors {
    member: String
}

list BankDebitTypesEligibleConnectors {
    member: String
}

/// The list of banks enabled, if applicable for a payment method type
list BankNames {
    member: BankCodeResponse
}

/// The list of eligible connectors for a given payment experience
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list BankTransferTypesEligibleConnectors {
    member: String
}

/// List of payment methods shown on collect UI
@dataExamples([
    {
        json: "[{\"payment_method\": \"bank_transfer\", \"payment_method_types\": [\"ach\", \"bacs\", \"sepa\"]}]"
    }
])
list BusinessCollectLinkConfigAllOf1EnabledPaymentMethods {
    member: EnabledPaymentMethod
}

/// A list of allowed domains (glob patterns) where this link can be embedded / opened from
@uniqueItems
list BusinessGenericLinkConfigAllOf1AllowedDomains {
    member: String
}

/// A list of allowed domains (glob patterns) where this link can be embedded / opened from
@uniqueItems
list BusinessPaymentLinkConfigAllOf1AllowedDomains {
    member: String
}

@dataExamples([
    {
        json: "[Visa, Mastercard]"
    }
])
list CardBrands {
    member: CardNetwork
}

/// The list of eligible connectors for a given card network
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list CardNetworkTypesEligibleConnectors {
    member: String
}

list Condition {
    member: Comparison
}

list ConnectorSelectionOneOfAlt0Data {
    member: RoutableConnectorChoice
}

list ConnectorSelectionOneOfAlt1Data {
    member: ConnectorVolumeSplit
}

list Constants {
    member: Double
}

/// Type of payment experience enabled with the connector
@dataExamples([
    {
        json: [
            "redirect_to_url"
        ]
    }
])
list CustomerPaymentMethodPaymentExperience {
    member: PaymentExperience
}

/// List of payment methods for customer
list CustomerPaymentMethods {
    member: CustomerPaymentMethod
}

list DefaultGatewayExtraScore {
    member: DecisionEngineGatewayWiseExtraScore
}

list EliminationRoutingConfigParams {
    member: DynamicRoutingConfigParams
}

/// An array of associated payment method types
@uniqueItems
list EnabledPaymentMethodPaymentMethodTypes {
    member: PaymentMethodType
}

/// Filter events by their class.
@uniqueItems
list EventClasses {
    member: EventClass
}

/// The list of events
list Events {
    member: EventListItemResponse
}

/// Filter events by their type.
@uniqueItems
list EventTypes {
    member: EventType
}

list FeatureMatrixListResponseConnectors {
    member: ConnectorFeatureMatrixResponse
}

list FeatureMatrixRequestConnectors {
    member: Connector
}

list FieldTypeOneOfAlt10UserCurrencyOptions {
    member: String
}

list FieldTypeOneOfAlt18UserAddressCountryOptions {
    member: String
}

list FieldTypeOneOfAlt25UserShippingAddressCountryOptions {
    member: String
}

list FieldTypeOneOfAlt33DropDownOptions {
    member: String
}

list FieldTypeOneOfAlt36LanguagePreferenceOptions {
    member: String
}

list FieldTypeOneOfAlt9UserCountryOptions {
    member: String
}

/// payment methods that can be used in the payment
list FrmConfigsPaymentMethods {
    member: FrmPaymentMethod
}

/// payment method types(credit, debit) that can be used in the payment. This field is deprecated. It has not been removed to provide backward compatibility.
list FrmPaymentMethodPaymentMethodTypes {
    member: FrmPaymentMethodType
}

list GatewayExtraScore {
    member: DecisionEngineGatewayWiseExtraScore
}

list LabelInfo {
    member: LabelInformation
}

list LinkedRoutingConfigRetrieveResponseOneOfAlt1 {
    member: RoutingDictionaryRecord
}

list ListAllAPIKeysAssociatedWithAMerchantAccount200Body {
    member: RetrieveApiKeyResponse
}

list ListAllCustomersForAMerchant200Body {
    member: CustomerResponse
}

list ListAllDeliveryAttemptsForAnEvent200Body {
    member: EventRetrieveResponse
}

list ListAllMerchantConnectors200Body {
    member: MerchantConnectorListResponse
}

list ListAllPaymentMethodsForACustomerInputAcceptedCountries {
    member: CountryAlpha2
}

list ListAllPaymentMethodsForACustomerInputAcceptedCurrencies {
    member: Currency
}

list ListAllPaymentMethodsForACustomerInputCardNetworks {
    member: CardNetwork
}

list ListAllPaymentMethodsForAMerchantInputAcceptedCountries {
    member: CountryAlpha2
}

list ListAllPaymentMethodsForAMerchantInputAcceptedCurrencies {
    member: Currency
}

list ListAllPaymentMethodsForAMerchantInputCardNetworks {
    member: CardNetwork
}

list ListAllPayments200Body {
    member: PaymentListResponse
}

list ListCustomerPaymentMethodsViaClientSecretInputAcceptedCountries {
    member: CountryAlpha2
}

list ListCustomerPaymentMethodsViaClientSecretInputAcceptedCurrencies {
    member: Currency
}

list ListCustomerPaymentMethodsViaClientSecretInputCardNetworks {
    member: CardNetwork
}

list ListDisputes200Body {
    member: DisputeResponse
}

list ListMandatesForACustomer200Body {
    member: MandateResponse
}

list ListProfiles200Body {
    member: ProfileResponse
}

/// Details about the primary business unit of the merchant account
list MerchantAccountResponsePrimaryBusinessDetails {
    member: PrimaryBusinessDetails
}

/// Details about the primary business unit of the merchant account
list MerchantAccountUpdatePrimaryBusinessDetails {
    member: PrimaryBusinessDetails
}

/// The list of merchant capabilities(ex: whether capable of 3ds or no-3ds)
list MerchantCapabilities {
    member: String
}

/// Contains the frm configs for the merchant connector
@dataExamples([
    {
        json: "\n[{\"gateway\":\"stripe\",\"payment_methods\":[{\"payment_method\":\"card\",\"payment_method_types\":[{\"payment_method_type\":\"credit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\",\"action\":\"cancel_txn\"},{\"payment_method_type\":\"debit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\"}]}]}]\n"
    }
])
list MerchantConnectorCreateFrmConfigs {
    member: FrmConfigs
}

/// An object containing the details about the payment methods that need to be enabled under this merchant connector account
@dataExamples([
    {
        json: [
            {
                recurring_enabled: true
                maximum_amount: 68607706
                accepted_currencies: {
                    list: [
                        "USD"
                        "EUR"
                    ]
                    type: "enable_only"
                }
                installment_payment_enabled: true
                accepted_countries: {
                    type: "disable_only"
                    list: [
                        "FR"
                        "DE"
                        "IN"
                    ]
                }
                payment_schemes: [
                    "Discover"
                    "Discover"
                ]
                payment_method_types: [
                    "upi_collect"
                    "upi_intent"
                ]
                minimum_amount: 1
                payment_method_issuers: [
                    "labore magna ipsum"
                    "aute"
                ]
                payment_method: "wallet"
            }
        ]
    }
])
list MerchantConnectorCreatePaymentMethodsEnabled {
    member: PaymentMethodsEnabled
}

/// The list of merchant connector ids to filter the refunds list for selected label
list MerchantConnectorId {
    member: String
}

/// identifier for the verified domains of a particular connector account
list MerchantConnectorListResponseApplepayVerifiedDomains {
    member: String
}

/// Contains the frm configs for the merchant connector
@dataExamples([
    {
        json: "\n[{\"gateway\":\"stripe\",\"payment_methods\":[{\"payment_method\":\"card\",\"payment_method_types\":[{\"payment_method_type\":\"credit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\",\"action\":\"cancel_txn\"},{\"payment_method_type\":\"debit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\"}]}]}]\n"
    }
])
list MerchantConnectorListResponseFrmConfigs {
    member: FrmConfigs
}

/// An object containing the details about the payment methods that need to be enabled under this merchant connector account
@dataExamples([
    {
        json: [
            {
                recurring_enabled: true
                maximum_amount: 68607706
                accepted_currencies: {
                    list: [
                        "USD"
                        "EUR"
                    ]
                    type: "enable_only"
                }
                installment_payment_enabled: true
                accepted_countries: {
                    type: "disable_only"
                    list: [
                        "FR"
                        "DE"
                        "IN"
                    ]
                }
                payment_schemes: [
                    "Discover"
                    "Discover"
                ]
                payment_method_types: [
                    "upi_collect"
                    "upi_intent"
                ]
                minimum_amount: 1
                payment_method_issuers: [
                    "labore magna ipsum"
                    "aute"
                ]
                payment_method: "wallet"
            }
        ]
    }
])
list MerchantConnectorListResponsePaymentMethodsEnabled {
    member: PaymentMethodsEnabled
}

/// identifier for the verified domains of a particular connector account
list MerchantConnectorResponseApplepayVerifiedDomains {
    member: String
}

/// Contains the frm configs for the merchant connector
@dataExamples([
    {
        json: "\n[{\"gateway\":\"stripe\",\"payment_methods\":[{\"payment_method\":\"card\",\"payment_method_types\":[{\"payment_method_type\":\"credit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\",\"action\":\"cancel_txn\"},{\"payment_method_type\":\"debit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\"}]}]}]\n"
    }
])
list MerchantConnectorResponseFrmConfigs {
    member: FrmConfigs
}

/// An object containing the details about the payment methods that need to be enabled under this merchant connector account
@dataExamples([
    {
        json: [
            {
                recurring_enabled: true
                maximum_amount: 68607706
                accepted_currencies: {
                    list: [
                        "USD"
                        "EUR"
                    ]
                    type: "enable_only"
                }
                installment_payment_enabled: true
                accepted_countries: {
                    type: "disable_only"
                    list: [
                        "FR"
                        "DE"
                        "IN"
                    ]
                }
                payment_schemes: [
                    "Discover"
                    "Discover"
                ]
                payment_method_types: [
                    "upi_collect"
                    "upi_intent"
                ]
                minimum_amount: 1
                payment_method_issuers: [
                    "labore magna ipsum"
                    "aute"
                ]
                payment_method: "wallet"
            }
        ]
    }
])
list MerchantConnectorResponsePaymentMethodsEnabled {
    member: PaymentMethodsEnabled
}

/// Contains the frm configs for the merchant connector
@dataExamples([
    {
        json: "\n[{\"gateway\":\"stripe\",\"payment_methods\":[{\"payment_method\":\"card\",\"payment_method_types\":[{\"payment_method_type\":\"credit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\",\"action\":\"cancel_txn\"},{\"payment_method_type\":\"debit\",\"card_networks\":[\"Visa\"],\"flow\":\"pre\"}]}]}]\n"
    }
])
list MerchantConnectorUpdateFrmConfigs {
    member: FrmConfigs
}

/// An object containing the details about the payment methods that need to be enabled under this merchant connector account
@dataExamples([
    {
        json: [
            {
                recurring_enabled: true
                maximum_amount: 68607706
                accepted_currencies: {
                    list: [
                        "USD"
                        "EUR"
                    ]
                    type: "enable_only"
                }
                installment_payment_enabled: true
                accepted_countries: {
                    type: "disable_only"
                    list: [
                        "FR"
                        "DE"
                        "IN"
                    ]
                }
                payment_schemes: [
                    "Discover"
                    "Discover"
                ]
                payment_method_types: [
                    "upi_collect"
                    "upi_intent"
                ]
                minimum_amount: 1
                payment_method_issuers: [
                    "labore magna ipsum"
                    "aute"
                ]
                payment_method: "wallet"
            }
        ]
    }
])
list MerchantConnectorUpdatePaymentMethodsEnabled {
    member: PaymentMethodsEnabled
}

list Nested {
    member: IfStatement
}

/// The request headers sent in the webhook.
@dataExamples([
    {
        json: [
            [
                "content-type"
                "application/json"
            ]
            [
                "content-length"
                "1024"
            ]
        ]
    }
])
list OutgoingWebhookRequestContentHeaders {
    member: OutgoingWebhookRequestContentHeadersItem
}

list OutgoingWebhookRequestContentHeadersItem {
    member: OutgoingWebhookRequestContentHeadersItemItem
}

/// The response headers received for the webhook sent.
@dataExamples([
    {
        json: [
            [
                "content-type"
                "application/json"
            ]
            [
                "content-length"
                "1024"
            ]
        ]
    }
])
list OutgoingWebhookResponseContentHeaders {
    member: OutgoingWebhookResponseContentHeadersItem
}

list OutgoingWebhookResponseContentHeadersItem {
    member: OutgoingWebhookResponseContentHeadersItemItem
}

/// The list of eligible connectors for a given payment experience
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list PaymentExperienceTypesEligibleConnectors {
    member: String
}

/// A list of allowed domains (glob patterns) where this link can be embedded / opened from
@uniqueItems
list PaymentLinkConfigAllowedDomains {
    member: String
}

/// Dynamic details related to merchant to be rendered in payment link
list PaymentLinkConfigRequestTransactionDetails {
    member: PaymentLinkTransactionDetails
}

/// Dynamic details related to merchant to be rendered in payment link
list PaymentLinkConfigTransactionDetails {
    member: PaymentLinkTransactionDetails
}

list PaymentListResponseData {
    member: PaymentsResponse
}

/// List of payment methods shown on collect UI
@dataExamples([
    {
        json: "[{\"payment_method\": \"bank_transfer\", \"payment_method_types\": [\"ach\", \"bacs\"]}]"
    }
])
list PaymentMethodCollectLinkRequestAllOf1EnabledPaymentMethods {
    member: EnabledPaymentMethod
}

/// List of payment methods shown on collect UI
@dataExamples([
    {
        json: "[{\"payment_method\": \"bank_transfer\", \"payment_method_types\": [\"ach\", \"bacs\"]}]"
    }
])
list PaymentMethodCollectLinkResponseAllOf1EnabledPaymentMethods {
    member: EnabledPaymentMethod
}

/// Information about the payment method
list PaymentMethodListResponsePaymentMethods {
    member: ResponsePaymentMethodsEnabled
}

/// Type of payment experience enabled with the connector
@dataExamples([
    {
        json: [
            "redirect_to_url"
        ]
    }
])
list PaymentMethodResponsePaymentExperience {
    member: PaymentExperience
}

/// Subtype of payment method
@dataExamples([
    {
        json: [
            "credit"
        ]
    }
])
list PaymentMethodsEnabledPaymentMethodTypes {
    member: RequestPaymentMethodTypes
}

/// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
list PaymentsConfirmRequestAllowedPaymentMethodTypes {
    member: PaymentMethodType
}

/// This allows to manually select a connector with which the payment can go through.
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list PaymentsConfirmRequestConnector {
    member: Connector
}

/// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
@dataExamples([
    {
        json: "[{\n        \"product_name\": \"Apple iPhone 16\",\n        \"quantity\": 1,\n        \"amount\" : 69000\n        \"product_img_link\" : \"https://dummy-img-link.com\"\n    }]"
    }
])
list PaymentsConfirmRequestOrderDetails {
    member: OrderDetailsWithAmount
}

/// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
list PaymentsCreateRequestAllowedPaymentMethodTypes {
    member: PaymentMethodType
}

/// This allows to manually select a connector with which the payment can go through.
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list PaymentsCreateRequestConnector {
    member: Connector
}

/// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
@dataExamples([
    {
        json: "[{\n        \"product_name\": \"Apple iPhone 16\",\n        \"quantity\": 1,\n        \"amount\" : 69000\n        \"product_img_link\" : \"https://dummy-img-link.com\"\n    }]"
    }
])
list PaymentsCreateRequestOrderDetails {
    member: OrderDetailsWithAmount
}

/// Allowed Payment Method Types for a given PaymentIntent
list PaymentsCreateResponseOpenApiAllowedPaymentMethodTypes {
    member: PaymentMethodType
}

/// List of attempts that happened on this intent
list PaymentsCreateResponseOpenApiAttempts {
    member: PaymentAttemptResponse
}

/// List of captures done on latest attempt
list PaymentsCreateResponseOpenApiCaptures {
    member: CaptureResponse
}

/// List of disputes that happened on this intent
list PaymentsCreateResponseOpenApiDisputes {
    member: DisputeResponsePaymentsRetrieve
}

/// List of incremental authorizations happened to the payment
list PaymentsCreateResponseOpenApiIncrementalAuthorizations {
    member: IncrementalAuthorizationResponse
}

/// Information about the product , quantity and amount for connectors. (e.g. Klarna)
@dataExamples([
    {
        json: "[{\n        \"product_name\": \"gillete creme\",\n        \"quantity\": 15,\n        \"amount\" : 900\n    }]"
    }
])
list PaymentsCreateResponseOpenApiOrderDetails {
    member: OrderDetailsWithAmount
}

/// An array of refund objects associated with this payment. Empty or null if no refunds have been processed.
list PaymentsCreateResponseOpenApiRefunds {
    member: RefundResponse
}

/// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
list PaymentsRequestAllowedPaymentMethodTypes {
    member: PaymentMethodType
}

/// This allows to manually select a connector with which the payment can go through.
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list PaymentsRequestConnector {
    member: Connector
}

/// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
@dataExamples([
    {
        json: "[{\n        \"product_name\": \"Apple iPhone 16\",\n        \"quantity\": 1,\n        \"amount\" : 69000\n        \"product_img_link\" : \"https://dummy-img-link.com\"\n    }]"
    }
])
list PaymentsRequestOrderDetails {
    member: OrderDetailsWithAmount
}

/// Allowed Payment Method Types for a given PaymentIntent
list PaymentsResponseAllowedPaymentMethodTypes {
    member: PaymentMethodType
}

/// List of attempts that happened on this intent
list PaymentsResponseAttempts {
    member: PaymentAttemptResponse
}

/// List of captures done on latest attempt
list PaymentsResponseCaptures {
    member: CaptureResponse
}

/// List of disputes that happened on this intent
list PaymentsResponseDisputes {
    member: DisputeResponsePaymentsRetrieve
}

/// List of incremental authorizations happened to the payment
list PaymentsResponseIncrementalAuthorizations {
    member: IncrementalAuthorizationResponse
}

/// Information about the product , quantity and amount for connectors. (e.g. Klarna)
@dataExamples([
    {
        json: "[{\n        \"product_name\": \"gillete creme\",\n        \"quantity\": 15,\n        \"amount\" : 900\n    }]"
    }
])
list PaymentsResponseOrderDetails {
    member: OrderDetailsWithAmount
}

/// An array of refund objects associated with this payment. Empty or null if no refunds have been processed.
list PaymentsResponseRefunds {
    member: RefundResponse
}

/// The list of session token object
list PaymentsSessionResponseSessionToken {
    member: SessionToken
}

/// List of payment statuses that triggers a webhook for payment intents
@dataExamples([
    {
        json: [
            "succeeded"
            "failed"
            "partially_captured"
            "requires_merchant_action"
        ]
    }
])
list PaymentStatusesEnabled {
    member: IntentStatus
}

/// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
list PaymentsUpdateRequestAllowedPaymentMethodTypes {
    member: PaymentMethodType
}

/// This allows to manually select a connector with which the payment can go through.
@dataExamples([
    {
        json: [
            "stripe"
            "adyen"
        ]
    }
])
list PaymentsUpdateRequestConnector {
    member: Connector
}

/// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
@dataExamples([
    {
        json: "[{\n        \"product_name\": \"Apple iPhone 16\",\n        \"quantity\": 1,\n        \"amount\" : 69000\n        \"product_img_link\" : \"https://dummy-img-link.com\"\n    }]"
    }
])
list PaymentsUpdateRequestOrderDetails {
    member: OrderDetailsWithAmount
}

/// This field allows the merchant to manually select a connector with which the payout can go through.
@dataExamples([
    {
        json: [
            "wise"
            "adyen"
        ]
    }
])
list PayoutConfirmRequestConnector {
    member: PayoutConnectors
}

/// List of payout methods shown on collect UI
@dataExamples([
    {
        json: "[{\"payment_method\": \"bank_transfer\", \"payment_method_types\": [\"ach\", \"bacs\"]}]"
    }
])
list PayoutCreatePayoutLinkConfigAllOf1EnabledPaymentMethods {
    member: EnabledPaymentMethod
}

/// List of attempts
list PayoutCreateResponseAttempts {
    member: PayoutAttemptResponse
}

/// The list of connectors to filter payouts list
@dataExamples([
    {
        json: [
            "wise"
            "adyen"
        ]
    }
])
list PayoutListFilterConstraintsAllOf1Connector {
    member: PayoutConnectors
}

/// The list of payout methods to filter payouts list
@dataExamples([
    {
        json: [
            "bank"
            "card"
        ]
    }
])
list PayoutListFilterConstraintsAllOf1PayoutMethod {
    member: PayoutType
}

/// The list of payout status to filter payouts list
@dataExamples([
    {
        json: [
            "pending"
            "failed"
        ]
    }
])
list PayoutListFilterConstraintsAllOf1Status {
    member: PayoutStatus
}

/// The list of available connector filters
list PayoutListFiltersConnector {
    member: PayoutConnectors
}

/// The list of available currency filters
list PayoutListFiltersCurrency {
    member: Currency
}

/// The list of available payout method filters
list PayoutListFiltersPayoutMethod {
    member: PayoutType
}

/// The list of available payout status filters
list PayoutListFiltersStatus {
    member: PayoutStatus
}

/// The list of payouts response objects
list PayoutListResponseData {
    member: PayoutCreateResponse
}

/// This field allows the merchant to manually select a connector with which the payout can go through.
@dataExamples([
    {
        json: [
            "wise"
            "adyen"
        ]
    }
])
list PayoutsCreateRequestConnector {
    member: PayoutConnectors
}

/// List of payout statuses that triggers a webhook for payouts
@dataExamples([
    {
        json: [
            "success"
            "failed"
        ]
    }
])
list PayoutStatusesEnabled {
    member: PayoutStatus
}

/// This field allows the merchant to manually select a connector with which the payout can go through.
@dataExamples([
    {
        json: [
            "wise"
            "adyen"
        ]
    }
])
list PayoutUpdateRequestConnector {
    member: PayoutConnectors
}

/// Verified Apple Pay domains for a particular profile
list ProfileCreateApplepayVerifiedDomains {
    member: String
}

list ProfileDefaultRoutingConfigConnectors {
    member: RoutableConnectorChoice
}

/// Verified Apple Pay domains for a particular profile
list ProfileResponseApplepayVerifiedDomains {
    member: String
}

list Records {
    member: RoutingDictionaryRecord
}

/// The list of connectors to filter refunds list
list RefundListRequestAllOf1Connector {
    member: String
}

/// The list of currencies to filter refunds list
list RefundListRequestAllOf1Currency {
    member: Currency
}

/// The List of refund response object
list RefundListResponseData {
    member: RefundResponse
}

/// List of refund statuses that triggers a webhook for refunds
@dataExamples([
    {
        json: [
            "success"
            "failure"
        ]
    }
])
list RefundStatusesEnabled {
    member: IntentStatus
}

list RequestPaymentMethodTypesCardNetworks {
    member: CardNetwork
}

/// The list of payment method types enabled for a connector account
list ResponsePaymentMethodsEnabledPaymentMethodTypes {
    member: ResponsePaymentMethodTypes
}

/// The list of card networks enabled, if applicable for a payment method type
list ResponsePaymentMethodTypesCardNetworks {
    member: CardNetworkTypes
}

/// The list of payment experiences enabled, if applicable for a payment method type
list ResponsePaymentMethodTypesPaymentExperience {
    member: PaymentExperienceTypes
}

list RetrieveDefaultFallbackConfig200Body {
    member: RoutableConnectorChoice
}

list RoutingKindOneOfAlt1 {
    member: RoutingDictionaryRecord
}

list RuleConnectorSelectionStatements {
    member: IfStatement
}

list RuleThreeDsDecisionRuleStatements {
    member: IfStatement
}

/// Additional tags to be used for global search
list SearchTags {
    member: String
}

/// Data for the split items
list SplitItems {
    member: AdyenSplitItem
}

list StaticRoutingAlgorithmOneOfAlt1Data {
    member: RoutableConnectorChoice
}

list StaticRoutingAlgorithmOneOfAlt2Data {
    member: ConnectorVolumeSplit
}

list StraightThroughAlgorithmOneOfAlt1Data {
    member: RoutableConnectorChoice
}

list StraightThroughAlgorithmOneOfAlt2Data {
    member: ConnectorVolumeSplit
}

list SubLevelInputConfig {
    member: DecisionEngineSRSubLevelInputConfig
}

list SuccessBasedRoutingConfigParams {
    member: DynamicRoutingConfigParams
}

/// List of supported capture methods supported by the payment method type
list SupportedCaptureMethods {
    member: CaptureMethod
}

/// List of supported card networks
list SupportedCardNetworks {
    member: CardNetwork
}

/// List of countries supported by the payment method type via the connector
@uniqueItems
list SupportedCountries {
    member: CountryAlpha3
}

/// List of currencies supported by the payment method type via the connector
@uniqueItems
list SupportedCurrencies {
    member: Currency
}

/// The list of supported networks
list SupportedNetworks {
    member: String
}

/// The list of payment methods supported by the connector
list SupportedPaymentMethods {
    member: SupportedPaymentMethod
}

/// The list of webhook flows supported by the connector
list SupportedWebhookFlows {
    member: EventClass
}

list UpdateDefaultConfigsForAllProfilesInputBody {
    member: RoutableConnectorChoice
}

list UpdateDefaultFallbackConfig200Body {
    member: RoutableConnectorChoice
}

list UpdateDefaultFallbackConfigInputBody {
    member: RoutableConnectorChoice
}

/// Represents an array of numbers. This is basically used for
/// "one of the given numbers" operations
/// eg: payment.method.amount = (1, 2, 3)
list ValueTypeOneOfAlt4Value {
    member: MinorUnit
}

/// Similar to NumberArray but for enum variants
/// eg: payment.method.cardtype = (debit, credit)
list ValueTypeOneOfAlt5Value {
    member: String
}

/// Like a number array but can include comparisons. Useful for
/// conditions like "500 < amount < 1000"
/// eg: payment.amount = (> 500, < 1000)
list ValueTypeOneOfAlt6Value {
    member: NumberComparison
}

/// The list of the supported wallets
list Wallets {
    member: PaymentMethodType
}

/// Array of objects that define how the platform wants to route the fees and to which accounts.
list XenditMultipleSplitRequestRoutes {
    member: XenditSplitRoute
}

/// Array of objects that define how the platform wants to route the fees and to which accounts.
list XenditMultipleSplitResponseRoutes {
    member: XenditSplitRoute
}

/// Acquirer configs
map AcquirerConfigMap {
    key: String
    value: AcquirerConfig
}

/// list of configs for multi theme setup
map BusinessSpecificConfigs {
    key: String
    value: PaymentLinkConfigRequest
}

/// Additional metadata that the Static Analyzer and Backend does not touch.
/// This can be used to store useful information for the frontend and is required for communication
/// between the static analyzer and the frontend.
map ComparisonMetadata {
    key: String
    value: ComparisonMetadataItem
}

/// Payment link configuration rules
map PaymentLinkConfigPaymentLinkUiRules {
    key: String
    value: PaymentLinkConfigPaymentLinkUiRulesItem
}

map PaymentLinkConfigPaymentLinkUiRulesItem {
    key: String
    value: String
}

/// Payment link configuration rules
map PaymentLinkConfigRequestPaymentLinkUiRules {
    key: String
    value: PaymentLinkConfigRequestPaymentLinkUiRulesItem
}

map PaymentLinkConfigRequestPaymentLinkUiRulesItem {
    key: String
    value: String
}

/// SDK configuration rules
map PaymentLinkConfigRequestSdkUiRules {
    key: String
    value: PaymentLinkConfigRequestSdkUiRulesItem
}

map PaymentLinkConfigRequestSdkUiRulesItem {
    key: String
    value: String
}

/// SDK configuration rules
map PaymentLinkConfigSdkUiRules {
    key: String
    value: PaymentLinkConfigSdkUiRulesItem
}

map PaymentLinkConfigSdkUiRulesItem {
    key: String
    value: String
}

map ProgramConnectorSelectionMetadata {
    key: String
    value: ProgramConnectorSelectionMetadataItem
}

map ProgramThreeDsDecisionRuleMetadata {
    key: String
    value: ProgramThreeDsDecisionRuleMetadataItem
}

/// Required fields for the payment_method_type.
map RequiredFields {
    key: String
    value: RequiredFieldInfo
}

/// Public key component of the ephemeral key pair generated by the 3DS SDK
map SdkEphemPubKey {
    key: String
    value: String
}

/// This is used to indicate if the mandate was accepted online or offline
enum AcceptanceType {
    online
    offline
}

enum AcceptedCountriesOneOfAlt0Type {
    enable_only
}

enum AcceptedCountriesOneOfAlt1Type {
    disable_only
}

enum AcceptedCountriesOneOfAlt2Type {
    all_accepted
}

enum AcceptedCurrenciesOneOfAlt0Type {
    enable_only
}

enum AcceptedCurrenciesOneOfAlt1Type {
    disable_only
}

enum AcceptedCurrenciesOneOfAlt2Type {
    all_accepted
}

enum AdyenSplitType {
    BalanceAccount
    AcquiringFees
    PaymentFee
    AdyenFees
    AdyenCommission
    AdyenMarkup
    Interchange
    SchemeFee
    Commission
    TopUp
    Vat
}

document AliPayHkRedirection

document AliPayQr

document AliPayRedirection

document AmazonPayRedirectData

enum ApiKeyExpirationOneOfAlt0 {
    never
}

@timestampFormat("date-time")
timestamp ApiKeyExpirationOneOfAlt1

enum ApplePayAddressParameters {
    postalAddress
    phone
    email
}

enum ApplepayInitiative {
    web
    ios
}

enum ApplePayPaymentTiming {
    immediate
    recurring
}

document ApplePayRedirectData

document ApplePayThirdPartySdkData

/// The status of the attempt
enum AttemptStatus {
    started
    authentication_failed
    router_declined
    authentication_pending
    authentication_successful
    authorized
    authorization_failed
    charged
    authorizing
    cod_initiated
    voided
    void_initiated
    capture_initiated
    capture_failed
    void_failed
    auto_refunded
    partial_charged
    partial_charged_and_chargeable
    unresolved
    pending
    failure
    payment_method_awaited
    confirmation_awaited
    device_data_collection_pending
    integrity_failure
}

enum AuthenticationConnectors {
    threedsecureio
    netcetera
    gpayments
    ctp_mastercard
    unified_authentication_service
    juspaythreedsserver
    ctp_visa
}

enum AuthenticationStatus {
    started
    pending
    success
    failed
}

/// Specifies the type of cardholder authentication to be applied for a payment.
/// 
/// - `ThreeDs`: Requests 3D Secure (3DS) authentication. If the card is enrolled, 3DS authentication will be activated, potentially shifting chargeback liability to the issuer.
/// - `NoThreeDs`: Indicates that 3D Secure authentication should not be performed. The liability for chargebacks typically remains with the merchant. This is often the default if not specified.
/// 
/// Note: The actual authentication behavior can also be influenced by merchant configuration and specific connector defaults. Some connectors might still enforce 3DS or bypass it regardless of this parameter.
enum AuthenticationType {
    three_ds
    no_three_ds
}

enum AuthorizationStatus {
    success
    failure
    processing
    unresolved
}

enum BankHolderType {
    personal
    business
}

enum BankType {
    checking
    savings
}

enum BlocklistDataKind {
    payment_method
    card_bin
    extended_card_bin
}

enum BlocklistRequestOneOfAlt0Type {
    card_bin
}

enum BlocklistRequestOneOfAlt1Type {
    fingerprint
}

enum BlocklistRequestOneOfAlt2Type {
    extended_card_bin
}

/// Specifies how the payment is captured.
/// - `automatic`: Funds are captured immediately after successful authorization. This is the default behavior if the field is omitted.
/// - `manual`: Funds are authorized but not captured. A separate request to the `/payments/{payment_id}/capture` endpoint is required to capture the funds.
enum CaptureMethod {
    automatic
    manual
    manual_multiple
    scheduled
    sequential_automatic
}

enum CaptureStatus {
    started
    charged
    pending
    failed
}

/// Indicates the method by which a card is discovered during a payment
enum CardDiscovery {
    manual
    saved_card
    click_to_pay
}

/// Indicates the card network.
enum CardNetwork {
    Visa
    Mastercard
    AmericanExpress
    JCB
    DinersClub
    Discover
    CartesBancaires
    UnionPay
    Interac
    RuPay
    Maestro
    Star
    Pulse
    Accel
    Nyce
}

enum CardTestingGuardStatus {
    enabled
    disabled
}

document CashappQr

/// Conditional comparison type
enum ComparisonType {
    equal
    not_equal
    less_than
    less_than_equal
    greater_than
    greater_than_equal
}

enum Connector {
    authipay
    adyenplatform
    stripe_billing_test
    phonypay
    fauxpay
    pretendpay
    stripe_test
    adyen_test
    checkout_test
    paypal_test
    aci
    adyen
    airwallex
    archipel
    authorizedotnet
    bambora
    bamboraapac
    bankofamerica
    barclaycard
    billwerk
    bitpay
    bluesnap
    boku
    braintree
    cashtocode
    celero
    chargebee
    checkout
    coinbase
    coingate
    cryptopay
    ctp_mastercard
    ctp_visa
    cybersource
    datatrans
    deutschebank
    digitalvirgo
    dlocal
    ebanx
    elavon
    facilitapay
    fiserv
    fiservemea
    fiuu
    forte
    getnet
    globalpay
    globepay
    gocardless
    gpayments
    hipay
    helcim
    hyperswitch_vault
    inespay
    iatapay
    itaubank
    jpmorgan
    juspaythreedsserver
    klarna
    mifinity
    mollie
    moneris
    multisafepay
    netcetera
    nexinets
    nexixpay
    nmi
    nomupay
    noon
    novalnet
    nuvei
    opennode
    paybox
    payload
    payme
    payone
    paypal
    paystack
    payu
    placetopay
    powertranz
    prophetpay
    rapyd
    razorpay
    recurly
    redsys
    santander
    shift4
    square
    stax
    stripe
    stripebilling
    taxjar
    threedsecureio
    tokenio
    trustpay
    tsys
    vgs
    volt
    wellsfargo
    wise
    worldline
    worldpay
    worldpayvantiv
    worldpayxml
    signifyd
    plaid
    riskified
    xendit
    zen
    zsl
}

enum ConnectorSelectionOneOfAlt0Type {
    priority
}

enum ConnectorSelectionOneOfAlt1Type {
    volume_split
}

enum ConnectorStatus {
    inactive
    active
}

/// Type of the Connector for the financial use case. Could range from Payments to Accounting to Banking.
enum ConnectorType {
    payment_processor
    payment_vas
    fin_operations
    fiz_operations
    networks
    banking_entities
    non_banking_finance
    payout_processor
    payment_method_auth
    authentication_processor
    tax_processor
    billing_processor
    vault_processor
}

enum ContractBasedTimeScale {
    day
    month
}

enum Country {
    Afghanistan
    AlandIslands
    Albania
    Algeria
    AmericanSamoa
    Andorra
    Angola
    Anguilla
    Antarctica
    AntiguaAndBarbuda
    Argentina
    Armenia
    Aruba
    Australia
    Austria
    Azerbaijan
    Bahamas
    Bahrain
    Bangladesh
    Barbados
    Belarus
    Belgium
    Belize
    Benin
    Bermuda
    Bhutan
    BoliviaPlurinationalState
    BonaireSintEustatiusAndSaba
    BosniaAndHerzegovina
    Botswana
    BouvetIsland
    Brazil
    BritishIndianOceanTerritory
    BruneiDarussalam
    Bulgaria
    BurkinaFaso
    Burundi
    CaboVerde
    Cambodia
    Cameroon
    Canada
    CaymanIslands
    CentralAfricanRepublic
    Chad
    Chile
    China
    ChristmasIsland
    CocosKeelingIslands
    Colombia
    Comoros
    Congo
    CongoDemocraticRepublic
    CookIslands
    CostaRica
    CotedIvoire
    Croatia
    Cuba
    Curacao
    Cyprus
    Czechia
    Denmark
    Djibouti
    Dominica
    DominicanRepublic
    Ecuador
    Egypt
    ElSalvador
    EquatorialGuinea
    Eritrea
    Estonia
    Ethiopia
    FalklandIslandsMalvinas
    FaroeIslands
    Fiji
    Finland
    France
    FrenchGuiana
    FrenchPolynesia
    FrenchSouthernTerritories
    Gabon
    Gambia
    Georgia
    Germany
    Ghana
    Gibraltar
    Greece
    Greenland
    Grenada
    Guadeloupe
    Guam
    Guatemala
    Guernsey
    Guinea
    GuineaBissau
    Guyana
    Haiti
    HeardIslandAndMcDonaldIslands
    HolySee
    Honduras
    HongKong
    Hungary
    Iceland
    India
    Indonesia
    IranIslamicRepublic
    Iraq
    Ireland
    IsleOfMan
    Israel
    Italy
    Jamaica
    Japan
    Jersey
    Jordan
    Kazakhstan
    Kenya
    Kiribati
    KoreaDemocraticPeoplesRepublic
    KoreaRepublic
    Kuwait
    Kyrgyzstan
    LaoPeoplesDemocraticRepublic
    Latvia
    Lebanon
    Lesotho
    Liberia
    Libya
    Liechtenstein
    Lithuania
    Luxembourg
    Macao
    MacedoniaTheFormerYugoslavRepublic
    Madagascar
    Malawi
    Malaysia
    Maldives
    Mali
    Malta
    MarshallIslands
    Martinique
    Mauritania
    Mauritius
    Mayotte
    Mexico
    MicronesiaFederatedStates
    MoldovaRepublic
    Monaco
    Mongolia
    Montenegro
    Montserrat
    Morocco
    Mozambique
    Myanmar
    Namibia
    Nauru
    Nepal
    Netherlands
    NewCaledonia
    NewZealand
    Nicaragua
    Niger
    Nigeria
    Niue
    NorfolkIsland
    NorthernMarianaIslands
    Norway
    Oman
    Pakistan
    Palau
    PalestineState
    Panama
    PapuaNewGuinea
    Paraguay
    Peru
    Philippines
    Pitcairn
    Poland
    Portugal
    PuertoRico
    Qatar
    Reunion
    Romania
    RussianFederation
    Rwanda
    SaintBarthelemy
    SaintHelenaAscensionAndTristandaCunha
    SaintKittsAndNevis
    SaintLucia
    SaintMartinFrenchpart
    SaintPierreAndMiquelon
    SaintVincentAndTheGrenadines
    Samoa
    SanMarino
    SaoTomeAndPrincipe
    SaudiArabia
    Senegal
    Serbia
    Seychelles
    SierraLeone
    Singapore
    SintMaartenDutchpart
    Slovakia
    Slovenia
    SolomonIslands
    Somalia
    SouthAfrica
    SouthGeorgiaAndTheSouthSandwichIslands
    SouthSudan
    Spain
    SriLanka
    Sudan
    Suriname
    SvalbardAndJanMayen
    Swaziland
    Sweden
    Switzerland
    SyrianArabRepublic
    TaiwanProvinceOfChina
    Tajikistan
    TanzaniaUnitedRepublic
    Thailand
    TimorLeste
    Togo
    Tokelau
    Tonga
    TrinidadAndTobago
    Tunisia
    Turkey
    Turkmenistan
    TurksAndCaicosIslands
    Tuvalu
    Uganda
    Ukraine
    UnitedArabEmirates
    UnitedKingdomOfGreatBritainAndNorthernIreland
    UnitedStatesOfAmerica
    UnitedStatesMinorOutlyingIslands
    Uruguay
    Uzbekistan
    Vanuatu
    VenezuelaBolivarianRepublic
    Vietnam
    VirginIslandsBritish
    VirginIslandsUS
    WallisAndFutuna
    WesternSahara
    Yemen
    Zambia
    Zimbabwe
}

enum CountryAlpha2 {
    AF
    AX
    AL
    DZ
    AS
    AD
    AO
    AI
    AQ
    AG
    AR
    AM
    AW
    AU
    AT
    AZ
    BS
    BH
    BD
    BB
    BY
    BE
    BZ
    BJ
    BM
    BT
    BO
    BQ
    BA
    BW
    BV
    BR
    IO
    BN
    BG
    BF
    BI
    KH
    CM
    CA
    CV
    KY
    CF
    TD
    CL
    CN
    CX
    CC
    CO
    KM
    CG
    CD
    CK
    CR
    CI
    HR
    CU
    CW
    CY
    CZ
    DK
    DJ
    DM
    DO
    EC
    EG
    SV
    GQ
    ER
    EE
    ET
    FK
    FO
    FJ
    FI
    FR
    GF
    PF
    TF
    GA
    GM
    GE
    DE
    GH
    GI
    GR
    GL
    GD
    GP
    GU
    GT
    GG
    GN
    GW
    GY
    HT
    HM
    VA
    HN
    HK
    HU
    IS
    IN
    ID
    IR
    IQ
    IE
    IM
    IL
    IT
    JM
    JP
    JE
    JO
    KZ
    KE
    KI
    KP
    KR
    KW
    KG
    LA
    LV
    LB
    LS
    LR
    LY
    LI
    LT
    LU
    MO
    MK
    MG
    MW
    MY
    MV
    ML
    MT
    MH
    MQ
    MR
    MU
    YT
    MX
    FM
    MD
    MC
    MN
    ME
    MS
    MA
    MZ
    MM
    NA
    NR
    NP
    NL
    NC
    NZ
    NI
    NE
    NG
    NU
    NF
    MP
    NO
    OM
    PK
    PW
    PS
    PA
    PG
    PY
    PE
    PH
    PN
    PL
    PT
    PR
    QA
    RE
    RO
    RU
    RW
    BL
    SH
    KN
    LC
    MF
    PM
    VC
    WS
    SM
    ST
    SA
    SN
    RS
    SC
    SL
    SG
    SX
    SK
    SI
    SB
    SO
    ZA
    GS
    SS
    ES
    LK
    SD
    SR
    SJ
    SZ
    SE
    CH
    SY
    TW
    TJ
    TZ
    TH
    TL
    TG
    TK
    TO
    TT
    TN
    TR
    TM
    TC
    TV
    UG
    UA
    AE
    GB
    UM
    UY
    UZ
    VU
    VE
    VN
    VG
    VI
    WF
    EH
    YE
    ZM
    ZW
    US
}

enum CountryAlpha3 {
    AFG
    ALA
    ALB
    DZA
    ASM
    AND
    AGO
    AIA
    ATA
    ATG
    ARG
    ARM
    ABW
    AUS
    AUT
    AZE
    BHS
    BHR
    BGD
    BRB
    BLR
    BEL
    BLZ
    BEN
    BMU
    BTN
    BOL
    BES
    BIH
    BWA
    BVT
    BRA
    IOT
    BRN
    BGR
    BFA
    BDI
    CPV
    KHM
    CMR
    CAN
    CYM
    CAF
    TCD
    CHL
    CHN
    CXR
    CCK
    COL
    COM
    COG
    COD
    COK
    CRI
    CIV
    HRV
    CUB
    CUW
    CYP
    CZE
    DNK
    DJI
    DMA
    DOM
    ECU
    EGY
    SLV
    GNQ
    ERI
    EST
    ETH
    FLK
    FRO
    FJI
    FIN
    FRA
    GUF
    PYF
    ATF
    GAB
    GMB
    GEO
    DEU
    GHA
    GIB
    GRC
    GRL
    GRD
    GLP
    GUM
    GTM
    GGY
    GIN
    GNB
    GUY
    HTI
    HMD
    VAT
    HND
    HKG
    HUN
    ISL
    IND
    IDN
    IRN
    IRQ
    IRL
    IMN
    ISR
    ITA
    JAM
    JPN
    JEY
    JOR
    KAZ
    KEN
    KIR
    PRK
    KOR
    KWT
    KGZ
    LAO
    LVA
    LBN
    LSO
    LBR
    LBY
    LIE
    LTU
    LUX
    MAC
    MKD
    MDG
    MWI
    MYS
    MDV
    MLI
    MLT
    MHL
    MTQ
    MRT
    MUS
    MYT
    MEX
    FSM
    MDA
    MCO
    MNG
    MNE
    MSR
    MAR
    MOZ
    MMR
    NAM
    NRU
    NPL
    NLD
    NCL
    NZL
    NIC
    NER
    NGA
    NIU
    NFK
    MNP
    NOR
    OMN
    PAK
    PLW
    PSE
    PAN
    PNG
    PRY
    PER
    PHL
    PCN
    POL
    PRT
    PRI
    QAT
    REU
    ROU
    RUS
    RWA
    BLM
    SHN
    KNA
    LCA
    MAF
    SPM
    VCT
    WSM
    SMR
    STP
    SAU
    SEN
    SRB
    SYC
    SLE
    SGP
    SXM
    SVK
    SVN
    SLB
    SOM
    ZAF
    SGS
    SSD
    ESP
    LKA
    SDN
    SUR
    SJM
    SWZ
    SWE
    CHE
    SYR
    TWN
    TJK
    TZA
    THA
    TLS
    TGO
    TKL
    TON
    TTO
    TUN
    TUR
    TKM
    TCA
    TUV
    UGA
    UKR
    ARE
    GBR
    USA
    UMI
    URY
    UZB
    VUT
    VEN
    VNM
    VGB
    VIR
    WLF
    ESH
    YEM
    ZMB
    ZWE
}

enum CtpServiceProvider {
    visa
    mastercard
}

/// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
enum Currency {
    AED
    AFN
    ALL
    AMD
    ANG
    AOA
    ARS
    AUD
    AWG
    AZN
    BAM
    BBD
    BDT
    BGN
    BHD
    BIF
    BMD
    BND
    BOB
    BRL
    BSD
    BTN
    BWP
    BYN
    BZD
    CAD
    CDF
    CHF
    CLF
    CLP
    CNY
    COP
    CRC
    CUC
    CUP
    CVE
    CZK
    DJF
    DKK
    DOP
    DZD
    EGP
    ERN
    ETB
    EUR
    FJD
    FKP
    GBP
    GEL
    GHS
    GIP
    GMD
    GNF
    GTQ
    GYD
    HKD
    HNL
    HRK
    HTG
    HUF
    IDR
    ILS
    INR
    IQD
    IRR
    ISK
    JMD
    JOD
    JPY
    KES
    KGS
    KHR
    KMF
    KPW
    KRW
    KWD
    KYD
    KZT
    LAK
    LBP
    LKR
    LRD
    LSL
    LYD
    MAD
    MDL
    MGA
    MKD
    MMK
    MNT
    MOP
    MRU
    MUR
    MVR
    MWK
    MXN
    MYR
    MZN
    NAD
    NGN
    NIO
    NOK
    NPR
    NZD
    OMR
    PAB
    PEN
    PGK
    PHP
    PKR
    PLN
    PYG
    QAR
    RON
    RSD
    RUB
    RWF
    SAR
    SBD
    SCR
    SDG
    SEK
    SGD
    SHP
    SLE
    SLL
    SOS
    SRD
    SSP
    STD
    STN
    SVC
    SYP
    SZL
    THB
    TJS
    TMT
    TND
    TOP
    TRY
    TTD
    TWD
    TZS
    UAH
    UGX
    USD
    UYU
    UZS
    VES
    VND
    VUV
    WST
    XAF
    XCD
    XOF
    XPF
    YER
    ZAR
    ZMW
    ZWL
}

enum CustomerDeviceDisplaySize {
    size320x568
    size375x667
    size390x844
    size414x896
    size428x926
    size768x1024
    size834x1112
    size834x1194
    size1024x1366
    size1280x720
    size1366x768
    size1440x900
    size1920x1080
    size2560x1440
    size3840x2160
    size500x600
    size600x400
    size360x640
    size412x915
    size800x1280
}

enum CustomerDevicePlatform {
    web
    android
    ios
}

enum CustomerDeviceType {
    mobile
    tablet
    desktop
    gaming_console
}

enum DecoupledAuthenticationType {
    challenge
    frictionless
}

/// Device Channel indicating whether request is coming from App or Browser
enum DeviceChannel {
    APP
    BRW
}

enum DynamicRoutingConfigParams {
    PaymentMethod
    PaymentMethodType
    AuthenticationType
    Currency
    Country
    CardNetwork
    CardBin
}

enum DynamicRoutingFeatures {
    metrics
    dynamic_connector_selection
    none
}

enum ElementPosition {
    left
    topleft = "top left"
    top
    topright = "top right"
    right
    bottomright = "bottom right"
    bottom
    bottomleft = "bottom left"
    center
}

enum ErrorCategory {
    frm_decline
    processor_downtime
    processor_decline_unauthorized
    issue_with_payment_method
    processor_decline_incorrect_data
}

enum EventClass {
    payments
    refunds
    disputes
    mandates
    payouts
}

enum EventType {
    payment_succeeded
    payment_failed
    payment_processing
    payment_cancelled
    payment_authorized
    payment_captured
    action_required
    refund_succeeded
    refund_failed
    dispute_opened
    dispute_expired
    dispute_accepted
    dispute_cancelled
    dispute_challenged
    dispute_won
    dispute_lost
    mandate_active
    mandate_revoked
    payout_success
    payout_failed
    payout_initiated
    payout_processing
    payout_cancelled
    payout_expired
    payout_reversed
}

/// The status of the feature
enum FeatureStatus {
    not_supported
    supported
}

enum FieldTypeOneOfAlt0 {
    user_card_number
}

enum FieldTypeOneOfAlt1 {
    user_card_expiry_month
}

enum FieldTypeOneOfAlt11 {
    user_crypto_currency_network
}

enum FieldTypeOneOfAlt12 {
    user_billing_name
}

enum FieldTypeOneOfAlt13 {
    user_address_line1
}

enum FieldTypeOneOfAlt14 {
    user_address_line2
}

enum FieldTypeOneOfAlt15 {
    user_address_city
}

enum FieldTypeOneOfAlt16 {
    user_address_pincode
}

enum FieldTypeOneOfAlt17 {
    user_address_state
}

enum FieldTypeOneOfAlt19 {
    user_shipping_name
}

enum FieldTypeOneOfAlt2 {
    user_card_expiry_year
}

enum FieldTypeOneOfAlt20 {
    user_shipping_address_line1
}

enum FieldTypeOneOfAlt21 {
    user_shipping_address_line2
}

enum FieldTypeOneOfAlt22 {
    user_shipping_address_city
}

enum FieldTypeOneOfAlt23 {
    user_shipping_address_pincode
}

enum FieldTypeOneOfAlt24 {
    user_shipping_address_state
}

enum FieldTypeOneOfAlt26 {
    user_social_security_number
}

enum FieldTypeOneOfAlt27 {
    user_blik_code
}

enum FieldTypeOneOfAlt28 {
    user_bank
}

enum FieldTypeOneOfAlt29 {
    user_bank_account_number
}

enum FieldTypeOneOfAlt3 {
    user_card_cvc
}

enum FieldTypeOneOfAlt30 {
    user_source_bank_account_id
}

enum FieldTypeOneOfAlt31 {
    user_destination_bank_account_id
}

enum FieldTypeOneOfAlt32 {
    text
}

enum FieldTypeOneOfAlt34 {
    user_date_of_birth
}

enum FieldTypeOneOfAlt35 {
    user_vpa_id
}

enum FieldTypeOneOfAlt37 {
    user_pix_key
}

enum FieldTypeOneOfAlt38 {
    user_cpf
}

enum FieldTypeOneOfAlt39 {
    user_cnpj
}

enum FieldTypeOneOfAlt4 {
    user_card_network
}

enum FieldTypeOneOfAlt40 {
    user_iban
}

enum FieldTypeOneOfAlt41 {
    user_bsb_number
}

enum FieldTypeOneOfAlt42 {
    user_bank_sort_code
}

enum FieldTypeOneOfAlt43 {
    user_bank_routing_number
}

enum FieldTypeOneOfAlt44 {
    user_msisdn
}

enum FieldTypeOneOfAlt45 {
    user_client_identifier
}

enum FieldTypeOneOfAlt46 {
    order_details_product_name
}

enum FieldTypeOneOfAlt5 {
    user_full_name
}

enum FieldTypeOneOfAlt6 {
    user_email_address
}

enum FieldTypeOneOfAlt7 {
    user_phone_number
}

enum FieldTypeOneOfAlt8 {
    user_phone_number_country_code
}

enum FrmAction {
    cancel_txn
    auto_refund
    manual_review
}

enum FrmPreferredFlowTypes {
    pre
    post
}

/// Specifies how the payment method can be used for future payments.
/// - `off_session`: The payment method can be used for future payments when the customer is not present.
/// - `on_session`: The payment method is intended for use only when the customer is present during checkout.
/// If omitted, defaults to `on_session`.
enum FutureUsage {
    off_session
    on_session
}

document GcashRedirection

document GooglePayRedirectData

document GooglePayThirdPartySdkData

document GoPayRedirection

enum GpayBillingAddressFormat {
    FULL
    MIN
}

enum GsmDecision {
    retry
    requeue
    do_default
}

/// Represents the overall status of a payment intent.
/// The status transitions through various states depending on the payment method, confirmation, capture method, and any subsequent actions (like customer authentication or manual capture).
enum IntentStatus {
    succeeded
    failed
    cancelled
    processing
    requires_customer_action
    requires_merchant_action
    requires_payment_method
    requires_confirmation
    requires_capture
    partially_captured
    partially_captured_and_capturable
    conflicted
}

document KakaoPayRedirection

/// The status of the mandate, which indicates whether it can be used to initiate a payment.
enum MandateStatus {
    active
    inactive
    pending
    revoked
}

enum MerchantAccountRequestType {
    standard
    connected
}

enum MerchantAccountType {
    standard
    platform
    connected
}

enum MerchantCategoryCode {
    n5411 = "5411"
    n7011 = "7011"
    n0763 = "0763"
    n8111 = "8111"
    n5021 = "5021"
    n4816 = "4816"
    n5661 = "5661"
}

enum MerchantProductType {
    orchestration
    vault
    recon
    recovery
    cost_observability
    dynamic_routing
}

enum MethodKey {
    threeDSMethodData
}

/// This Unit struct represents MinorUnit in which core amount works
long MinorUnit

enum MobilePaymentConsent {
    consent_required
    consent_not_required
    consent_optional
}

document MobilePayRedirection

document MomoRedirection

enum NextActionCall {
    post_session_tokens
    confirm
    sync
    complete_authorize
}

enum NextActionDataOneOfAlt0Type {
    redirect_to_url
}

enum NextActionDataOneOfAlt10Type {
    collect_otp
}

enum NextActionDataOneOfAlt11Type {
    invoke_hidden_iframe
}

enum NextActionDataOneOfAlt1Type {
    redirect_inside_popup
}

enum NextActionDataOneOfAlt2Type {
    display_bank_transfer_information
}

enum NextActionDataOneOfAlt3Type {
    third_party_sdk_session_token
}

enum NextActionDataOneOfAlt4Type {
    qr_code_information
}

enum NextActionDataOneOfAlt5Type {
    fetch_qr_code_information
}

enum NextActionDataOneOfAlt6Type {
    display_voucher_information
}

enum NextActionDataOneOfAlt7Type {
    wait_screen_information
}

enum NextActionDataOneOfAlt8Type {
    three_ds_invoke
}

enum NextActionDataOneOfAlt9Type {
    invoke_sdk_client
}

enum NextActionType {
    redirect_to_url
    display_qr_code
    invoke_sdk_client
    trigger_api
    display_bank_transfer_information
    display_wait_screen
    collect_otp
    redirect_inside_popup
}

enum OutgoingWebhookContentOneOfAlt0Type {
    payment_details
}

enum OutgoingWebhookContentOneOfAlt1Type {
    refund_details
}

enum OutgoingWebhookContentOneOfAlt2Type {
    dispute_details
}

enum OutgoingWebhookContentOneOfAlt3Type {
    mandate_details
}

enum OutgoingWebhookContentOneOfAlt4Type {
    payout_details
}

/// Connector Access Method
enum PaymentConnectorCategory {
    payment_gateway
    alternative_payment_method
    bank_acquirer
}

/// To indicate the type of payment experience that the customer would go through
enum PaymentExperience {
    redirect_to_url
    invoke_sdk_client
    display_qr_code
    one_click
    link_wallet
    invoke_payment_app
    display_wait_screen
    collect_otp
}

enum PaymentLinkDetailsLayout {
    layout1
    layout2
}

enum PaymentLinkSdkLabelType {
    above
    floating
    never
}

enum PaymentLinkShowSdkTerms {
    always
    auto
    never
}

/// Status Of the Payment Link
enum PaymentLinkStatus {
    active
    expired
}

/// Indicates the type of payment method. Eg: 'card', 'wallet', etc.
enum PaymentMethod {
    card
    card_redirect
    pay_later
    wallet
    bank_redirect
    bank_transfer
    crypto
    bank_debit
    reward
    real_time_payment
    upi
    voucher
    gift_card
    open_banking
    mobile_payment
}

enum PaymentMethodDataOneOfAlt10 {
    reward
}

enum PaymentMethodDataOneOfAlt9 {
    mandate_payment
}

enum PaymentMethodIssuerCode {
    jp_hdfc
    jp_icici
    jp_googlepay
    jp_applepay
    jp_phonepay
    jp_wechat
    jp_sofort
    jp_giropay
    jp_sepa
    jp_bacs
}

/// Payment Method Status
enum PaymentMethodStatus {
    active
    inactive
    processing
    awaiting_data
}

/// Indicates the sub type of payment method. Eg: 'google_pay' & 'apple_pay' for wallets.
enum PaymentMethodType {
    ach
    affirm
    afterpay_clearpay
    alfamart
    ali_pay
    ali_pay_hk
    alma
    amazon_pay
    apple_pay
    atome
    bacs
    bancontact_card
    becs
    benefit
    bizum
    blik
    boleto
    bca_bank_transfer
    bni_va
    bri_va
    card_redirect
    cimb_va
    classic
    credit
    crypto_currency
    cashapp
    dana
    danamon_va
    debit
    duit_now
    efecty
    eft
    eps
    fps
    evoucher
    giropay
    givex
    google_pay
    go_pay
    gcash
    ideal
    interac
    indomaret
    klarna
    kakao_pay
    local_bank_redirect
    mandiri_va
    knet
    mb_way
    mobile_pay
    momo
    momo_atm
    multibanco
    online_banking_thailand
    online_banking_czech_republic
    online_banking_finland
    online_banking_fpx
    online_banking_poland
    online_banking_slovakia
    oxxo
    pago_efectivo
    permata_bank_transfer
    open_banking_uk
    pay_bright
    paypal
    paze
    pix
    pay_safe_card
    przelewy24
    prompt_pay
    pse
    red_compra
    red_pagos
    samsung_pay
    sepa
    sepa_bank_transfer
    sofort
    swish
    touch_n_go
    trustly
    twint
    upi_collect
    upi_intent
    vipps
    viet_qr
    venmo
    walley
    we_chat_pay
    seven_eleven
    lawson
    mini_stop
    family_mart
    seicomart
    pay_easy
    local_bank_transfer
    mifinity
    open_banking_pis
    direct_carrier_billing
    instant_bank_transfer
    instant_bank_transfer_finland
    instant_bank_transfer_poland
    revolut_pay
}

enum PaymentProcessingDetailsAtOneOfAlt0AllOf1PaymentProcessingDetailsAt {
    Hyperswitch
}

enum PaymentProcessingDetailsAtOneOfAlt1PaymentProcessingDetailsAt {
    Connector
}

/// The type of the payment that differentiates between normal and various types of mandate payments. Use 'setup_mandate' in case of zero auth flow.
enum PaymentType {
    normal
    new_mandate
    setup_mandate
    recurring_mandate
}

enum PayoutConnectors {
    adyen
    adyenplatform
    cybersource
    ebanx
    nomupay
    payone
    paypal
    stripe
    wise
}

/// Type of entity to whom the payout is being carried out to, select from the given list of options
enum PayoutEntityType {
    Individual
    Company
    NonProfit
    PublicSector
    NaturalPerson
    lowercase
    Personal
}

/// The send method which will be required for processing payouts, check options for better understanding.
enum PayoutSendPriority {
    instant
    fast
    regular
    wire
    cross_border
    internal
}

enum PayoutStatus {
    success
    failed
    cancelled
    initiated
    expired
    reversed
    pending
    ineligible
    requires_creation
    requires_confirmation
    requires_payout_method_data
    requires_fulfillment
    requires_vendor_account_creation
}

/// The payout_type of the payout request is a mandatory field for confirming the payouts. It should be specified in the Create request. If not provided, it must be updated in the Payout Update request before it can be confirmed.
enum PayoutType {
    card
    bank
    wallet
}

enum PollStatus {
    pending
    completed
    not_found
}

enum ProductType {
    physical
    digital
    travel
    ride
    event
    accommodation
}

enum ReconStatus {
    not_requested
    requested
    active
    disabled
}

enum RecurringDetailsOneOfAlt0Type {
    mandate_id
}

enum RecurringDetailsOneOfAlt1Type {
    payment_method_id
}

enum RecurringDetailsOneOfAlt2Type {
    processor_payment_token
}

enum RecurringDetailsOneOfAlt3Type {
    network_transaction_id_and_card_details
}

enum RecurringPaymentIntervalUnit {
    year
    month
    day
    hour
    minute
}

/// The status for refunds
enum RefundStatus {
    succeeded
    failed
    pending
    review
}

/// To indicate whether to refund needs to be instant or scheduled
enum RefundType {
    scheduled
    instant
}

enum RelayStatus {
    created
    pending
    success
    failure
}

enum RelayType {
    refund
}

/// Denotes the retry action
enum RetryAction {
    manual_retry
    requeue
}

document RevolutPayData

enum RoutableChoiceKind {
    OnlyConnector
    FullStruct
}

/// RoutableConnectors are the subset of Connectors that are eligible for payments routing
enum RoutableConnectors {
    authipay
    adyenplatform
    stripe_billing_test
    phonypay
    fauxpay
    pretendpay
    stripe_test
    adyen_test
    checkout_test
    paypal_test
    aci
    adyen
    airwallex
    archipel
    authorizedotnet
    bankofamerica
    barclaycard
    billwerk
    bitpay
    bambora
    bamboraapac
    bluesnap
    boku
    braintree
    cashtocode
    celero
    chargebee
    checkout
    coinbase
    coingate
    cryptopay
    cybersource
    datatrans
    deutschebank
    digitalvirgo
    dlocal
    ebanx
    elavon
    facilitapay
    fiserv
    fiservemea
    fiuu
    forte
    getnet
    globalpay
    globepay
    gocardless
    hipay
    helcim
    iatapay
    inespay
    itaubank
    jpmorgan
    klarna
    mifinity
    mollie
    moneris
    multisafepay
    nexinets
    nexixpay
    nmi
    nomupay
    noon
    novalnet
    nuvei
    opennode
    paybox
    payme
    payload
    payone
    paypal
    paystack
    payu
    placetopay
    powertranz
    prophetpay
    rapyd
    razorpay
    recurly
    redsys
    riskified
    santander
    shift4
    signifyd
    square
    stax
    stripe
    stripebilling
    trustpay
    tokenio
    tsys
    volt
    wellsfargo
    wise
    worldline
    worldpay
    worldpayvantiv
    worldpayxml
    xendit
    zen
    plaid
    zsl
}

enum RoutingAlgorithmKind {
    single
    priority
    volume_split
    advanced
    dynamic
    three_ds_decision_rule
}

enum SamsungPayAmountFormat {
    FORMAT_TOTAL_PRICE_ONLY
    FORMAT_TOTAL_ESTIMATED_AMOUNT
}

enum SamsungPayCardBrand {
    visa
    mastercard
    amex
    discover
    unknown
}

enum SamsungPayProtocolType {
    PROTOCOL3DS
}

/// SCA Exemptions types available for authentication
enum ScaExemptionType {
    low_value
    transaction_risk_analysis
}

/// Enum representing the type of 3DS SDK.
enum SdkType {
    n01 = "01"
    n02 = "02"
    n03 = "03"
    n04 = "04"
    n05 = "05"
}

enum SessionTokenOneOfAlt0AllOf1WalletName {
    google_pay
}

enum SessionTokenOneOfAlt1AllOf1WalletName {
    samsung_pay
}

enum SessionTokenOneOfAlt2AllOf1WalletName {
    klarna
}

enum SessionTokenOneOfAlt3AllOf1WalletName {
    paypal
}

enum SessionTokenOneOfAlt4AllOf1WalletName {
    apple_pay
}

enum SessionTokenOneOfAlt5AllOf1WalletName {
    open_banking
}

enum SessionTokenOneOfAlt6AllOf1WalletName {
    paze
}

enum SessionTokenOneOfAlt7AllOf1WalletName {
    click_to_pay
}

enum SessionTokenOneOfAlt8WalletName {
    no_session_token_received
}

enum SizeVariants {
    cover
    contain
}

enum StaticRoutingAlgorithmOneOfAlt0Type {
    single
}

enum StaticRoutingAlgorithmOneOfAlt1Type {
    priority
}

enum StaticRoutingAlgorithmOneOfAlt2Type {
    volume_split
}

enum StaticRoutingAlgorithmOneOfAlt3Type {
    advanced
}

enum StaticRoutingAlgorithmOneOfAlt4Type {
    three_ds_decision_rule
}

enum StraightThroughAlgorithmOneOfAlt0Type {
    single
}

enum StraightThroughAlgorithmOneOfAlt1Type {
    priority
}

enum StraightThroughAlgorithmOneOfAlt2Type {
    volume_split
}

/// Connector specific types to send
string StringMinorUnit

enum StripeChargeType {
    direct
    destination
}

enum SuccessRateSpecificityLevel {
    merchant
    global
}

enum SurchargeResponseOneOfAlt0Type {
    fixed
}

enum SurchargeResponseOneOfAlt1Type {
    rate
}

document SwishQrData

/// Indicates if 3DS method data was successfully completed or not
enum ThreeDsCompletionIndicator {
    Y
    N
    U
}

/// Enum representing the possible outcomes of the 3DS Decision Rule Engine.
enum ThreeDSDecision {
    no_three_ds
    challenge_requested
    challenge_preferred
    three_ds_exemption_requested_tra
    three_ds_exemption_requested_low_value
    issuer_three_ds_exemption_requested
}

enum ThreeDsMethodKey {
    threeDSMethodData
}

/// The type of token data to fetch for get-token endpoint
enum TokenDataType {
    single_use_token
    multi_use_token
    network_token
}

document TouchNGoRedirection

/// Indicates the transaction status
enum TransactionStatus {
    Y
    N
    U
    A
    R
    C
    D
    I
}

enum TransactionType {
    payment
    payout
    three_ds_authentication
}

enum TriggeredBy {
    internal
    external
}

enum UIWidgetFormLayout {
    tabs
    journey
}

document UpiIntentData

enum ValueTypeOneOfAlt0Type {
    number
}

enum ValueTypeOneOfAlt1Type {
    enum_variant
}

enum ValueTypeOneOfAlt2Type {
    metadata_variant
}

enum ValueTypeOneOfAlt3Type {
    str_value
}

enum ValueTypeOneOfAlt4Type {
    number_array
}

enum ValueTypeOneOfAlt5Type {
    enum_variant_array
}

enum ValueTypeOneOfAlt6Type {
    number_comparison_array
}

enum VoucherDataOneOfAlt1 {
    efecty
}

enum VoucherDataOneOfAlt2 {
    pago_efectivo
}

enum VoucherDataOneOfAlt3 {
    red_compra
}

enum VoucherDataOneOfAlt4 {
    red_pagos
}

enum VoucherDataOneOfAlt7 {
    oxxo
}

enum WebhookDeliveryAttempt {
    initial_attempt
    automatic_retry
    manual_retry
}

document WeChatPay

document WeChatPayQr

document WeChatPayRedirection
