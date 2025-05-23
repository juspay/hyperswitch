# PSync (Payment Sync) Flow Information

This document consolidates information related to the PSync (Payment Sync) flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Type Alias Generics (DLocal Integration Learnings - Session 3)
- **`PaymentsResponseRouterData<R>`:** This alias in `crate::types` is defined with a single generic parameter `R`. If used with more than one (e.g., `PaymentsResponseRouterData<A, B>`), it causes an E0107 error.
    - **Solution:** Either correct the usage to provide only one generic argument if that's the intent, or if the alias is not flexible enough, use the full underlying type `ResponseRouterData<F, Resp, ReqBody, Output>` directly in the function signatures or `TryFrom` implementations. For the Dlocal `PaymentsSyncRouterData` `TryFrom` implementation, using the full `ResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse, PaymentsSyncData, PaymentsResponseData>` was the path taken.

### Field Access in `TryFrom` for `RouterData` Wrappers (DLocal Integration Learnings - Session 3)
- **Spreading `item.data`:** The `..item.data` spread is correct if `item.data` is of the same type as the `RouterData` struct being constructed (e.g., `RouterData<F, T, Output>`). If the `TryFrom` is for a more specific type alias that *is* `item.data` (e.g., `TryFrom<ResponseRouterData<...>> for PaymentsSyncRouterData`), then `..item.data` is correct. The E0308 mismatched types error for `..item.data` in the `PaymentsSyncRouterData` `TryFrom` was because the `TryFrom` should have been for the outer `RouterData<PaymentsSyncRouterData, _, _>` type.

### Module Paths for Flow Types and Traits (DLocal Integration Learnings - Session 4)
- **Flow Types (Authorize, PSync, etc.):** These are located in `hyperswitch_interfaces::api`. So, use `api::Authorize`, `api::PSync`, etc., after `use hyperswitch_interfaces::api;`. The alias `hyperswitch_types` (for `hyperswitch_domain_models::types`) does not contain an `api` submodule.

### Request Data Structs (DLocal Integration Learnings - Session 4)
- **`PaymentsAuthorizeData`, `PaymentsSyncData`, `PaymentsCaptureData`, `RefundsData`:** These concrete request data structs (used as the `T` generic in `RouterData<F, T, Op>`) are located in `hyperswitch_domain_models::router_request_types`. They should be imported from there when defining the `Payable` and `Refundable` trait implementations.

### Other Minor Alignments (DLocal Integration Learnings - Session 8)
- **`get_request_body` for PSync/Void/RSync:** Ensured these return `Ok(RequestContent::NoContent)` as these are GET or bodyless POSTs.
- **`get_content_type` for PSync/RSync:** Returns `""` as these are GET requests.

### Topic: Import Paths and Type Aliases (Airwallex Implementation Comparison)
(Relevant parts mentioning PaymentsSyncRouterData)
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
\`\`\`rust
// hyperswitch_types is an alias for hyperswitch_domain_models::types
use hyperswitch_types::{
    PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsCancelRouterData,
    PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
    AccessTokenResponseRouterData, 
};
\`\`\`

### Connector Transaction ID Source for PSync (`PaymentsSyncData.connector_transaction_id`) (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Observation**: For `PSync`, the `PaymentsSyncData.connector_transaction_id` field holds an `Option<ConnectorTransactionId>`. The actual string ID is obtained by calling the inherent method `get_connector_transaction_id()` on the `ConnectorTransactionId` struct.
*   **Lesson**: The trait `PaymentsSyncRequestData::get_connector_transaction_id()` (implemented for `PaymentsSyncData`) returns `Result<String, _>`. Use `req.request.get_connector_transaction_id()?`.

### 7. Testing All Flows (Common Pitfalls and Lessons Learned)
- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

## From grace/guides/types/types.md

### Sync (PSync / RSync) Transformation (Authorize.Net Connector Type Mappings)
*   `PaymentsSyncRouterData` or `RefundsRouterData<RSync>` is transformed into `AuthorizedotnetCreateSyncRequest`.
    *   This request calls the `getTransactionDetailsRequest` API.
*   Response (`AuthorizedotnetSyncResponse` for payments, `AuthorizedotnetRSyncResponse` for refunds) contains `transaction_status` which is mapped to Hyperswitch's `AttemptStatus` or `RefundStatus`.

### Sync (PSync / RSync) Transformation (Bambora APAC Connector Type Mappings)
*   SOAP request for `<dts:QueryTransaction>`.
*   Criteria include `AccountNumber`, date range, and `Receipt` (for PSync) or `CustRef` (for RSync - refund ID).
*   Response (`BamboraapacSyncResponse`) structure is similar to payment response.
*   Status mapping for PSync is similar to authorize response.
*   Status mapping for RSync is similar to refund response.

### Sync (PSync / RSync) Transformation (Bank of America Connector Type Mappings)
*   `PaymentsSyncRouterData` or `RefundsRouterData<RSync>` is transformed to request transaction details.
*   Response (`BankOfAmericaTransactionResponse` for PSync, `BankOfAmericaRsyncResponse` for RSync) contains `application_information.status` which is mapped to Hyperswitch's `AttemptStatus` or `RefundStatus`.

## From grace/guides/integrations/integrations.md

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Structuring a New Hyperswitch Connector)
This is where the logic for each specific payment/refund flow (Authorize, Capture, PSync, Refund Execute, Refund Sync, etc.) resides.
(This general structure applies to the PSync flow as well)

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
(This general structure applies to the PSync flow as well)

### Detailed Request/Response Transformation: PayPal Example (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   **Response Transformation (`RouterData::try_from(ResponseRouterData<F, PaypalAuthResponse, T, PaymentsResponseData>)`)**:
        *   **`PaypalOrdersResponse`**:
            *   `connector_metadata`: A `PaypalMeta` struct is created.
                *   `psync_flow`: Set to the `intent` from the response.
        *   **`PaypalRedirectResponse`**:
            *   `connector_metadata`: `PaypalMeta` with `psync_flow` set to the response `intent`.
        *   **`PaypalThreeDsResponse`**:
            *   `connector_metadata`: `PaypalMeta` with `psync_flow` set to `Authenticate`.

### Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
(This general structure applies to the PSync flow as well)

### Payments (PSync) (Connector Deep Dive: Adyen)
    *   Used for redirect flows. `encoded_data` from the redirect is parsed into `AdyenRedirectRequestTypes` (AdyenRedirection, AdyenThreeDS, AdyenRefusal) and sent to the `/payments/details` endpoint.
    *   If `encoded_data` is not present (non-redirect flow), PSync is effectively skipped, relying on webhooks.
