# Authorization Flow Information

This document consolidates information related to the Authorization flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Trait Imports for Methods (DLocal Integration Learnings - Session 3)
- **`PaymentsAuthorizeRequestData` (and similar for other flows):** Methods like `get_email_for_connector()` or `get_webhook_url()` on `PaymentsAuthorizeData` (or `RefundsData`, etc.) are often part of specific request data traits (e.g., `crate::utils::PaymentsAuthorizeRequestData`). These need to be in scope.
    - **Note on `get_webhook_url()`:** This method might be on `item.router_data.request` (the specific `PaymentsAuthorizeData` etc.) rather than directly on `item.router_data` (the `RouterData` wrapper).

### `RouterData::try_from` Resolution (DLocal Integration Learnings - Session 3)
The call `RouterData::try_from(response_router_data_instance)` relies on a specific `impl TryFrom<SpecificResponseRouterData> for SpecificRouterData` being available.
- If `handle_response` returns, for example, `CustomResult<PaymentsAuthorizeRouterData, ...>`, then the `TryFrom` implementation in `transformers.rs` must be `impl TryFrom<ResponseRouterData<Authorize, DlocalPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData`.
- The compiler errors (E0277, E0308) related to this often mean the `TryFrom` signature in `transformers.rs` does not exactly match what `dlocal.rs`'s `handle_response` expects, or the `RouterData::try_from` call is being made on the generic `RouterData` type instead of the specific type alias for which the `TryFrom` is implemented.
- **Resolution:** Ensure `handle_response` returns the specific `RouterData` alias (e.g., `PaymentsAuthorizeRouterData`) and that the `TryFrom` in `transformers.rs` is defined for this specific alias, converting from the appropriate `ResponseRouterData`. My previous fix correctly changed the `TryFrom` target to the generic `RouterData<F, T, PaymentsResponseData>`, which should work if `F` and `T` are correctly inferred by the compiler from the `handle_response` signature. The latest errors indicate that the `TryFrom` in `transformers.rs` needs to be for the specific `RouterData` aliases (e.g., `PaymentsAuthorizeRouterData`) rather than the generic `RouterData<F, T, OpResponse>`.

### Module Paths for Flow Types and Traits (DLocal Integration Learnings - Session 4)
- **Flow Types (Authorize, PSync, etc.):** These are located in `hyperswitch_interfaces::api`. So, use `api::Authorize`, `api::PSync`, etc., after `use hyperswitch_interfaces::api;`. The alias `hyperswitch_types` (for `hyperswitch_domain_models::types`) does not contain an `api` submodule.
- **`Capturable`, `Refundable` Traits:** These traits are defined in `hyperswitch_interfaces::types`. Import as `use hyperswitch_interfaces::types as hyperswitch_connector_types;` and use `hyperswitch_connector_types::Capturable`.

### Request Data Structs (DLocal Integration Learnings - Session 4)
- **`PaymentsAuthorizeData`, `PaymentsSyncData`, `PaymentsCaptureData`, `RefundsData`:** These concrete request data structs (used as the `T` generic in `RouterData<F, T, Op>`) are located in `hyperswitch_domain_models::router_request_types`. They should be imported from there when defining the `Payable` and `Refundable` trait implementations.
- **Privacy of Type Aliases in `hyperswitch_domain_models::types`:** Some type aliases like `hyperswitch_domain_models::types::PaymentsCaptureData` might point to structs that are not publicly re-exported in a way that makes them directly usable as a generic argument in another crate. It's safer to use the direct path from `router_request_types`.

