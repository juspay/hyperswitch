$version: "2"

namespace com.hyperswitch

use com.hyperswitch.smithy.types#PaymentsRequest
use com.hyperswitch.smithy.types#PaymentsResponse
use com.hyperswitch.smithy.types#RefundRequest
use com.hyperswitch.smithy.types#RefundResponse

use aws.protocols#restJson1

/// The Hyperswitch API.
@restJson1
@aws.api#service(
    sdkId: "hyperswitch",
    arnNamespace: "hyperswitch",
    cloudFormationName: "Hyperswitch",
    endpointPrefix: "api"
)
service Hyperswitch {
    version: "2024-07-31",
    operations: [PaymentsCreate, PaymentsRetrieve, RefundsCreate, RefundsRetrieve]
}

@documentation("Retrieve a refund using the refund_id.")
@http(method: "GET", uri: "/refunds/{id}")
operation RefundsRetrieve {
    input: RefundsRetrieveRequest,
    output: RefundResponse,
}

structure RefundsRetrieveRequest {
    /// The unique identifier for the refund to retrieve
    @required
    @httpLabel
    id: smithy.api#String
}

@documentation("Create a refund for a payment.")
@http(method: "POST", uri: "/refunds")
operation RefundsCreate {
    input: RefundRequest,
    output: RefundResponse,
}

@documentation("Retrieve a payment using the payment_id.")
@http(method: "GET", uri: "/payments/{payment_id}")
operation PaymentsRetrieve {
    input: PaymentsRetrieveRequest,
    output: PaymentsResponse,
}

structure PaymentsRetrieveRequest {
    /// The unique identifier for the payment to retrieve
    @required
    @httpLabel
    payment_id: smithy.api#String
}

@documentation("Create a payment with the specified details.")
@http(method: "POST", uri: "/payments")
operation PaymentsCreate {
    input: PaymentsRequest,
    output: PaymentsResponse,
}
