# RSync (Refund Sync) Flow Information

This document consolidates information related to the RSync (Refund Sync) flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Trait Paths for Flow Markers (DLocal Integration Learnings - Session 5)
- **`RefundableFlow` Trait:** The generic parameter `F` in `RefundsRouterData<F>` is typically one of the specific flow structs like `Execute` or `RSync` from `hyperswitch_domain_models::router_flow_types::refunds`.

### Other Minor Alignments (DLocal Integration Learnings - Session 8)
- **RSync URL:** Aligned the RSync URL to `{}refunds/{}` as per `real-codebase` (DLocal docs suggest `{}refunds/{}/status`).
- **`get_request_body` for PSync/Void/RSync:** Ensured these return `Ok(RequestContent::NoContent)` as these are GET or bodyless POSTs.
- **`get_content_type` for PSync/RSync:** Returns `""` as these are GET requests.

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
(This general structure applies to the RSync flow as well)

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
(This general structure applies to the RSync flow as well)

### Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
(This general structure applies to the RSync flow as well)
