# Cancellation (Void) Flow Information

This document consolidates information related to the Cancellation (Void) flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Enum Variants (DLocal Integration Learnings - Session 2)
- **`RequestContent::NoContent`:** For requests with no body (e.g., GET requests, or POSTs that Dlocal expects no body for like Void/Cancel if applicable), use `RequestContent::NoContent`. The `real-codebase` for Dlocal's GET requests in `build_request` simply doesn't call `.set_body()`, which is equivalent. For POSTs that expect no body, `RequestContent::NoContent` is appropriate.

### Other Minor Alignments (DLocal Integration Learnings - Session 8)
- **`get_request_body` for PSync/Void/RSync:** Ensured these return `Ok(RequestContent::NoContent)` as these are GET or bodyless POSTs.

### Connector Transaction ID Source (`RouterData.reference_id`) (Advanced Learnings from Real Codebase - Airwallex Example)
- **Observation**: For flows like Authorize, Capture, Void, CompleteAuthorize that operate on an existing payment intent (created during PreProcessing), the `connector_transaction_id` (which is the Airwallex `payment_intent_id`) is stored in `RouterData.reference_id`.
- **Lesson**: In `get_url` methods for these flows, use `req.reference_id.clone().ok_or(...)` to get the payment intent ID.

### Flow: `Void` (PaymentsCancel) (Bambora Connector (bambora.rs) Implementation Comparison)
#### My Implementation:
Not implemented (empty struct `impl ConnectorIntegration<Void, ...> for Bambora {}`).

#### Reference Implementation:
Fully implemented.
*   `get_url()`: `base_url/v1/payments/{connector_payment_id}/void`.
*   `get_request_body()`: Uses `BamboraVoidRequest { amount: f64 }` from transformers.
*   `handle_response()`: Parses into `bambora::BamboraPaymentsResponse`.

#### Lessons Learned:
*   Bambora has a dedicated `/void` endpoint.
*   Void request also takes an `amount`.

### 7. Testing All Flows (Common Pitfalls and Lessons Learned)
- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

## From grace/guides/types/types.md

### Cancellation (Void) Transformation (Adyen Connector Type Mappings)
*   **Request**: `PaymentsCancelRouterData` is transformed into `AdyenCancelRequest`.
    *   Includes `merchant_account` and `reference` (Hyperswitch's `connector_request_reference_id`).
*   **Response**: `AdyenCancelResponse` is mapped to `PaymentsCancelRouterData`.
    *   Status is typically set to `storage_enums::AttemptStatus::Pending`.

### Void (Cancel) Transformation (Airwallex Connector Type Mappings)
*   `PaymentsCancelRouterData` is transformed into `AirwallexPaymentsCancelRequest`.
    *   `request_id`: A new UUID.
    *   `cancellation_reason: Option<String>`.

### Void (Cancel) Transformation (Authorize.Net Connector Type Mappings)
*   `PaymentsCancelRouterData` is transformed into `CancelOrCaptureTransactionRequest`.
*   `transaction_type` is `voidTransaction`.
*   `ref_trans_id` is the original connector transaction ID.
*   Response (`AuthorizedotnetVoidResponse`) status (`AuthorizedotnetVoidStatus`) is mapped to `enums::AttemptStatus` (`Voided`, `VoidFailed`, `VoidInitiated`).

### Void (Cancel) Transformation (Bank of America Connector Type Mappings)
*   `PaymentsCancelRouterData` is transformed into `BankOfAmericaVoidRequest`.
*   Includes `client_reference_information` and `reversal_information` (with amount, currency, and reason).
*   Response is `BankOfAmericaPaymentsResponse`, status mapped accordingly (e.g., `Voided`, `Failure`).

## From grace/guides/integrations/integrations.md

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Structuring a New Hyperswitch Connector)
This is where the logic for each specific payment/refund flow (Authorize, Capture, PSync, Refund Execute, Refund Sync, etc.) resides. For each flow:
    *   `get_headers()`: Defines request headers.
    *   `get_content_type()`: Defines request content type.
    *   `get_url()`: Constructs the specific API endpoint URL for the flow.
    *   `get_request_body()`: Transforms Hyperswitch's `RouterData` into the connector's specific request struct (from `transformers.rs`).
    *   `build_request()`: Assembles the `services::Request` object.
    *   `handle_response()`: Parses the connector's HTTP response into its specific response struct and transforms it back into Hyperswitch's `RouterData`.
    *   `get_error_response()`: Handles error responses, usually delegating to `build_error_response`.
(This general structure applies to the Void/Cancel flow as well)

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
(This general structure applies to the Void/Cancel flow as well)

### Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
(This general structure applies to the Void/Cancel flow as well)

### Payments (Void/Cancel) (Connector Deep Dive: Adyen)
    *   Uses `/payments/{payment_id}/cancels` endpoint.
    *   `AdyenCancelRequest` includes merchant account and reference.
    *   Response `AdyenCancelResponse` indicates "received" or "processing".