### `TryFrom` Implementation for `RouterData` Aliases (DLocal Integration Learnings - Session 4)
The `handle_response` functions in `dlocal.rs` return specific `RouterData` aliases (e.g., `PaymentsAuthorizeRouterData`).
The `TryFrom` implementation in `transformers.rs` should be for these specific aliases, not for the generic `RouterData<F, T, OpResponse>`.
  \`\`\`rust
  // Example for Authorize flow in transformers.rs
  use hyperswitch_domain_models::types::PaymentsAuthorizeRouterData; // This is RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>
  // ...
  impl TryFrom<ResponseRouterData<api::Authorize, DlocalPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData {
      // ...
  }
  \`\`\`
  This ensures that when `PaymentsAuthorizeRouterData::try_from(...)` is called in `dlocal.rs`, it matches this specific implementation.

### HMAC Signature Content (Re-evaluation) (DLocal Integration Learnings - Session 7)
- **Initial Approach:** My `generate_dlocal_hmac_signature` function took `request_body_str: Option<&str>` and `request_path_and_query: &str`, and conditionally used one or the other.
- **Dlocal Documentation & `real-codebase` Discrepancy:**
    - Dlocal's cURL examples show POST signatures based on `X-Login + X-Date + RequestBody`.
    - Dlocal's cURL examples show GET signatures based on `X-Login + X-Date + RequestPathAndQuery`.
    - The `real-codebase`'s generic `ConnectorCommonExt::build_headers` function prepares a signature string `X-Login + X-Date + RequestBodyContentString`. For GET requests where `RequestBodyContentString` would be empty, this results in a signature of `X-Login + X-Date`. This contradicts Dlocal's GET example.
- **Refined Approach for `generate_dlocal_hmac_signature`:**
    - The helper function should take a single `data_for_signature: &str` parameter.
    - The calling code in `build_request` for each flow will be responsible for constructing this `data_for_signature` string correctly:
        - For POST/PUT: `data_for_signature` will be the `request_body_str`.
        - For GET: `data_for_signature` will be the `request_path_and_query`.
    - The signature string itself will then be `X-Login + X-Date + data_for_signature`.
    - This makes the helper simpler and places the logic for what data to sign closer to the context of the HTTP method being used, aligning better with Dlocal's distinct examples.
- **Header Construction Strategy:**
    - Flow-specific `get_headers` methods should return `Ok(Vec::new())`.
    - All headers, including `Content-Type` (for POST/PUT) and all Dlocal-specific auth headers (`X-Login`, `X-Trans-Key`, `X-Date`, `X-Version`, `User-Agent`, `Authorization`), will be constructed within each flow's `build_request` method.
    - The `RequestBuilder::attach_default_headers()` call should be removed from `build_request` if all headers are being manually set.

### Header Generation Strategy (Alignment Implemented) (DLocal Integration Learnings - Session 8)
- **Adopted `real-codebase` Approach:**
    - Centralized all header generation (dynamic Dlocal auth headers: X-Login, X-Trans-Key, X-Date, X-Version; Authorization with HMAC; Content-Type) into `ConnectorCommonExt::build_headers`.
    - `ConnectorCommonExt::build_headers` now retrieves the request body string (handling different `RequestContent` variants), generates timestamps, extracts auth details, constructs the signature payload (`X-Login + X-Date + RequestBodyString`), generates the HMAC, and assembles all headers.
    - For GET requests, `RequestBodyString` is empty, so the signature payload becomes `X-Login + X-Date`. This differs from Dlocal's documentation (which includes path for GET) but aligns with `real-codebase`.
- **Changes Made:**
    - Removed the standalone `generate_dlocal_hmac_signature` helper function.
    - Flow-specific `get_headers` methods now directly call `self.build_headers(req, connectors)`.
    - Flow-specific `build_request` methods now use `RequestBuilder::new().attach_default_headers().headers(self.get_headers(...))...`, relying on the centralized header generation.
    - Ensured `DlocalAuthType` in `transformers.rs` correctly provides `x_login`, `x_trans_key`, and `secret_key`.

### Topic: Import Paths and Type Aliases (Airwallex Implementation Comparison)
(Relevant parts mentioning PaymentsAuthorizeRouterData)
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
\`\`\`rust
// hyperswitch_types is an alias for hyperswitch_domain_models::types
use hyperswitch_types::{
    PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsCancelRouterData,
    PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
    AccessTokenResponseRouterData, 
};
\`\`\`
#### Differences:
3.  **`Payments...RouterData` Wrappers**: These are type aliases defined in `hyperswitch_domain_models::types` (accessible via `hyperswitch_types`). My initial imports in `transformers.rs` from `crate::types` were problematic because `crate::types` (i.e. `hyperswitch_interfaces::types`) might not re-export all of them or might have its own versions.

### Request Data Accessor Traits (Advanced Learnings from Real Codebase - Airwallex Example)
- **Observation**: Methods like `get_amount()`, `get_currency()`, `get_browser_info()`, `get_router_return_url()` are provided by traits defined in `crate::utils` (e.g., `PaymentsAuthorizeRequestData`, `PaymentsPreProcessingRequestData`).
- **Import**: These traits must be imported into the scope where the methods are used (typically `transformers.rs`).
    \`\`\`rust
    // Example in transformers.rs
    use crate::utils::{PaymentsAuthorizeRequestData, PaymentsPreProcessingRequestData /*, etc. */};
    \`\`\`

### Connector Transaction ID Source (`RouterData.reference_id`) (Advanced Learnings from Real Codebase - Airwallex Example)
- **Observation**: For flows like Authorize, Capture, Void, CompleteAuthorize that operate on an existing payment intent (created during PreProcessing), the `connector_transaction_id` (which is the Airwallex `payment_intent_id`) is stored in `RouterData.reference_id`.
- **Lesson**: In `get_url` methods for these flows, use `req.reference_id.clone().ok_or(...)` to get the payment intent ID.

### Flow: `Authorize` (Bambora Connector (bambora.rs) Implementation Comparison)
#### My Implementation `get_url()`:
\`\`\`rust
fn get_url(&self, _req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
    Ok(format!("{}{}", self.base_url(connectors), "/payments"))
}
\`\`\`
#### Reference Implementation `get_url()`:
\`\`\`rust
fn get_url(
    &self,
    _req: &PaymentsAuthorizeRouterData,
    connectors: &Connectors,
) -> CustomResult<String, errors::ConnectorError> {
    Ok(format!("{}{}", self.base_url(connectors), "/v1/payments")) // Includes /v1/
}
\`\`\`
#### My Implementation `get_request_body()`:
\`\`\`rust
let amount_minor_unit = utils::convert_amount( // Uses self.amount_converter
    self.amount_converter,
    req.request.minor_amount,
    req.request.currency,
)?;
let connector_router_data = bambora::BamboraRouterData::try_from((
    amount_minor_unit, // StringMinorUnit
    req.request.currency,
    req,
))?;
// ...
\`\`\`
#### Reference Implementation `get_request_body()`:
\`\`\`rust
let connector_router_data = bambora::BamboraRouterData::try_from((
    &self.get_currency_unit(), // api::CurrencyUnit::Base in reference
    req.request.currency,
    req.request.amount, // This is i64 (minor units from Hyperswitch core)
    req,
))?;
// ...
\`\`\`
#### My Implementation `handle_response()`:
Parses into `bambora::BamboraPaymentsResponse` (my simpler version).
#### Reference Implementation `handle_response()`:
Parses into `bambora::BamboraResponse` (enum for Normal vs 3DS).

#### Differences:
1.  **URL Path**: Reference includes `/v1/` in the path. Mine did not.
2.  **Amount Conversion in `get_request_body`**:
    *   Mine explicitly called `utils::convert_amount` (using `self.amount_converter`) to get `StringMinorUnit` then passed this to `BamboraRouterData::try_from`.
    *   Reference passes `&self.get_currency_unit()` (which it defines as `Base`), `req.request.currency`, and `req.request.amount` (the `i64` minor unit amount from `RouterData`) directly to its `BamboraRouterData::try_from`. The conversion to `f64` major units happens inside the reference `BamboraRouterData::try_from` using `utils::get_amount_as_f64`.
3.  **Response Handling**: Reference handles the `BamboraResponse` enum for 3DS.

#### Lessons Learned:
*   The `/v1/` prefix in API paths is common and should be included.
*   The reference's way of passing raw amount and currency unit to `BamboraRouterData::try_from` is cleaner, assuming `BamboraRouterData`'s transformer correctly handles it. The discrepancy in `get_currency_unit` (Base vs. Minor) is still a point of concern for correct amount conversion if `req.request.amount` is minor.
*   Full 3DS support requires handling the `BamboraResponse` enum.

### 1. Authentication Mechanism Issues (Common Pitfalls and Lessons Learned)
- **Verify the auth type carefully**: Some connectors use `HeaderKey`, others use `BodyKey`, and some require complex multi-step authentication. Read the API docs thoroughly.
- **Check for Base64 encoding requirements**: Many Basic Auth implementations require Base64 encoding of credentials.
- **Use the correct credential handling**: For sensitive data, leverage `PeekInterface` rather than `expose()` when possible.

### 7. Testing All Flows (Common Pitfalls and Lessons Learned)
- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

### Type Resolution and Imports (Spreedly Integration Learnings)
- **`crate::types` vs. `hyperswitch_domain_models::types`**:
    - `crate::types` in the connector crate typically refers to `hyperswitch_interfaces::types`.
    - Specific `RouterData` aliases (e.g., `PaymentsAuthorizeRouterData`, `RefundsRouterData`) are usually defined in `hyperswitch_domain_models::types`. These should be imported from there or via a common alias like `hyperswitch_types`.
    - The generic `ResponseRouterData` struct used as a wrapper for `TryFrom` implementations (to satisfy orphan rules if the target is also foreign) is often `crate::types::ResponseRouterData`.
- **Unused Imports**: Regularly clean up unused imports flagged by the compiler to maintain code clarity. This was done for `RedirectForm`, `MandateReference`, `StringMinorUnit`, `ResultExt`, `Secret`, `FromStr`, `PaymentsAuthorizeRequestData`, and `ResponseRouterData` (from `crate::types` when `types::self` was used) in `spreedly/transformers.rs`. Similarly, `AttemptStatus`, `CaptureMethod`, `PaymentExperience`, `PaymentMethod`, `PaymentMethodType`, and `utils` were removed from `spreedly.rs`.

## From grace/guides/types/types.md

### Core Request Transformation (ACI Connector Type Mappings)
The primary goal is to convert Hyperswitch's `PaymentsAuthorizeRouterData` into an `AciPaymentsRequest` structure suitable for the ACI API. This involves several steps and intermediate types:
*   **`AciRouterData<T>`**: A wrapper that pairs the Hyperswitch router data (e.g., `PaymentsAuthorizeRouterData`) with the `amount` (as `StringMajorUnit`).
*   **`AciAuthType`**: Extracts and stores ACI-specific authentication credentials (`api_key`, `entity_id`) from Hyperswitch's generic `ConnectorAuthType::BodyKey`.
*   **`AciPaymentsRequest`**: The main request payload for ACI. It includes:
    *   `txn_details: TransactionDetails` (amount, currency, ACI entity ID, payment type).
    *   `payment_method: PaymentDetails` (an enum detailing the payment method used).
    *   `instruction: Option<Instruction>` (for mandate/recurring payments).
    *   `shopper_result_url: Option<String>` (return URL for redirection flows).
*   **`PaymentDetails` (enum)**: Represents the specific payment method being used. Key variants include:
    *   `AciCard(Box<CardDetails>)`: For card payments, using `CardDetails` (number, holder, expiry, CVV).
    *   `BankRedirect(Box<BankRedirectionPMData>)`: For bank redirect payments (e.g., EPS, iDEAL, Sofort), using `BankRedirectionPMData` which includes bank-specific details and `PaymentBrand`.
    *   `Wallet(Box<WalletPMData>)`: For wallet payments (e.g., MBWay, AliPay), using `WalletPMData` which includes wallet-specific details and `PaymentBrand`.
    *   `Klarna`: For Klarna Pay Later payments.
    *   `Mandate`: For payments using a previously established mandate.
    *   These variants are populated by `TryFrom` implementations that convert Hyperswitch types like `hyperswitch_domain_models::payment_method_data::Card`, `WalletData`, and `BankRedirectData`.
*   **`PaymentBrand` (enum)**: Standardizes payment brand names for ACI (e.g., `Eps`, `Ideal`, `Sofortueberweisung`, `AliPay`).
*   **`Instruction`, `InstructionMode`, `InstructionType`, `InstructionSource`**: These types are used to convey details for recurring or merchant-initiated transactions based on mandates.
*   **`AciPaymentType` (enum)**: Specifies the ACI transaction type (e.g., `Debit`, `Preauthorization`, `Credit`, `Refund`).

The transformation process involves `TryFrom` implementations on `AciPaymentsRequest` that take `AciRouterData<&PaymentsAuthorizeRouterData>` and the specific `PaymentMethodData` (e.g., `Card`, `WalletData`) as input.

### Core Request Transformation (Payments) (Adyen Connector Type Mappings)
The primary goal is to convert Hyperswitch's `PaymentsAuthorizeRouterData` into an `AdyenPaymentRequest` structure.
*   **`AdyenRouterData<T>`**: A wrapper that pairs Hyperswitch router data (e.g., `PaymentsAuthorizeRouterData`) with the `amount` (as `MinorUnit`).
*   **`AdyenAuthType`**: Extracts Adyen-specific authentication credentials (`api_key`, `merchant_account`, `review_key`) from Hyperswitch's `ConnectorAuthType`.
*   **`AdyenPaymentRequest`**: The main request payload for Adyen. It's a comprehensive structure including:
    *   `amount: Amount` (value and currency).
    *   `merchant_account: Secret<String>`.
    *   `payment_method: PaymentMethod` (an enum detailing the specific Adyen payment method).
    *   `reference: String` (Hyperswitch's `connector_request_reference_id`).
    *   `return_url: String`.
    *   `shopper_interaction: AdyenShopperInteraction` (e.g., `Ecommerce`, `ContAuth`).
    *   `recurring_processing_model: Option<AdyenRecurringModel>` (e.g., `UnscheduledCardOnFile`).
    *   `additional_data: Option<AdditionalData>` (for manual capture, 3DS, recurring details, risk data).
    *   `shopper_reference: Option<String>` (derived from Hyperswitch `customer_id` and `merchant_id`).
    *   `store_payment_method: Option<bool>` (for tokenization).
    *   Shopper details: `shopper_name`, `shopper_ip`, `shopper_locale`, `shopper_email`, `telephone_number`.
    *   Address details: `billing_address`, `delivery_address` (transformed from Hyperswitch address models into Adyen's `Address` struct).
    *   `browser_info: Option<AdyenBrowserInfo>` (for 3DS and certain payment methods).
    *   `line_items: Option<Vec<LineItem>>` (derived from `order_details`).
    *   `channel: Option<Channel>` (e.g., `Web` for certain payment methods).
    *   `mpi_data: Option<AdyenMpiData>` (for 3DS with Paze).
    *   `splits: Option<Vec<AdyenSplitData>>` (for Adyen's split payment feature).
    *   `store: Option<String>` (related to split payments).
    *   `device_fingerprint: Option<Secret<String>>`.
*   **`PaymentMethod` (enum)**: This is a crucial enum that branches into `AdyenPaymentMethod` or `AdyenMandatePaymentMethod`.
    *   **`AdyenPaymentMethod` (enum)**: Represents various payment types Adyen supports. Each variant often wraps a specific struct containing the necessary data for that payment method. Examples:
        *   `AdyenCard(Box<AdyenCard>)`: For card payments. `AdyenCard` includes number, expiry, CVC, holder name, brand, and `network_payment_reference`.
        *   `AdyenKlarna`, `AdyenPaypal`, `AliPay`, `ApplePay(Box<AdyenApplePay>)`, `Gpay(Box<AdyenGPay>)`.
        *   Bank Redirects: `Eps(Box<BankRedirectionWithIssuer>)`, `Ideal`, `OnlineBankingCzechRepublic`, etc. These often take an `issuer` if applicable.
        *   Bank Debits: `AchDirectDebit(Box<AchDirectDebitData>)`, `SepaDirectDebit(Box<SepaDirectDebitData>)`, `BacsDirectDebit(Box<BacsDirectDebitData>)`.
        *   Vouchers: `BoletoBancario`, `Oxxo`, `SevenEleven(Box<JCSVoucherData>)`.
        *   Wallets: `Mbway(Box<MbwayData>)`, `SamsungPay(Box<SamsungPayPmData>)`.
        *   Gift Cards: `AdyenGiftCard(Box<AdyenGiftCardData>)`.
        *   Network Tokens: `NetworkToken(Box<AdyenNetworkTokenData>)`, `AdyenPaze(Box<AdyenPazeData>)`.
    *   **`AdyenMandatePaymentMethod(Box<AdyenMandate>)`**: Used when a payment is made using a stored mandate. `AdyenMandate` includes `payment_type` (e.g., `Scheme`), `stored_payment_method_id`, and `holder_name`.
*   **Transformation Logic**:
    *   The main `TryFrom` for `AdyenPaymentRequest` on `AdyenRouterData<&PaymentsAuthorizeRouterData>` first checks if a `mandate_id` is present.
        *   If yes, it constructs an `AdyenMandatePaymentMethod`.
        *   If no, it branches based on `PaymentMethodData` (Card, Wallet, PayLater, BankRedirect, etc.) and calls specific `TryFrom` implementations to populate the correct `AdyenPaymentMethod` variant.
    *   Helper functions like `get_recurring_processing_model`, `get_browser_info`, `get_additional_data`, `get_address_info`, `get_line_items` are used to extract and transform data.
    *   `AdyenShopperInteraction` is determined based on `request.off_session`.
    *   `RiskData` can be populated from `request.metadata`.

### Core Request Transformation (Payments) (Airwallex Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` is transformed into an `AirwallexPaymentsRequest`. This often involves a two-step process:
1.  **Intent Creation (Preprocessing)**: `PaymentsPreProcessingRouterData` is transformed into `AirwallexIntentRequest`.
    *   `request_id`: A UUID for the request.
    *   `amount`: Converted to currency base unit string.
    *   `currency`.
    *   `merchant_order_id`: Hyperswitch's `connector_request_reference_id`.
    *   `referrer_data`: Static data (`type: "hyperswitch"`, `version: "1.0.0"`) identifying Hyperswitch.
    This step typically returns an `intent_id` and `client_secret` from Airwallex, which are then used in the confirmation step.

