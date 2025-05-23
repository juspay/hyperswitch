# Capture Flow Information

This document consolidates information related to the Capture flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Request Data Structs (DLocal Integration Learnings - Session 4)
- **`PaymentsAuthorizeData`, `PaymentsSyncData`, `PaymentsCaptureData`, `RefundsData`:** These concrete request data structs (used as the `T` generic in `RouterData<F, T, Op>`) are located in `hyperswitch_domain_models::router_request_types`. They should be imported from there when defining the `Payable` and `Refundable` trait implementations.
- **Privacy of Type Aliases in `hyperswitch_domain_models::types`:** Some type aliases like `hyperswitch_domain_models::types::PaymentsCaptureData` might point to structs that are not publicly re-exported in a way that makes them directly usable as a generic argument in another crate. It's safer to use the direct path from `router_request_types`.

### Type Annotations for Trait Methods (E0283) (DLocal Integration Learnings - Session 6)
- **Solution for Dlocal:**
    - The `build_headers` in `ConnectorCommonExt` was calling `self.get_content_type()`. For Dlocal, this was problematic.
    - The fix was to ensure that `build_request` in each specific flow (Authorize, Capture, etc.) directly uses `self.common_get_content_type()` when constructing the `Content-Type` header, as this method is non-generic and directly implemented by `Dlocal`.

### Connector Transaction ID Source (`RouterData.reference_id`) (Advanced Learnings from Real Codebase - Airwallex Example)
- **Observation**: For flows like Authorize, Capture, Void, CompleteAuthorize that operate on an existing payment intent (created during PreProcessing), the `connector_transaction_id` (which is the Airwallex `payment_intent_id`) is stored in `RouterData.reference_id`.
- **Lesson**: In `get_url` methods for these flows, use `req.reference_id.clone().ok_or(...)` to get the payment intent ID.

### Amount in `AirwallexPaymentsCaptureRequest` (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Observation**: `PaymentsCaptureData.amount_to_capture` is `i64`. The `AirwallexPaymentsCaptureRequest.amount` field in `real-codebase` is `Option<String>`, populated by converting `amount_to_capture` using `utils::to_currency_base_unit`.
*   **Lesson (Initial)**: My `AirwallexPaymentsCaptureRequest.amount` was `Option<StringMinorUnit>`. The conversion `Some(StringMinorUnit::from(item.request.amount_to_capture))` was thought to be correct.
*   **Correction (21/05/2025)**: `StringMinorUnit::from(i64)` is incorrect (type mismatch). `StringMinorUnit::new(String)` is private. The field `AirwallexPaymentsCaptureRequest.amount` should be `Option<String>`. The conversion should use `crate::utils::to_currency_base_unit(item.request.amount_to_capture, item.request.currency)` to get a `String` representing minor units.

