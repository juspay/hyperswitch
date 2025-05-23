# Refund Flow Information

This document consolidates information related to the Refund flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Trait Usage and Imports (DLocal Integration Learnings - Session 1)
- **`RefundsRequestData` Trait:** Provides `get_connector_refund_id()`. Import `use crate::utils::RefundsRequestData;`.

### Dlocal Specifics (DLocal Integration Learnings - Session 1)
- **Currency Unit:**
    - **Decision:** Align with `real-codebase` and use `api::CurrencyUnit::Minor`. This implies that `DlocalPaymentsRequest` and `DlocalRefundRequest` in `transformers.rs` should expect `i64` (minor unit) amounts, and the `DlocalRouterData` should also handle `i64`.

### Amount Handling (Revisited) (DLocal Integration Learnings - Session 2)
    - **Alignment Decision:** Based on `real-codebase` and `get_currency_unit()` being `Minor`, the `DlocalPaymentsRequest` and `DlocalRefundRequest` in `transformers.rs` should use `i64` for amount. The `DlocalRouterData` struct should also be adapted to hold `i64`. This simplifies amount handling and removes the need for `to_major_unit_as_f64` in `dlocal.rs`.

### Trait Imports for Methods (DLocal Integration Learnings - Session 3)
- **`PaymentsAuthorizeRequestData` (and similar for other flows):** Methods like `get_email_for_connector()` or `get_webhook_url()` on `PaymentsAuthorizeData` (or `RefundsData`, etc.) are often part of specific request data traits (e.g., `crate::utils::PaymentsAuthorizeRequestData`). These need to be in scope.

### Request Data Structs (DLocal Integration Learnings - Session 4)
- **`PaymentsAuthorizeData`, `PaymentsSyncData`, `PaymentsCaptureData`, `RefundsData`:** These concrete request data structs (used as the `T` generic in `RouterData<F, T, Op>`) are located in `hyperswitch_domain_models::router_request_types`. They should be imported from there when defining the `Payable` and `Refundable` trait implementations.

### Trait Paths for Flow Markers (DLocal Integration Learnings - Session 5)
- **`RefundableFlow` Trait:** This trait (or similar flow-specific marker traits if they exist) is not found in `hyperswitch_interfaces::types`. The generic parameter `F` in `RefundsRouterData<F>` is typically one of the specific flow structs like `Execute` or `RSync` from `hyperswitch_domain_models::router_flow_types::refunds`. Adding a separate trait bound like `F: RefundableFlow` is often unnecessary and can lead to "trait not found" errors if the trait doesn't exist or isn't in scope. The specific type of `F` already constrains the `RefundsRouterData`.
    - **Solution:** Remove the `where F: hyperswitch_connector_types::RefundableFlow` bound from `TryFrom<&DlocalRouterData<&RefundsRouterData<F>>> for DlocalRefundRequest`.

