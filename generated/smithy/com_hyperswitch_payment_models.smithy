$version: "2"

namespace com.hyperswitch.payment.models

structure PaymentsRequest {
    /// Passing this object during payments creates a mandate. The mandate_type sub object is passed by the server.
    mandate_data: com.hyperswitch.types#MandateData
    /// Custom payment link config id set at business profile, send only if business_specific_configs is configured
    payment_link_config_id: smithy.api#String
    /// Whether to perform external authentication (if applicable)
    request_external_three_ds_authentication: smithy.api#Boolean
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: smithy.api#Document
    /// A timestamp (ISO 8601 code) that determines when the payment should be captured. Providing this field will automatically set `capture` to true
    capture_on: com.hyperswitch.types#PrimitiveDateTime
    /// An arbitrary string attached to the payment. Often useful for displaying to users or for your own internal record-keeping.
    description: smithy.api#String
    /// Some connectors like Apple pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.
    connector_metadata: com.hyperswitch.types#ConnectorMetadata
    /// To indicate the type of payment experience that the payment method would go through
    payment_experience: com.hyperswitch.types#api_enums::PaymentExperience
    payment_link_config: com.hyperswitch.types#PaymentCreatePaymentLinkConfig
    /// Whether to calculate tax for this payment intent
    skip_external_tax_calculation: smithy.api#Boolean
    /// Indicates if the redirection has to open in the iframe
    is_iframe_redirection_enabled: smithy.api#Boolean
    surcharge_details: com.hyperswitch.types#RequestSurchargeDetails
    /// A unique identifier to link the payment to a mandate. To do Recurring payments after a mandate has been created, pass the mandate_id instead of payment_method_data
    mandate_id: smithy.api#String
    /// Total tax amount applicable to the order, in the lowest denomination of the currency.
    order_tax_amount: smithy.api#Long
    /// Optional boolean value to extent authorization period of this payment  capture method must be manual or manual_multiple
    request_extended_authorization: com.hyperswitch.types#RequestExtendedAuthorizationBool
    /// Service details for click to pay external authentication
    ctp_service_details: com.hyperswitch.types#CtpServiceDetails
    /// If enabled, provides whole connector response
    all_keys_required: smithy.api#Boolean
    /// Set to true to indicate that the customer is not in your checkout flow during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you collect card details and charge them later. When making a recurring payment by passing a mandate_id, this parameter is mandatory
    off_session: smithy.api#Boolean
    /// Use this parameter to restrict the Payment Method Types to show for a given PaymentIntent
    allowed_payment_method_types: smithy.api#List<com.hyperswitch.types#api_enums::PaymentMethodType>
    /// Indicates if 3DS method data was successfully completed or not
    threeds_method_comp_ind: com.hyperswitch.types#ThreeDsCompletionIndicator
    /// The customer's name. This field will be deprecated soon, use the customer object instead.
    name: smithy.api#String
    capture_method: com.hyperswitch.types#api_enums::CaptureMethod
    payment_method_data: com.hyperswitch.types#PaymentMethodDataRequest
    /// Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins
    session_expiry: smithy.api#Integer
    merchant_connector_details: com.hyperswitch.types#admin::MerchantConnectorDetailsWrap
    /// Passing this object creates a new customer or attaches an existing customer to the payment
    customer: com.hyperswitch.types#CustomerDetails
    /// Additional details required by 3DS 2.0
    browser_info: smithy.api#Document
    /// Whether to generate the payment link for this payment or not (if applicable)
    payment_link: smithy.api#Boolean
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
    /// The shipping address for the payment
    shipping: com.hyperswitch.types#Address
    /// Business country of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_country: com.hyperswitch.types#api_enums::CountryAlpha2
    /// This is used along with the payment_token field while collecting during saved card payments. This field will be deprecated soon, use the payment_method_data.card_token object instead
    card_cvc: smithy.api#String
    /// The three-letter ISO 4217 currency code (e.g., "USD", "EUR") for the payment amount. This field is mandatory for creating a payment.
    currency: com.hyperswitch.types#api_enums::Currency
    /// The billing details of the payment. This address will be used for invoicing.
    billing: com.hyperswitch.types#Address
    /// Request an incremental authorization, i.e., increase the authorized amount on a confirmed payment before you capture it.
    request_incremental_authorization: smithy.api#Boolean
    /// Denotes the retry action
    retry_action: com.hyperswitch.types#api_enums::RetryAction
    /// The amount to be captured from the user's payment method, in the lowest denomination. If not provided, and `capture_method` is `automatic`, the full payment `amount` will be captured. If `capture_method` is `manual`, this can be specified in the `/capture` call. Must be less than or equal to the authorized amount.
    amount_to_capture: smithy.api#Long
    /// Use this object to capture the details about the different products for which the payment is being made. The sum of amount across different products here should be equal to the overall payment amount
    order_details: smithy.api#List<com.hyperswitch.types#OrderDetailsWithAmount>
    /// If set to `true`, Hyperswitch attempts to confirm and authorize the payment immediately after creation, provided sufficient payment method details are included. If `false` or omitted (default is `false`), the payment is created with a status such as `requires_payment_method` or `requires_confirmation`, and a separate `POST /payments/{payment_id}/confirm` call is necessary to proceed with authorization.
    confirm: smithy.api#Boolean
    /// The identifier for the customer
    customer_id: com.hyperswitch.types#id_type::CustomerId
    authentication_type: com.hyperswitch.types#api_enums::AuthenticationType
    /// Can be used to specify the Payment Method Type
    payment_method_type: com.hyperswitch.types#api_enums::PaymentMethodType
    /// Business label of the merchant for this payment. To be deprecated soon. Pass the profile_id instead
    business_label: smithy.api#String
    /// Provides information about a card payment that customers see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
    statement_descriptor_suffix: smithy.api#String
    /// Choose what kind of sca exemption is required for this payment
    psd2_sca_exemption_type: com.hyperswitch.types#api_enums::ScaExemptionType
    /// Your unique identifier for this payment or order. This ID helps you reconcile payments on your system. If provided, it is passed to the connector if supported.
    merchant_order_reference_id: smithy.api#String
    /// The customer's email address. This field will be deprecated soon, use the customer object instead
    email: com.hyperswitch.types#Email
    /// The business profile to be used for this payment, if not passed the default business profile associated with the merchant account will be used. It is mandatory in case multiple business profiles have been set up.
    profile_id: com.hyperswitch.types#id_type::ProfileId
    /// The country code for the customer phone number This field will be deprecated soon, use the customer object instead
    phone_country_code: smithy.api#String
    /// This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.
    customer_acceptance: com.hyperswitch.types#common_payments_types::CustomerAcceptance
    /// Additional data related to some frm(Fraud Risk Management) connectors
    frm_metadata: com.hyperswitch.types#pii::SecretSerdeValue
    /// Indicates if 3ds challenge is forced
    force_3ds_challenge: smithy.api#Boolean
    /// This allows to manually select a connector with which the payment can go through.
    connector: smithy.api#List<com.hyperswitch.types#api_enums::Connector>
    /// The type of the payment that differentiates between normal and various types of mandate payments
    payment_type: com.hyperswitch.types#api_enums::PaymentType
    /// This is an identifier for the merchant account. This is inferred from the API key provided during the request
    merchant_id: com.hyperswitch.types#id_type::MerchantId
    /// Details required for recurring payment
    recurring_details: com.hyperswitch.types#RecurringDetails
    /// Additional data that might be required by hyperswitch based on the requested features by the merchants.
    feature_metadata: com.hyperswitch.types#FeatureMetadata
    payment_method: com.hyperswitch.types#api_enums::PaymentMethod
    /// Fee information to be charged on the payment being collected
    split_payments: com.hyperswitch.types#common_types::payments::SplitPaymentsRequest
    /// Indicates whether the `payment_id` was provided by the merchant This value is inferred internally based on the request
    @required
    is_payment_id_from_merchant: smithy.api#Boolean
    /// It's a token used for client side verification.
    client_secret: smithy.api#String
    /// The URL to redirect the customer to after they complete the payment process or authentication. This is crucial for flows that involve off-site redirection (e.g., 3DS, some bank redirects, wallet payments).
    return_url: com.hyperswitch.types#Url
    /// As Hyperswitch tokenises the sensitive details about the payments method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive details are not exposed in any manner.
    payment_token: smithy.api#String
    /// The customer's phone number This field will be deprecated soon, use the customer object instead
    phone: smithy.api#String
    /// Optional. A merchant-provided unique identifier for the payment, contains 30 characters long (e.g., "pay_mbabizu24mvu3mela5njyhpit4"). If provided, it ensures idempotency for the payment creation request. If omitted, Hyperswitch generates a unique ID for the payment.
    payment_id: com.hyperswitch.types#PaymentIdType
    /// The shipping cost for the payment. This is required for tax calculation in some regions.
    shipping_cost: smithy.api#Long
    /// For non-card charges, you can use this value as the complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22 characters.
    statement_descriptor_name: smithy.api#String
    /// Details of the routing configuration for that payment
    routing: smithy.api#Document
    /// Business sub label for the payment
    business_sub_label: smithy.api#String
    setup_future_usage: com.hyperswitch.types#api_enums::FutureUsage
}

structure CardToken {
    /// The CVC number for the card
    card_cvc: smithy.api#String
    /// The card holder's name
    card_holder_name: smithy.api#String
}

structure PaymentListConstraints {
    /// Time less than or equals to the payment created time
    created_lte: com.hyperswitch.types#PrimitiveDateTime
    /// A cursor for use in pagination, fetch the next list after some object
    starting_after: com.hyperswitch.types#id_type::PaymentId
    /// limit on the number of objects to return
    @required
    limit: smithy.api#Integer
    /// Time greater than the payment created time
    created_gt: com.hyperswitch.types#PrimitiveDateTime
    /// Time greater than or equals to the payment created time
    created_gte: com.hyperswitch.types#PrimitiveDateTime
    /// The identifier for customer
    customer_id: com.hyperswitch.types#id_type::CustomerId
    /// Time less than the payment created time
    created_lt: com.hyperswitch.types#PrimitiveDateTime
    /// The time at which payment is created
    created: com.hyperswitch.types#PrimitiveDateTime
    /// A cursor for use in pagination, fetch the previous list before some object
    ending_before: com.hyperswitch.types#id_type::PaymentId
}