### Flow: `Capture` (Bambora Connector (bambora.rs) Implementation Comparison)
#### My Implementation `get_url()`:
\`\`\`rust
Ok(format!(
    "{}/payments/{}/complete", // Endpoint from my initial doc reading
    self.base_url(connectors),
    connector_payment_id
))
\`\`\`
#### Reference Implementation `get_url()`:
\`\`\`rust
Ok(format!(
    "{}/v1/payments/{}/completions", // /v1/ and /completions (plural)
    self.base_url(connectors),
    req.request.connector_transaction_id,
))
\`\`\`
#### My Implementation `get_request_body()`:
Uses `BamboraCaptureRequest { amount: f64 }`.
#### Reference Implementation `get_request_body()`:
Uses `BamboraPaymentsCaptureRequest { amount: f64, payment_method: PaymentMethod::Card }`.

#### Differences:
1.  **URL Path**: Reference uses `/v1/` and `/completions` (plural). Mine used `/complete`.
2.  **Request Body**: Reference `BamboraPaymentsCaptureRequest` also includes `payment_method`.

#### Lessons Learned:
*   API endpoint paths need to be exact (`/v1/` and pluralization).
*   Capture request might also need `payment_method` field.

### 7. Testing All Flows (Common Pitfalls and Lessons Learned)
- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

## From grace/guides/types/types.md

### Core Request Transformation (Payments) (Adyen Connector Type Mappings)
*   **`AdyenPaymentRequest`**:
    *   `additional_data: Option<AdditionalData>` (for manual capture, 3DS, recurring details, risk data).
*   **Status Mapping**:
    *   The mapping depends on whether manual capture is enabled (`is_manual_capture`). For example, `AdyenStatus::Authorised` maps to `AttemptStatus::Authorized` if manual capture, else `AttemptStatus::Charged`.

### Capture Transformation (Adyen Connector Type Mappings)
*   **Request**: `AdyenRouterData<&PaymentsCaptureRouterData>` is transformed into `AdyenCaptureRequest`.
    *   Includes `merchant_account`, `amount`, and `reference`. The `reference` is the `capture_id` for multiple captures or `attempt_id` for single captures.
*   **Response**: `AdyenCaptureResponse` is mapped to `PaymentsCaptureRouterData`.
    *   Status is typically set to `storage_enums::AttemptStatus::Pending`.
    *   `psp_reference` (for multiple captures) or `payment_psp_reference` (for single capture) becomes the `resource_id`.

### Core Request Transformation (Payments) (Airwallex Connector Type Mappings)
2.  **Payment Confirmation**: `PaymentsAuthorizeRouterData` (often enriched with the intent details from preprocessing) is transformed into `AirwallexPaymentsRequest`.
    *   `payment_method_options: Option<AirwallexPaymentOptions>`:
        *   For cards, this is `AirwallexPaymentOptions::Card(AirwallexCardPaymentOptions)`.
        *   `AirwallexCardPaymentOptions` contains `auto_capture: bool`, determined by Hyperswitch's `capture_method`.

### Capture Transformation (Airwallex Connector Type Mappings)
*   `PaymentsCaptureRouterData` is transformed into `AirwallexPaymentsCaptureRequest`.
    *   `request_id`: A new UUID.
    *   `amount: Option<String>`: Amount to capture, converted to base unit string.

### Core Request Transformation (Card Payments) (Amazonpay Connector Type Mappings)
*   **`AmazonpayPaymentsRequest`**:
    *   `card: AmazonpayCard`:
        *   `complete: bool`: A flag indicating if the transaction is for auto-capture. This is determined by `item.router_data.request.is_auto_capture()?`.
*   **Transformation Logic**:
    *   The `complete` field in `AmazonpayCard` is set based on whether the Hyperswitch request indicates auto-capture.

### Core Request Transformation (Payments) (Authorize.Net Connector Type Mappings)
*   **`TransactionRequest`**:
    *   `transaction_type: TransactionType`: Determined by Hyperswitch's `capture_method`.
        *   `authCaptureTransaction` (Payment) for auto-capture.
        *   `authOnlyTransaction` (Authorization) for manual capture.

### Capture Transformation (Authorize.Net Connector Type Mappings)
*   `PaymentsCaptureRouterData` is transformed into `CancelOrCaptureTransactionRequest` wrapping `AuthorizedotnetPaymentCancelOrCaptureRequest`.
*   `transaction_type` is `priorAuthCaptureTransaction`.
*   `ref_trans_id` is the original connector transaction ID.
*   Response is similar to payment response, status mapped accordingly.

### Core Request Transformation (Card Payments) (Bambora Connector Type Mappings)
*   **`BamboraPaymentsRequest`**:
    *   `card: BamboraCard`:
        *   `complete: bool`: Set based on Hyperswitch's `capture_method` (true for auto-capture, false for manual).

### Core Request Transformation (Card Payments & Mandates) (Bambora APAC Connector Type Mappings)
*   **`<Transaction>` (for Payments)**:
    *   `TrnType`: Transaction type (integer: `1` for Sale/Auth+Capture, `2` for Auth only). Determined by Hyperswitch's `capture_method`.

### Capture Transformation (Bambora APAC Connector Type Mappings)
*   SOAP request for `<dts:SubmitSingleCapture>`.
*   Contains `Receipt` (original auth transaction ID), `Amount`, and `Security` details.
*   Response (`BamboraapacCaptureResponse`) structure is similar to payment response.
*   Status: `Charged` if `ResponseCode == 0`, else `Failure`.
*   `authorize_id` (original auth receipt) is stored in `connector_metadata`.

### Core Request Transformation (Payments & Mandates) (Bank of America Connector Type Mappings)
*   **`BankOfAmericaPaymentsRequest`**:
    *   `processing_information: ProcessingInformation`:
        *   `capture: Option<bool>`: True for auto-capture, false for manual.

### Capture Transformation (Bank of America Connector Type Mappings)
*   `PaymentsCaptureRouterData` is transformed into `BankOfAmericaCaptureRequest`.
*   Includes `order_information` (with amount and currency) and `client_reference_information`.
*   Response is `BankOfAmericaPaymentsResponse`, status mapped accordingly (typically `Charged` or `Failure`).

## From grace/guides/integrations/integrations.md

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Structuring a New Hyperswitch Connector)
This is where the logic for each specific payment/refund flow (Authorize, Capture, PSync, Refund Execute, Refund Sync, etc.) resides.
(This general structure applies to the Capture flow as well)

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
(This general structure applies to the Capture flow as well)

### Detailed Request/Response Transformation: PayPal Example (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   **Request Transformation (`PaypalPaymentsRequest::try_from(&PaypalRouterData<&PaymentsAuthorizeRouterData>)`)**:
        *   **Intent**: Determined by `is_auto_capture()`: `PaypalPaymentIntent::Capture` or `PaypalPaymentIntent::Authorize`.
    *   **Response Transformation (`RouterData::try_from(ResponseRouterData<F, PaypalAuthResponse, T, PaymentsResponseData>)`)**:
        *   **`PaypalOrdersResponse`**:
            *   `connector_metadata`: A `PaypalMeta` struct is created.
                *   `authorize_id` or `capture_id`: Extracted from the first `purchase_units.payments.authorizations[0].id` or `captures[0].id`.

### Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
(This general structure applies to the Capture flow as well)

### `AdyenCaptureRequest` (Connector Deep Dive: Adyen)
    *   Fields: `merchant_account`, `amount`, `reference`.
    *   `TryFrom<&AdyenRouterData<&PaymentsCaptureRouterData>>`: Populates amount and merchant account. `reference` is either `capture_id` (for multiple captures) or `connector_request_reference_id` (for single capture).

### `AdyenCaptureResponse` (Connector Deep Dive: Adyen)
    *   Fields: `psp_reference` (capture ID), `payment_psp_reference` (original payment ID), `status` (string, usually "received"), `amount`.
    *   `TryFrom<PaymentsCaptureResponseRouterData<AdyenCaptureResponse>> for PaymentsCaptureRouterData`: Maps to `AttemptStatus::Pending` as Adyen capture is asynchronous. `resource_id` is the `psp_reference`.

### Payments (Capture) (Connector Deep Dive: Adyen)
    *   Uses `/payments/{payment_id}/captures` endpoint.
    *   `AdyenCaptureRequest` includes amount and merchant account.
    *   Response `AdyenCaptureResponse` usually indicates "received", final status via webhook.