2.  **Payment Confirmation**: `PaymentsAuthorizeRouterData` (often enriched with the intent details from preprocessing) is transformed into `AirwallexPaymentsRequest`.
    *   `request_id`: A new UUID for this specific request.
    *   `payment_method: AirwallexPaymentMethod`: Details of the payment method. For cards, this is `AirwallexPaymentMethod::Card(AirwallexCard)`.
        *   **`AirwallexCard`**:
            *   `card: AirwallexCardDetails`: Contains `number` (CardNumber), `expiry_month`, `expiry_year` (4-digit), and `cvc`.
            *   `payment_method_type: AirwallexPaymentType::Card`.
    *   `payment_method_options: Option<AirwallexPaymentOptions>`:
        *   For cards, this is `AirwallexPaymentOptions::Card(AirwallexCardPaymentOptions)`.
        *   `AirwallexCardPaymentOptions` contains `auto_capture: bool`, determined by Hyperswitch's `capture_method`.
    *   `return_url: Option<String>`: Hyperswitch's `complete_authorize_url`.
    *   `device_data: DeviceData`: Contains browser and device information extracted from Hyperswitch's `BrowserInfo`.
        *   `accept_header`, `ip_address`, `language`, `screen_color_depth`, `screen_height`, `screen_width`, `timezone`.
        *   `browser: Browser` (with `java_enabled`, `javascript_enabled`, `user_agent`).
        *   `mobile: Option<Mobile>` (with `device_model`, `os_type`, `os_version` if available).
