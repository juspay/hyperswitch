$version: "2"

namespace com.hyperswitch.smithy.types

structure PaymentMethodUpdate {
    /// This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK
    @length(min: 30, max: 30)
    client_secret: smithy.api#String
    /// Card Details
    card: CardDetailUpdate
}

structure CardDetailFromLocker {
    expiry_month: smithy.api#String
    card_type: smithy.api#String
    card_fingerprint: smithy.api#String
    expiry_year: smithy.api#String
    card_holder_name: smithy.api#String
    scheme: smithy.api#String
    card_token: smithy.api#String
    last4_digits: smithy.api#String
    card_number: smithy.api#String
    issuer_country: smithy.api#String
    card_network: CardNetwork
    card_isin: smithy.api#String
    nick_name: smithy.api#String
    @required
    saved_to_locker: Boolean
    card_issuer: smithy.api#String
}

structure PaymentMethodResponse {
    /// For Client based calls
    client_secret: smithy.api#String
    /// The type of payment method use for the payment.
    payment_method: PaymentMethod
    /// This is a sub-category of payment method.
    payment_method_type: PaymentMethodType
    /// Indicates whether the payment method supports recurring payments. Optional.
    recurring_enabled: Boolean
    /// Card details from card locker
    card: CardDetailFromLocker
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    metadata: Document
    /// The unique identifier of the Payment method
    @required
    payment_method_id: smithy.api#String
    /// The unique identifier of the customer.
    @length(min: 1, max: 63)
    customer_id: smithy.api#String
    /// Type of payment experience enabled with the connector
    payment_experience: List<PaymentExperience>
    /// Unique identifier for a merchant
    @required
    merchant_id: smithy.api#String
    /// Indicates whether the payment method is eligible for installment payments (e.g., EMI, BNPL). Optional.
    installment_payment_enabled: Boolean
    /// A timestamp (ISO 8601 code) that determines when the payment method was created
    created: Timestamp
    last_used_at: Timestamp
}

structure CardDetailUpdate {
    /// Card Holder's Nick Name
    nick_name: smithy.api#String
    /// Card Expiry Month
    card_exp_month: smithy.api#String
    /// Card Expiry Year
    card_exp_year: smithy.api#String
    /// Card Holder Name
    card_holder_name: smithy.api#String
}

