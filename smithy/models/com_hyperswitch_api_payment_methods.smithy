$version: "2"

namespace com.hyperswitch.api.payment_methods

structure PaymentMethodDeleteResponse {
    /// The unique identifier of the Payment method
    @required
    payment_method_id: smithy.api#String
    /// Whether payment method was deleted or not
    @required
    deleted: smithy.api#Boolean
}