*   **`AirwallexRouterData<T>`**: A generic wrapper that pairs Hyperswitch router data with the `amount` formatted as a string according to Airwallex's currency unit requirements.

### Core Request Transformation (Card Payments) (Amazonpay Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` is transformed into an `AmazonpayPaymentsRequest` for card payments.
*   **`AmazonpayRouterData<T>`**: A generic wrapper that pairs Hyperswitch router data with the `amount` (as `StringMinorUnit`).
*   **`AmazonpayPaymentsRequest`**: The main request payload for Amazonpay card payments.
    *   `amount: StringMinorUnit`: The transaction amount.
    *   `card: AmazonpayCard`: Details of the card being used.
        *   **`AmazonpayCard`**:
            *   `number: cards::CardNumber`: The card number.
            *   `expiry_month: Secret<String>`: Card expiry month.
            *   `expiry_year: Secret<String>`: Card expiry year.
            *   `cvc: Secret<String>`: Card verification code.
            *   `complete: bool`: A flag indicating if the transaction is for auto-capture. This is determined by `item.router_data.request.is_auto_capture()?`.
*   **Transformation Logic**:
    *   The `TryFrom` implementation for `AmazonpayPaymentsRequest` takes `AmazonpayRouterData<&PaymentsAuthorizeRouterData>`.
    *   It specifically handles `PaymentMethodData::Card`. Other payment methods will result in a `NotImplemented` error.
    *   The `complete` field in `AmazonpayCard` is set based on whether the Hyperswitch request indicates auto-capture.

### Core Request Transformation (Payments) (Authorize.Net Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` is transformed into a `CreateTransactionRequest` which wraps an `AuthorizedotnetPaymentsRequest`.
*   **`AuthorizedotnetRouterData<T>`**: A generic wrapper that pairs Hyperswitch router data with the `amount` (as `f64`).
*   **`CreateTransactionRequest` / `AuthorizedotnetPaymentsRequest`**:
    *   `merchant_authentication: AuthorizedotnetAuthType`.
    *   `ref_id: Option<String>`: Hyperswitch's `connector_request_reference_id` (if length <= 20).
    *   `transaction_request: TransactionRequest`: Contains the core transaction details.
*   **`TransactionRequest`**:
    *   `transaction_type: TransactionType`: Determined by Hyperswitch's `capture_method`.
        *   `authCaptureTransaction` (Payment) for auto-capture.
        *   `authOnlyTransaction` (Authorization) for manual capture.
    *   `amount: f64`.
    *   `currency_code: common_enums::Currency`.
    *   `payment: Option<PaymentDetails>`: Details of the payment instrument.
        *   **`PaymentDetails::CreditCard(CreditCardDetails)`**: For card payments.
            *   `card_number: StrongSecret<String, cards::CardNumberStrategy>`.
            *   `expiration_date: Secret<String>` (format "YYYY-MM").
            *   `card_code: Option<Secret<String>>` (CVV).
        *   **`PaymentDetails::OpaqueData(WalletDetails)`**: For Google Pay and Apple Pay.
            *   `data_descriptor: WalletMethod` (enum: `Googlepay`, `Applepay`).
            *   `data_value: Secret<String>` (the encrypted wallet token).
        *   **`PaymentDetails::PayPal(PayPalDetails)`**: For PayPal.
            *   `success_url: Option<String>`.
            *   `cancel_url: Option<String>`.
    *   `profile: Option<ProfileDetails>`: Used for creating/using customer profiles (mandates).
        *   `CreateProfileDetails { create_profile: bool }` if creating a new profile.
        *   `CustomerProfileDetails { customer_profile_id, payment_profile: { payment_profile_id } }` if using an existing profile.
    *   `order: Order`: Contains `invoice_number` (random string) and `description` (Hyperswitch `connector_request_reference_id`).
    *   `customer: Option<CustomerDetails>`: Contains `id` (Hyperswitch `payment_id` or random string if too long) and `email`.
    *   `bill_to: Option<BillTo>`: Billing address details.
    *   `user_fields: Option<UserFields>`: For metadata, transformed from Hyperswitch metadata.
    *   `processing_options: Option<ProcessingOptions>`: `is_subsequent_auth: bool` (true if using a profile/mandate or network token).
    *   `subsequent_auth_information: Option<SubsequentAuthInformation>`: For network token payments, includes `original_network_trans_id` and `reason`.
    *   `authorization_indicator_type: Option<AuthorizationIndicator>`: Maps Hyperswitch `capture_method` to `AuthorizationType` (`Final` or `Pre`).