### Topic: Import Paths and Type Aliases (Airwallex Implementation Comparison)
(Relevant parts mentioning RefundsResponseRouterData)
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
\`\`\`rust
use crate::{ // crate::types is hyperswitch_interfaces::types
    types::{
        RefundsResponseRouterData, ResponseRouterData, 
    },
};
\`\`\`

### Flow: `RefundExecute` (Bambora Connector (bambora.rs) Implementation Comparison)
#### My Implementation `get_url()`:
\`\`\`rust
Ok(format!(
    "{}/payments/{}/returns",
    self.base_url(connectors),
    req.request.connector_transaction_id
))
\`\`\`
#### Reference Implementation `get_url()`:
\`\`\`rust
Ok(format!(
    "{}/v1/payments/{}/returns", // Includes /v1/
    self.base_url(connectors),
    connector_payment_id,
))
\`\`\`
#### Differences:
1.  **URL Path**: Reference includes `/v1/`.

#### Lessons Learned:
*   Consistency with `/v1/` prefix.

### 7. Testing All Flows (Common Pitfalls and Lessons Learned)
- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

### Type Resolution and Imports (Spreedly Integration Learnings)
- **`crate::types` vs. `hyperswitch_domain_models::types`**:
    - Specific `RouterData` aliases (e.g., `PaymentsAuthorizeRouterData`, `RefundsRouterData`) are usually defined in `hyperswitch_domain_models::types`.

## From grace/guides/types/types.md

### Refund Transformation (ACI Connector Type Mappings)
*   **Request**: Hyperswitch's `RefundsRouterData` is transformed into an `AciRefundRequest`, which includes amount, currency, entity ID, and sets `payment_type` to `AciPaymentType::Refund`.
*   **Response**: ACI's `AciRefundResponse` is mapped back to `RefundsRouterData`.
    *   The `ResultCode.code` is parsed into `AciRefundStatus` (`Succeeded`, `Failed`, `Pending`).
    *   `AciRefundStatus` is then mapped to Hyperswitch's `common_enums::RefundStatus` (`Success`, `Failure`, `Pending`).
    *   The `id` from `AciRefundResponse` becomes the `connector_refund_id`.

### Refund Transformation (Adyen Connector Type Mappings)
*   **Request**: `AdyenRouterData<&RefundsRouterData<F>>` is transformed into `AdyenRefundRequest`.
    *   Includes `merchant_account`, `amount`, `merchant_refund_reason`, and `reference` (Hyperswitch's `refund_id`).
    *   Supports `splits` for Adyen's split refund feature.
*   **Response**: `AdyenRefundResponse` (containing `psp_reference`, `status`) is mapped to `RefundsRouterData`.
    *   Adyen's refund `status` (typically "received") is mapped to `storage_enums::RefundStatus::Pending`, as the final outcome is usually via webhook.

### Refund Transformation (Airwallex Connector Type Mappings)
*   **Request**: `AirwallexRouterData<&types::RefundsRouterData<F>>` is transformed into `AirwallexRefundRequest`.
    *   `request_id`: A new UUID.
    *   `amount: Option<String>`: Refund amount as a string.
    *   `reason: Option<String>`.
    *   `payment_intent_id: String`: The connector transaction ID of the original payment.
*   **Response**: `RefundResponse` from Airwallex is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   `status: RefundStatus` (Airwallex's enum: `Succeeded`, `Failed`, `Received`, `Accepted`) is mapped to Hyperswitch's `enums::RefundStatus` (`Success`, `Failure`, `Pending`).

### Refund Transformation (Card Payments) (Amazonpay Connector Type Mappings)
*   **Request**: `AmazonpayRouterData<&RefundsRouterData<F>>` is transformed into `AmazonpayRefundRequest`.
    *   `amount: StringMinorUnit`: The amount to be refunded.
*   **Response**: `RefundResponse` from Amazonpay is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   `status: RefundStatus` (Amazonpay's enum: `Succeeded`, `Failed`, `Processing`) is mapped to Hyperswitch's `enums::RefundStatus` (`Success`, `Failure`, `Pending`).

### Refund Transformation (Authorize.Net Connector Type Mappings)
*   **Request**: `RefundsRouterData` is transformed into `CreateRefundRequest` wrapping `AuthorizedotnetRefundRequest`.
    *   `transaction_type` is `refundTransaction`.
    *   `payment`: Contains card details (masked number, dummy expiry) retrieved from `connector_metadata` of the original charge.
    *   `refTransId`: Original connector transaction ID.
*   **Response**: `AuthorizedotnetRefundResponse` is mapped to `RefundsRouterData`.
    *   `transaction_response.response_code` (`AuthorizedotnetRefundStatus`) is mapped to `enums::RefundStatus` (`Success`, `Failure`, `Pending`).

### Refund Transformation (Bambora APAC Connector Type Mappings)
*   **Request**: SOAP request for `<dts:SubmitSingleRefund>`.
    *   Contains `CustRef` (refund ID), `Receipt` (original capture/payment transaction ID), `Amount`, and `Security`.
*   **Response**: `BamboraapacRefundsResponse` structure is similar to payment response.
    *   Status: `Success` if `ResponseCode == 0`, `Failure` if `ResponseCode == 1`, else `Pending`.

### Refund Transformation (Bank of America Connector Type Mappings)
*   **Request**: `RefundsRouterData` is transformed into `BankOfAmericaRefundRequest`.
    *   Includes `order_information` (amount, currency) and `client_reference_information` (refund ID).
*   **Response**: `BankOfAmericaRefundResponse` is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   `status: BankofamericaRefundStatus` is mapped to `enums::RefundStatus` (`Success`, `Failure`, `Pending`).
        *   `TwoZeroOne` status with `PROCESSOR_DECLINED` reason maps to `Failure`.

## From grace/guides/integrations/integrations.md

### Marker Trait Implementations (Structuring a New Hyperswitch Connector)
Empty implementations indicating supported Hyperswitch flows (e.g., `api::Payment`, `api::PaymentAuthorize`, `api::RefundExecute`).

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Structuring a New Hyperswitch Connector)
This is where the logic for each specific payment/refund flow (Authorize, Capture, PSync, Refund Execute, Refund Sync, etc.) resides.
(This general structure applies to the Refund flow as well)

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
(This general structure applies to the Refund flow as well)

### Detailed Request/Response Transformation: PayPal Example (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   **Refund Transformation (`RouterData::try_from(ResponseRouterData<Execute, RefundResponse, RefundsData, RefundsResponseData>)`)**:
        *   `PaypalRefundRequest` contains `amount` (major unit).
        *   `RefundResponse` contains `id` (refund ID) and `status` (`RefundStatus` enum: `COMPLETED`, `PENDING`, `FAILED`).
        *   This maps to `RefundsResponseData` with `connector_refund_id` and `refund_status`.

### Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
(This general structure applies to the Refund flow as well)

### `AdyenRefundRequest` (Connector Deep Dive: Adyen)
    *   Fields: `merchant_account`, `amount`, `merchant_refund_reason`, `reference` (Hyperswitch's `refund_id`), `splits`, `store`.
    *   `TryFrom<&AdyenRouterData<&RefundsRouterData<Execute>>>`: Populates fields.

### `AdyenRefundResponse` (Connector Deep Dive: Adyen)
    *   Fields: `psp_reference` (refund ID), `status` (string, usually "received").
    *   `TryFrom<RefundsResponseRouterData<F, AdyenRefundResponse>> for RefundsRouterData<F>`: Maps to `RefundStatus::Pending`.

### Refunds (Execute) (Connector Deep Dive: Adyen)
    *   Uses `/payments/{payment_id}/refunds` endpoint.
    *   `AdyenRefundRequest` includes amount, merchant account, reason.
    *   Response `AdyenRefundResponse` indicates "received".
