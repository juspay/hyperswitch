$version: "2"

namespace com.hyperswitch

use com.hyperswitch.smithy.types#PaymentsRequest
use com.hyperswitch.smithy.types#PaymentsResponse

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
    operations: [PaymentsCreate]
}

@documentation("Create a payment with the specified details.")
@http(method: "POST", uri: "/payments")
operation PaymentsCreate {
    input: PaymentsRequest,
    output: PaymentsResponse,
}
