$version: "2"

namespace com.hyperswitch.payment.models

structure PaymentsRequest {
    /// The primary amount for the payment, provided in the lowest denomination of the specified currency (e.g., 6540 for $65.40 USD). This field is mandatory for creating a payment.
    amount: smithy.api#Long
}

structure CardToken {
}

structure PaymentListConstraints {
}

