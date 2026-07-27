$version: "2"

namespace com.hyperswitch.default

/// Represents the rules for legal protest (official debt registration) for non-payment, including the type of protest and the number of days after which the protest is initiated.
enum BoletoPaymentTypeConstraints {
    /// The payer may make multiple payments, up to a specific limit.
    installment
    /// Only the exact nominal amount can be paid.
    fixed_amount
    /// The payer may pay any amount within an allowed range.
    flexible_amount
}

structure DDCData {
}

/// Represents the rules for legal protest (official debt registration) for non-payment, including the type of protest and the number of days after which the protest is initiated.
structure InstallmentDetails {
    /// Maximum number of partial payments allowed (Up to 99 for Santander).
    max_partial_payments: smithy.api#Integer
    /// Defines if the min/max values are percentages or flat amounts
    value_type: com.hyperswitch.smithy.types#CalculationType
}

/// Source of the token
enum TokenSource {
    /// Google Pay
    google_pay
    /// Apple Pay
    apple_pay
}

/// Represents the rules for legal protest (official debt registration) for non-payment, including the type of protest and the number of days after which the protest is initiated.
structure FlexibleAmountDetails {
    /// Defines if the min/max values are percentages or flat amounts
    value_type: com.hyperswitch.smithy.types#CalculationType
    /// Minimum value allowed (e.g., "10.00")
    min_value: smithy.api#String
    /// Maximum value allowed (e.g., "5000.00")
    max_value: smithy.api#String
}

