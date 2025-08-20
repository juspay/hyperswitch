$version: "2"

namespace com.hyperswitch

use com.hyperswitch.smithy.types#PaymentsRequest
use com.hyperswitch.smithy.types#PaymentsResponse
use com.hyperswitch.smithy.types#RefundRequest
use com.hyperswitch.smithy.types#RefundResponse
use com.hyperswitch.smithy.types#PaymentsCaptureRequest
use com.hyperswitch.smithy.types#CustomerRequest
use com.hyperswitch.smithy.types#CustomerResponse
use com.hyperswitch.smithy.types#CustomerDeleteResponse

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
    operations: [PaymentsCreate, PaymentsRetrieve, RefundsCreate, RefundsRetrieve, PaymentsCapture, CustomersCreate, CustomersRetrieve, CustomersUpdate, CustomersDelete]
}

/// Input structure for capturing a payment
structure PaymentsCaptureRequestInput {
    /// The unique identifier for the payment to capture
    @required
    @httpLabel
    payment_id: smithy.api#String

    /// The capture request details
    @required
    @httpPayload
    payload: PaymentsCaptureRequest
}

@documentation("Capture a payment that has been previously authorized.")
@http(method: "POST", uri: "/payments/{payment_id}/capture")
operation PaymentsCapture {
    input: PaymentsCaptureRequestInput,
    output: PaymentsResponse,
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

/// Structure for creating a refund
structure RefundsCreateRequest {
    /// The refund request details
    @required
    @httpPayload
    payload: RefundRequest
}

@documentation("Create a refund for a payment.")
@http(method: "POST", uri: "/refunds")
operation RefundsCreate {
    input: RefundsCreateRequest,
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

/// Structure for creating a payment
structure PaymentsCreateRequest {
    /// The payment request details
    @required
    @httpPayload
    payload: PaymentsRequest
}

@documentation("Create a payment with the specified details.")
@http(method: "POST", uri: "/payments")
operation PaymentsCreate {
    input: PaymentsCreateRequest,
    output: PaymentsResponse,
}

/// Structure for creating a customer
structure CustomersCreateRequest {
    /// The customer request details
    @required
    @httpPayload
    payload: CustomerRequest
}

@documentation("Create a customer with the specified details.")
@http(method: "POST", uri: "/customers")
operation CustomersCreate {
    input: CustomersCreateRequest,
    output: CustomerResponse,
}

@documentation("Retrieve a customer using the customer_id.")
@http(method: "GET", uri: "/customers/{customer_id}")
operation CustomersRetrieve {
    input: CustomersRetrieveRequest,
    output: CustomerResponse,
}

structure CustomersRetrieveRequest {
    /// The unique identifier for the customer to retrieve
    @required
    @httpLabel
    customer_id: smithy.api#String
}

/// Structure for updating a customer
structure CustomersUpdateRequest {
    /// The unique identifier for the customer to update
    @required
    @httpLabel
    customer_id: smithy.api#String

    /// The customer update request details
    @required
    @httpPayload
    payload: CustomerRequest
}

@documentation("Update a customer using the customer_id.")
@http(method: "POST", uri: "/customers/{customer_id}")
operation CustomersUpdate {
    input: CustomersUpdateRequest,
    output: CustomerResponse,
}

@documentation("Delete a customer using the customer_id.")
@http(method: "DELETE", uri: "/customers/{customer_id}")
operation CustomersDelete {
    input: CustomersDeleteRequest,
    output: CustomerDeleteResponse,
}

structure CustomersDeleteRequest {
    /// The unique identifier for the customer to delete
    @required
    @httpLabel
    customer_id: smithy.api#String
}