### Core Request Transformation (Card Payments) (Bambora Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` is transformed into a `BamboraPaymentsRequest`.
*   **`BamboraRouterData<T>`**: A generic wrapper that pairs Hyperswitch router data with the `amount` (as `f64`).
*   **`BamboraPaymentsRequest`**:
    *   `order_number: String`: Hyperswitch's `connector_request_reference_id`.
    *   `amount: f64`: Transaction amount.
    *   `payment_method: PaymentMethod::Card`: Specifies card payment.
    *   `customer_ip: Option<Secret<String, IpAddress>>`: Included if 3DS is enabled.
    *   `term_url: Option<String>`: Hyperswitch's `complete_authorize_url`, used for 3DS redirection.
    *   `card: BamboraCard`: Details of the card.
        *   **`BamboraCard`**:
            *   `name: Secret<String>`: Cardholder name from billing address.
            *   `number: cards::CardNumber`.
            *   `expiry_month: Secret<String>`.
            *   `expiry_year: Secret<String>` (2-digit format).
            *   `cvd: Secret<String>` (CVV).
            *   `complete: bool`: Set based on Hyperswitch's `capture_method` (true for auto-capture, false for manual).
            *   `three_d_secure: Option<ThreeDSecure>`: Included if 3DS is enabled.
                *   **`ThreeDSecure`**:
                    *   `browser: Option<BamboraBrowserInfo>`: Browser details for 3DS 2.0.
                    *   `enabled: bool` (true).
                    *   `version: Option<i64>` (e.g., 2 for 3DS 2.0).
                    *   `auth_required: Option<bool>` (true).
    *   `billing: AddressData`: Billing address details.
*   **`BamboraBrowserInfo`**: Populated from Hyperswitch's `BrowserInfo` if 3DS is active. Includes fields like `accept_header`, `java_enabled`, `language`, `color_depth`, `screen_height`, `screen_width`, `time_zone`, `user_agent`, `javascript_enabled`.
*   **`AddressData`**: Contains name, address lines, city, province (state), country, postal code, phone, and email.

### Core Request Transformation (Card Payments & Mandates) (Bambora APAC Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` is transformed into a SOAP XML request string.
*   **`BamboraapacRouterData<T>`**: A generic wrapper that pairs Hyperswitch router data with the `amount` (as `MinorUnit`).
*   **SOAP Request Structure**:
    *   The request is wrapped in a `<soapenv:Envelope>` with a `<dts:SubmitSinglePayment>` or `<sipp:TokeniseCreditCard>` operation.
    *   The core transaction details are within a CDATA section as an XML string (`<Transaction>` or `<TokeniseCreditCard>`).
*   **`<Transaction>` (for Payments)**:
    *   `CustRef`: Hyperswitch's `connector_request_reference_id`.
    *   `Amount`: Transaction amount (from `BamboraapacRouterData.amount`).
    *   `TrnType`: Transaction type (integer: `1` for Sale/Auth+Capture, `2` for Auth only). Determined by Hyperswitch's `capture_method`.
    *   `AccountNumber`: Connector's account number from auth details.
    *   `CreditCard`: Contains card details.
        *   `Registered="False"` (can be true if using a token).
        *   `TokeniseAlgorithmID`: Set to `2` if `setup_future_usage` is `OffSession` or if it's a mandate payment.
        *   `CardNumber`, `ExpM`, `ExpY`, `CVN`, `CardHolderName`.
        *   If `PaymentMethodData::MandatePayment`, only `CardNumber` (which is the token/connector_mandate_id) and `TokeniseAlgorithmID` are sent.
    *   `Security`: Contains `UserName` and `Password` from auth details.

### Core Request Transformation (Payments & Mandates) (Bank of America Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` or `SetupMandateRouterData` is transformed into a `BankOfAmericaPaymentsRequest`.
*   **`BankOfAmericaRouterData<T>`**: A generic wrapper that pairs Hyperswitch router data with the `amount` (formatted as a string).
*   **`BankOfAmericaPaymentsRequest`**:
    *   `processing_information: ProcessingInformation`: Contains details about how the transaction should be processed.
        *   `action_list: Option<Vec<BankOfAmericaActionsList>>`: e.g., `TokenCreate` for mandates.
        *   `action_token_types: Option<Vec<BankOfAmericaActionsTokenType>>`: e.g., `PaymentInstrument`, `Customer` for mandates.
        *   `authorization_options: Option<BankOfAmericaAuthorizationOptions>`: For recurring/mandate payments, includes `initiator` and `merchant_intitiated_transaction` (with `original_authorized_amount`).
        *   `commerce_indicator: String`: Varies based on card network (e.g., "internet", "spa", "aesk").
        *   `capture: Option<bool>`: True for auto-capture, false for manual.
        *   `payment_solution: Option<String>`: Indicates wallet type (e.g., "001" for ApplePay, "012" for GooglePay).
    *   `payment_information: PaymentInformation`: An enum detailing the payment instrument.
        *   **`PaymentInformation::Cards(Box<CardPaymentInformation>)`**:
            *   `card: Card`: Contains `number`, `expiration_month`, `expiration_year`, `security_code`, and optional `card_type` (mapped from card network).
        *   **`PaymentInformation::GooglePay(Box<GooglePayPaymentInformation>)`**:
            *   `fluid_data: FluidData`: Contains Base64 encoded Google Pay token.
        *   **`PaymentInformation::ApplePay(Box<ApplePayPaymentInformation>)`**: (Used for Setup Mandate with Apple Pay token)
            *   `tokenized_card: TokenizedCard`: Contains decrypted Apple Pay token details like `number`, `expiration_month`, `expiration_year`, `cryptogram`.
        *   **`PaymentInformation::ApplePayToken(Box<ApplePayTokenPaymentInformation>)`**: (Used for Payments with raw Apple Pay token)
            *   `fluid_data: FluidData`: Contains raw Apple Pay token.
        *   **`PaymentInformation::SamsungPay(Box<SamsungPayPaymentInformation>)`**:
            *   `fluid_data: FluidData`: Contains Base64 encoded, structured Samsung Pay token.
            *   `tokenized_card: SamsungPayTokenizedCard`.
        *   **`PaymentInformation::MandatePayment(Box<MandatePaymentInformation>)`**:
            *   `payment_instrument: BankOfAmericaPaymentInstrument`: Contains the `id` (connector mandate ID).
    *   `order_information: OrderInformationWithBill`:
        *   `amount_details: Amount` (total_amount as string, currency). For setup mandate, amount is "0".
        *   `bill_to: Option<BillTo>`: Billing address details.
    *   `client_reference_information: ClientReferenceInformation`: Contains `code` (Hyperswitch `connector_request_reference_id`).
    *   `consumer_authentication_information: Option<BankOfAmericaConsumerAuthInformation>`: For 3DS or some wallet payments, can include `cavv`, `xid`, `ucaf_collection_indicator`.
    *   `merchant_defined_information: Option<Vec<MerchantDefinedInformation>>`: For metadata.

