$version: "2"

namespace com.hyperswitch

use com.hyperswitch.smithy.types#PaymentsRequest
use com.hyperswitch.smithy.types#PaymentsResponse
use com.hyperswitch.smithy.types#RefundRequest
use com.hyperswitch.smithy.types#RefundResponse
use com.hyperswitch.smithy.types#RefundUpdateRequest
use com.hyperswitch.smithy.types#RefundListRequest
use com.hyperswitch.smithy.types#RefundListResponse
use com.hyperswitch.smithy.types#PaymentsCaptureRequest
use com.hyperswitch.smithy.types#PaymentsCancelRequest
use com.hyperswitch.smithy.types#CustomerRequest
use com.hyperswitch.smithy.types#CustomerResponse
use com.hyperswitch.smithy.types#CustomerUpdateRequest
use com.hyperswitch.smithy.types#CustomerDeleteResponse
use com.hyperswitch.smithy.types#CustomerListRequest
use com.hyperswitch.smithy.types#MandateRevokedResponse
use com.hyperswitch.smithy.types#MandateResponse
use com.hyperswitch.smithy.types#MandateListConstraints

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
    operations: [PaymentsCreate, PaymentsConfirm, PaymentsUpdate, PaymentsRetrieve, PaymentsCapture, PaymentsCancel, RefundsCreate, RefundsRetrieve, RefundsUpdate, RefundsList, CustomersCreate, CustomersRetrieve, CustomersUpdate, CustomersDelete, CustomersList, MandatesRevoke, MandatesRetrieve, MandatesList]
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

/// Structure for confirming a payment
structure PaymentsConfirmRequest {
    /// The unique identifier for the payment to confirm
    @required
    @httpLabel
    payment_id: smithy.api#String

    /// The payment confirmation request details
    @required
    @httpPayload
    payload: PaymentsRequest
}

@documentation("Confirm a payment using the payment_id.")
@http(method: "POST", uri: "/payments/{payment_id}/confirm")
operation PaymentsConfirm {
    input: PaymentsConfirmRequest,
    output: PaymentsResponse,
}

/// Structure for updating a payment
structure PaymentsUpdateRequest {
    /// The unique identifier for the payment to update
    @required
    @httpLabel
    payment_id: smithy.api#String

    /// The payment update request details
    @required
    @httpPayload
    payload: PaymentsRequest
}

@documentation("Update a payment using the payment_id.")
@http(method: "POST", uri: "/payments/{payment_id}")
operation PaymentsUpdate {
    input: PaymentsUpdateRequest,
    output: PaymentsResponse,
}

structure PaymentsRetrieveRequest {
    /// The unique identifier for the payment to retrieve
    @required
    @httpLabel
    payment_id: smithy.api#String
}

@documentation("Retrieve a payment using the payment_id.")
@http(method: "GET", uri: "/payments/{payment_id}")
operation PaymentsRetrieve {
    input: PaymentsRetrieveRequest,
    output: PaymentsResponse,
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

/// Input structure for canceling a payment
structure PaymentsCancelRequestInput {
    /// The unique identifier for the payment to cancel
    @required
    @httpLabel
    payment_id: smithy.api#String

    /// The cancel request details
    @required
    @httpPayload
    payload: PaymentsCancelRequest
}

@documentation("Cancel a payment using the payment_id.")
@http(method: "POST", uri: "/payments/{payment_id}/cancel")
operation PaymentsCancel {
    input: PaymentsCancelRequestInput,
    output: PaymentsResponse,
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

structure RefundsRetrieveRequest {
    /// The unique identifier for the refund to retrieve
    @required
    @httpLabel
    id: smithy.api#String
}

@documentation("Retrieve a refund using the refund_id.")
@http(method: "GET", uri: "/refunds/{id}")
operation RefundsRetrieve {
    input: RefundsRetrieveRequest,
    output: RefundResponse,
}

/// Structure for updating a refund
structure RefundsUpdateRequest {
    /// The unique identifier for the refund to update
    @required
    @httpLabel
    id: smithy.api#String

    /// The refund update request details
    @required
    @httpPayload
    payload: RefundUpdateRequest
}

@documentation("Update a refund using the refund_id.")
@http(method: "POST", uri: "/refunds/{id}")
operation RefundsUpdate {
    input: RefundsUpdateRequest,
    output: RefundResponse,
}

/// Structure for listing refunds
structure RefundsListRequestInput {
    /// The refund list request details
    @required
    @httpPayload
    payload: RefundListRequest
}

@documentation("Retrieve a list of refunds.")
@http(method: "POST", uri: "/refunds/list")
operation RefundsList {
    input: RefundsListRequestInput,
    output: RefundListResponse,
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

/// Structure for retrieving a customer
structure CustomersRetrieveRequest {
    /// The unique identifier for the customer to retrieve
    @required
    @httpLabel
    customer_id: smithy.api#String
}

@documentation("Retrieve a customer using the customer_id.")
@http(method: "GET", uri: "/customers/{customer_id}")
operation CustomersRetrieve {
    input: CustomersRetrieveRequest,
    output: CustomerResponse,
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
    payload: CustomerUpdateRequest
}

@documentation("Update a customer using the customer_id.")
@http(method: "POST", uri: "/customers/{customer_id}")
operation CustomersUpdate {
    input: CustomersUpdateRequest,
    output: CustomerResponse,
}

/// Structure for deleting a customer
structure CustomersDeleteRequest {
    /// The unique identifier for the customer to delete
    @required
    @httpLabel
    customer_id: smithy.api#String
}

@documentation("Delete a customer using the customer_id.")
@http(method: "DELETE", uri: "/customers/{customer_id}")
operation CustomersDelete {
    input: CustomersDeleteRequest,
    output: CustomerDeleteResponse,
}

list CustomerResponseList {
    member: CustomerResponse
}

structure CustomersListRequestInput with [CustomerListRequest] {
}

@documentation("Retrieve a list of customers.")
@http(method: "GET", uri: "/customers/list")
operation CustomersList {
    input: CustomersListRequestInput,
    output:= {
        @httpPayload
        customers_list: smithy.api#Document
    },
}

/// Structure for revoking a mandate
structure MandatesRevokeRequest {
    /// The unique identifier for the mandate to revoke
    @required
    @httpLabel
    id: smithy.api#String
}

@documentation("Revoke a mandate using the mandate_id.")
@http(method: "POST", uri: "/mandates/revoke/{id}")
operation MandatesRevoke {
    input: MandatesRevokeRequest,
    output: MandateRevokedResponse,
}

/// Structure for retrieving a mandate
structure MandatesRetrieveRequest {
    /// The unique identifier for the mandate to retrieve
    @required
    @httpLabel
    id: smithy.api#String
}

@documentation("Retrieve a mandate using the mandate_id.")
@http(method: "GET", uri: "/mandates/{id}")
operation MandatesRetrieve {
    input: MandatesRetrieveRequest,
    output: MandateResponse,
}

list MandateResponseList {
    member: MandateResponse
}

structure MandatesListRequestInput with [MandateListConstraints] {
}

@documentation("Retrieve a list of mandates.")
@http(method: "GET", uri: "/mandates/list")
operation MandatesList {
    input: MandatesListRequestInput,
    output:= {
        @httpPayload
        mandates_list: smithy.api#Document
    },
}
