$version: "2"

namespace com.hyperswitch

use com.hyperswitch.smithy.types#PaymentsRequest
use com.hyperswitch.smithy.types#PaymentsResponse

/// The Hyperswitch API.
service Hyperswitch {
    version: "2024-07-31",
    operations: [PaymentsCreate],  // Bind operation directly to service
}

@documentation("Create a payment with the specified details.")
@http(method: "POST", uri: "/payments")
operation PaymentsCreate {
    input: PaymentsRequest,
    output: PaymentsResponse,
}