## From grace/guides/integrations/integrations.md

### Marker Trait Implementations (Structuring a New Hyperswitch Connector)
Empty implementations indicating supported Hyperswitch flows (e.g., `api::Payment`, `api::PaymentAuthorize`, `api::RefundExecute`).

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Structuring a New Hyperswitch Connector)
This is where the logic for each specific payment/refund flow (Authorize, Capture, PSync, Refund Execute, Refund Sync, etc.) resides. For each flow:
    *   `get_headers()`: Defines request headers.
    *   `get_content_type()`: Defines request content type.
    *   `get_url()`: Constructs the specific API endpoint URL for the flow.
    *   `get_request_body()`: Transforms Hyperswitch's `RouterData` into the connector's specific request struct (from `transformers.rs`).
    *   `build_request()`: Assembles the `services::Request` object.
    *   `handle_response()`: Parses the connector's HTTP response into its specific response struct and transforms it back into Hyperswitch's `RouterData`.
    *   `get_error_response()`: Handles error responses, usually delegating to `build_error_response`.

### `ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
    *   `get_headers()`: Often delegates to a common `build_headers()` (from `ConnectorCommonExt`) which might add shared headers (like `Content-Type`, `Authorization` from access token). Flow-specific headers or idempotency keys are added here or in `build_headers`.
    *   `get_url()`: Constructs the full API endpoint URL for the specific flow, often by appending a path to the `base_url()`. Path parameters (like transaction IDs or order IDs) are interpolated here.
    *   `get_request_body()`: This is a critical transformation step.
        *   It typically involves creating an instance of `[ConnectorName]RouterData` (a helper struct from `transformers.rs` that bundles `RouterData` with a converted amount).
        *   Then, it calls `TryFrom` on the connector-specific request struct (also from `transformers.rs`) passing the `[ConnectorName]RouterData`. This `TryFrom` implementation handles the detailed mapping from Hyperswitch's generic `RouterData` fields to the connector's expected request structure.
        *   Finally, it returns `RequestContent::Json(Box::new(connector_req))` for JSON bodies or `RequestContent::FormUrlEncoded(Box::new(connector_req))` for form-encoded bodies (like ACI).
    *   `handle_response()`:
        *   Parses the raw HTTP response (e.g., `res.response.parse_struct("[ConnectorName]ResponseStruct")`).
        *   The parsed connector-specific response struct is then converted back into Hyperswitch's generic `RouterData<Flow, RequestData, ResponseData>` by implementing `TryFrom<ResponseRouterData<F, [ConnectorName]ResponseStruct, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>` in `transformers.rs`. This involves mapping status codes, extracting transaction IDs, handling redirection data, and potentially storing connector-specific metadata.
    *   `get_error_response()`: Usually delegates to `self.build_error_response()` defined in `ConnectorCommon`.

### Authentication Mechanisms (Learnings from Analyzing Existing Hyperswitch Connectors)
(This section describes various authentication methods, some of which are part of authorization flows)
    *   **Basic Auth**: Braintree (`public_key:private_key`), Klarna (`username:password`).
    *   **API Key in Header**: Adyen (`x-api-key`), Stripe (`Authorization: Bearer <secret_key>`), Paypal (`Authorization: Bearer <access_token>`), Adyenplatform (`Authorization: <api_key>`).
    *   **OAuth or Custom Token Flow for Bearer Token**: Connectors like Paypal and Airwallex first obtain an access token.
        *   Paypal uses a standard OAuth-like flow.
        *   Airwallex uses a custom login endpoint (`/authentication/login`) with `X-API-KEY` and `X-CLIENT-ID` headers to get a Bearer token, which is then used for subsequent API calls in an `Authorization: Bearer <token>` header.
    *   **Custom Signature**:
        *   Fiserv: HMAC-SHA256 of concatenated string.
        *   Cybersource: HMAC-SHA256 of a string composed of specific headers (host, date, request-target, v-c-merchant-id) and a payload digest. The signature is passed in a `Signature` header, along with `v-c-merchant-id`, `Date`, `Host`, and `Digest` (for POST/PATCH). `CybersourceAuthType` holds `api_key`, `merchant_account`, and `api_secret`.
        *   ACI: Uses `Authorization` header with Bearer token from config.
    *   **Nuvei**: Uses a `session_token` obtained via `getSessionToken.do`, then includes `merchant_id`, `merchant_site_id`, `timestamp`, and a `checksum` (SHA256 hash of concatenated fields + secret) in payment requests.
    *   **Stripebilling**: Uses a Bearer token (`Authorization: Bearer <api_key>`) along with a specific API version header (`stripe-version: 2022-11-15`). The `StripebillingAuthType` in `transformers.rs` holds the `api_key`.
    *   **Shift4**: Uses a Bearer token (`Authorization: <api_key>`). The `Shift4AuthType` in `transformers.rs` holds the `api_key`.
    *   **Globalpay**: Implements an access token flow. `GlobalpayAuthType` stores `app_id` and `key`. To get an access token, a request is made to `/accesstoken` with `app_id`, a `nonce`, and a `secret` (SHA512 hash of `nonce + key`). Subsequent API calls use `Authorization: Bearer <access_token>` and an `X-GP-Version` header.
    *   **Worldpay**: Uses Basic Authentication (`Authorization: Basic <base64_encoded_string>`). The `WorldpayAuthType` in `transformers.rs` constructs this from `key1` (username) and `api_key` (password) from `ConnectorAuthType::SignatureKey`. It also requires an `X-WP-API-Version` header. The `entity_id` (from `api_secret` in `ConnectorAuthType::SignatureKey`) is included in request bodies.
    *   **Paypal**: Implements an OAuth 2.0 client credentials flow to obtain a Bearer token. `PaypalAuthType` can be `StandardIntegration` (client_id, client_secret) or `PartnerIntegration` (client_id, client_secret, payer_id). The access token is then used in `Authorization: Bearer <token>` header. Additional headers like `PayPal-Partner-Attribution-Id`, `PayPal-Request-Id`, and `Prefer` are also used. For partner integrations, a `PayPal-Auth-Assertion` header is constructed.

### Detailed Request/Response Transformation: PayPal Example (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   **Request Transformation (`PaypalPaymentsRequest::try_from(&PaypalRouterData<&PaymentsAuthorizeRouterData>)`)**:
        *   **Intent**: Determined by `is_auto_capture()`: `PaypalPaymentIntent::Capture` or `PaypalPaymentIntent::Authorize`.
        *   **`purchase_units`**: A `Vec<PurchaseUnitRequest>`. Typically one unit.
            *   `reference_id`, `custom_id`, `invoice_id`: Mapped from `connector_request_reference_id` and `merchant_order_reference_id`.
            *   `amount`: An `OrderRequestAmount` struct.
                *   `currency_code`: From `RouterData.request.currency`.
                *   `value`: From `PaypalRouterData.amount` (already converted to major unit string).
                *   `breakdown`: An `AmountBreakdown` struct.
                    *   `item_total`: `OrderAmount` with `value` from `PaypalRouterData.amount`.
                    *   `shipping`: `OrderAmount` with `value` from `PaypalRouterData.shipping_cost`.
            *   `payee`: Optional `Payee` struct with `merchant_id` (Paypal Payer ID from auth credentials if partner integration).
            *   `shipping`: Optional `ShippingAddress` struct, mapped from `RouterData.shipping_address`.
            *   `items`: A `Vec<ItemDetails>`, typically one item with name, quantity 1, and `unit_amount` (from `PaypalRouterData.amount`).
        *   **`payment_source`**: An `Option<PaymentSourceItem>` enum. This is where different payment methods are handled:
            *   `Card`: `PaymentSourceItem::Card(CardRequest::CardRequestStruct(...))`
                *   `billing_address`: Mapped from `RouterData.billing_address`.
                *   `expiry`: Formatted as `YYYY-MM`.
                *   `name`: From billing full name.
                *   `number`: `CardNumber`.
                *   `security_code`: CVC.
                *   `attributes.vault`: If `setup_future_usage` is `OffSession`, includes `PaypalVault` with `store_in_vault: OnSuccess` and `usage_type: Merchant`.
                *   `attributes.verification`: If `auth_type` is `ThreeDs`, includes `ThreeDsMethod` with `method: ScaAlways`.
            *   `PaypalRedirect`: `PaymentSourceItem::Paypal(PaypalRedirectionRequest::PaypalRedirectionStruct(...))`
                *   `experience_context`: `ContextStruct` with `return_url`, `cancel_url` (from `complete_authorize_url`), `shipping_preference` (`SetProvidedAddress` or `GetFromFile`), and `user_action: PayNow`.
                *   `attributes.vault`: Similar to Card for `OffSession` mandate.
            *   `BankRedirect` (Eps, Giropay, Ideal, Sofort): `PaymentSourceItem::Eps(RedirectRequest(...))` etc.
                *   `name`: Billing full name.
                *   `country_code`: Billing country.
                *   `experience_context`: Similar to PaypalRedirect.
            *   `MandatePayment`:
                *   If PMD is Card: `PaymentSourceItem::Card(CardRequest::CardVaultStruct(VaultStruct { vault_id: connector_mandate_id }))`.
                *   If PMD is Paypal: `PaymentSourceItem::Paypal(PaypalRedirectionRequest::PaypalVaultStruct(VaultStruct { vault_id: connector_mandate_id }))`.
    *   **Response Transformation (`RouterData::try_from(ResponseRouterData<F, PaypalAuthResponse, T, PaymentsResponseData>)`)**:
        *   The `PaypalAuthResponse` is an enum that can be `PaypalOrdersResponse`, `PaypalRedirectResponse`, or `PaypalThreeDsResponse`.
        *   **`PaypalOrdersResponse`**:
            *   `status`: Mapped from `PaypalOrdersResponse.status` (e.g., `COMPLETED`, `PAYER_ACTION_REQUIRED`) and `intent` to Hyperswitch `AttemptStatus`.
            *   `resource_id`: `PaypalOrdersResponse.id` (Order ID).
            *   `connector_metadata`: A `PaypalMeta` struct is created.
                *   `authorize_id` or `capture_id`: Extracted from the first `purchase_units.payments.authorizations[0].id` or `captures[0].id`.
                *   `psync_flow`: Set to the `intent` from the response.
            *   `mandate_reference`: If `payment_source.paypal.attributes.vault.id` or `payment_source.card.attributes.vault.id` is present, it's used as `connector_mandate_id`.
            *   `redirection_data`: Typically `None` for direct order responses unless `PAYER_ACTION_REQUIRED`.
        *   **`PaypalRedirectResponse`**:
            *   `status`: Mapped from `PaypalRedirectResponse.status` and `intent`.
            *   `resource_id`: `PaypalRedirectResponse.id`.
            *   `redirection_data`: A `RedirectForm` is constructed using the `href` from `links` where `rel == "payer-action"`.
            *   `connector_metadata`: `PaypalMeta` with `psync_flow` set to the response `intent`. If `payment_experience` is `InvokeSdkClient`, `next_action` is set to `CompleteAuthorize`.
        *   **`PaypalThreeDsResponse`**:
            *   `status`: Mapped from `PaypalThreeDsResponse.status` (usually `PAYER_ACTION_REQUIRED`).
            *   `resource_id`: `PaypalThreeDsResponse.id`.
            *   `redirection_data`: A `RedirectForm` is constructed using the `href` from `links` where `rel == "payer-action"`. The `redirect_uri` (Hyperswitch's `complete_authorize_url`) is added as a form field.
            *   `connector_metadata`: `PaypalMeta` with `psync_flow` set to `Authenticate`.

### `hyperswitch_domain_models` (Commonly Used Hyperswitch Types and Utilities)
    *   `payment_method_data::{PaymentMethodData, Card, WalletData, BankRedirectData, PayLaterData}`: Central for handling different payment types.
    *   `router_data::{RouterData, ConnectorAuthType, ErrorResponse, AccessToken}`: Core data carriers for flows.
    *   `router_request_types` & `router_response_types`: Define structures for specific flow requests/responses (e.g., `PaymentsAuthorizeData`, `PaymentsResponseData`).
    *   `address::{Address, AddressDetails}`: For billing and shipping information.
    *   `types::*`: Various supporting types.

### Request Transformation (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
        *   Create a helper struct: `pub struct [ConnectorName]RouterData<T> { amount: common_utils::types::MinorUnit, router_data: T }` (adjust `MinorUnit` based on `[ConnectorName]`'s currency unit).
        *   For each request struct: `impl TryFrom<&[ConnectorName]RouterData<&PaymentsAuthorizeRouterData>> for [ConnectorName]PaymentsRequest { ... }`.
            *   This implementation will map fields from Hyperswitch's generic `RouterData` (e.g., `router_data.request.amount`, `router_data.payment_method_data`, `router_data.address`) to `[ConnectorName]`'s request fields.
            *   Handle amount conversion using the appropriate utility (e.g., `utils::to_currency_base_unit_as_string`, `utils::to_currency_minor_unit_as_string`).
            *   Use `masking::Secret` for sensitive data and `.peek()` or `.expose()` when providing it to the connector.
            *   Access `router_data.router_data.request.connector_meta_data` if you need to retrieve previously stored metadata.

### Implement marker traits for supported flows (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `impl api::Payment for [ConnectorNamePascalCase] {}`, `impl api::PaymentAuthorize for [ConnectorNamePascalCase] {}`).

### Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow (How to Structure a New Connector: `[ConnectorName]` (Practical Guide))
(e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
    *   `get_headers()`: Define request headers. Often includes `Content-Type` and auth headers. Add idempotency keys if required by `[ConnectorName]` (e.g., `Idempotency-Key`, `PayPal-Request-Id`).
    *   `get_content_type()`: Return the content type from `ConnectorCommon`.
    *   `get_url()`: Construct the full API endpoint URL for this specific flow, appending paths to `base_url()`.
    *   `get_request_body()`:
        *   Create `[connector_name_module]::[ConnectorName]RouterData { amount: converted_amount, router_data: item }`.
        *   Convert it to `[ConnectorName]`'s request struct: `let connector_req = [connector_name_module]::[ConnectorName]PaymentsRequest::try_from(&router_data_obj)?;`
        *   Return `Ok(Some(types::RequestBody::log_and_get_request_body(Box::new(connector_req), utils::Encode::<[connector_name_module]::[ConnectorName]PaymentsRequest>::url_encode_to_string_tagged)?))` for form-urlencoded or `Ok(Some(types::RequestBody::log_and_get_request_body(Box::new(connector_req), utils::Encode::<[connector_name_module]::[ConnectorName]PaymentsRequest>::encode_to_string_of_json)?))` for JSON.
    *   `build_request()`: Assemble the `services::Request` object using the above methods.
    *   `handle_response()`:
        *   Parse `[ConnectorName]`'s HTTP response into its specific response struct: `let response: [connector_name_module]::[ConnectorName]PaymentsResponse = res.response.parse_struct("[ConnectorNamePascalCase] PaymentsResponse")?;`
        *   Convert it back to Hyperswitch's `RouterData`: `types::RouterData::try_from(types::ResponseRouterData { response, data: item.data, router_data: item.router_data })?`
    *   `get_error_response()`: Usually delegates to `self.build_error_response()`.

### `AdyenPaymentRequest<'a>` (Connector Deep Dive: Adyen)
    *   **Fields**: Includes `amount`, `merchant_account`, `payment_method` (an enum itself), `reference` (Hyperswitch's `connector_request_reference_id`), `return_url`, `browser_info`, `shopper_interaction`, `recurring_processing_model`, `additional_data`, `shopper_reference`, `store_payment_method`, address details, etc.
    *   **`TryFrom<&AdyenRouterData<&PaymentsAuthorizeRouterData>> for AdyenPaymentRequest<'_>`**: This is a complex implementation that branches based on:
        *   **Mandate ID**: If `mandate_id` is present in `RouterData`, it constructs an `AdyenMandate` within the `payment_method` field.
            *   `ConnectorMandateId`: Uses `storedPaymentMethodId`.
            *   `NetworkMandateId`: Constructs an `AdyenCard` with `networkPaymentReference`.
            *   `NetworkTokenWithNTI`: Constructs an `AdyenNetworkTokenData` with `networkPaymentReference`.
        *   **`PaymentMethodData` variant**:
            *   `Card`: Creates `AdyenCard` (maps card number, expiry, CVC, holder name, brand).
            *   `Wallet`: Handles various wallet types (GooglePay, ApplePay, Paypal, AliPay, etc.) by creating corresponding `AdyenPaymentMethod` enum variants (e.g., `AdyenGPay`, `AdyenApplePay`, `AdyenPaypal`). Specific data like tokens (`googlePayToken`, `applePayToken`) are mapped.
            *   `PayLater`: Handles Klarna, Affirm, AfterpayClearpay, etc. Requires specific fields like email, customer ID, country, and sometimes line items.
            *   `BankRedirect`: Handles Bancontact, BLIK, EPS, iDEAL, Sofort, Trustly, etc. Often involves mapping issuer codes or specific bank data.
            *   `BankDebit`: Handles ACH, SEPA, BACS. Maps account numbers, routing/sort codes, IBAN, and owner names.
            *   `BankTransfer`: Handles various virtual account methods (Permata, BCA, BNI, etc.) and Pix.
            *   `Voucher`: Handles Boleto, Oxxo, convenience store vouchers.
            *   `GiftCard`: Handles PaySafeCard, Givex.
            *   `NetworkToken`: Creates `AdyenNetworkTokenData`.
    *   **Common Logic in `TryFrom` for `AdyenPaymentRequest`**:
        *   `amount`: From `AdyenRouterData.amount`.
        *   `merchant_account`: From `AdyenAuthType`.
        *   `shopper_interaction`: Determined by `RouterData.request.off_session` (`Ecommerce` or `ContinuedAuthentication`).
        *   `recurring_processing_model`, `store_payment_method`, `shopper_reference`: Determined by `setup_future_usage` and `off_session` flags, and customer ID.
        *   `browser_info`: Populated if 3DS is required or for certain payment methods, using `RouterData.request.get_browser_info()`.
        *   `additional_data`: Includes `authorisation_type` (for manual capture), `manual_capture` flag, `execute_three_d` flag, and `riskdata` (if present in `RouterData.request.metadata`).
        *   `return_url`: From `RouterData.request.get_router_return_url()`.
        *   Address details (`billing_address`, `delivery_address`), shopper details (`shopper_name`, `shopper_email`, `telephone_number`), `country_code`, `line_items` are populated from `RouterData`.
        *   `channel`: Can be `Web` for certain payment methods like GoPay, Vipps.
        *   `splits`: If `RouterData.request.split_payments` is `AdyenSplitPayment`, it's mapped to `AdyenSplitData`.
        *   `device_fingerprint`: Extracted from `RouterData.request.metadata`.

### Payments (Authorize, SetupMandate) (Connector Deep Dive: Adyen)
    *   The `AdyenPaymentRequest` is constructed as detailed above, handling various PMDs.
    *   Response handling involves parsing `AdyenPaymentResponse` and its variants.
