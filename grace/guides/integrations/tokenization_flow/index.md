# Tokenization (Mandate Setup) Flow Information

This document consolidates information related to the Tokenization (Mandate Setup) flow, extracted from various guides.

## From grace/guides/learning/learning.md

### `RouterData` Construction in `AccessTokenAuth::handle_response` (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Correction (21/05/2025)**: Removed the attempt to set `status: common_enums::AttemptStatus::Tokenized` as it's not a valid variant and status should be preserved or handled generically. (This implies "Tokenized" is a status that might be relevant).

### Flow: `PaymentMethodToken` (Bambora Connector (bambora.rs) Implementation Comparison)
#### My Implementation:
Fully implemented using `/tokens` endpoint and `BamboraTokenizationRequest`/`Response`.
#### Reference Implementation:
Marked as `// Not Implemented (R)`.
#### Differences & Lessons Learned:
*   My implementation of the dedicated tokenization flow seems valid based on the API docs for `/tokens`. The reference codebase might handle tokenization differently or not prioritize this specific flow integration.

## From grace/guides/types/types.md

### Core Request Transformation (Payments) (Adyen Connector Type Mappings)
*   **`AdyenPaymentRequest`**:
    *   `store_payment_method: Option<bool>` (for tokenization).
    *   `recurring_processing_model: Option<AdyenRecurringModel>` (e.g., `UnscheduledCardOnFile`).
*   **Response Transformation (Payments)**:
    *   `MandateReference`: Extracted from `additional_data.recurring_detail_reference`.

### Mandate (Customer Profile) Creation (Authorize.Net Connector Type Mappings)
*   `SetupMandateRouterData` is transformed into `CreateCustomerProfileRequest`.
*   This request creates a customer profile and a payment profile in Authorize.Net.
*   `validation_mode`: `TestMode` or `LiveMode` based on Hyperswitch `test_mode`.
*   The payment profile includes card details or opaque data for wallets.
*   **Response Transformation (Payments)**:
    *   `MandateReference`: If `profile_response` is present, `customer_profile_id` and `customer_payment_profile_id_list` are combined to form the `connector_mandate_id`.

### Core Request Transformation (Card Payments & Mandates) (Bambora APAC Connector Type Mappings)
*   **SOAP Request Structure**:
    *   The request is wrapped in a `<soapenv:Envelope>` with a `<dts:SubmitSinglePayment>` or `<sipp:TokeniseCreditCard>` operation.
*   **`<Transaction>` (for Payments)**:
    *   `CreditCard`:
        *   `TokeniseAlgorithmID`: Set to `2` if `setup_future_usage` is `OffSession` or if it's a mandate payment.
*   **`<TokeniseCreditCard>` (for Setup Mandate/Tokenization)**:
    *   Used when `SetupMandateRouterData` is processed.
    *   Contains `CardNumber`, `ExpM`, `ExpY`, `CardHolderName`, `TokeniseAlgorithmID` (set to `2`), and `Security` details.
*   **Core Response Transformation (Payments & Mandates)**:
    *   **`BamboraapacPaymentsResponse` (for Payments)**:
        *   `CreditCardToken: Option<String>`: Token for the card if tokenization was requested.
    *   **`BamboraapacMandateResponse` (for Setup Mandate)**:
        *   **`MandateResponseBody`**:
            *   `Token: Option<String>`: The generated card token (connector mandate ID).
    *   **Mandate Reference**: For successful payment with tokenization or successful mandate setup, `CreditCardToken` or `Token` is used as `connector_mandate_id`.

### Core Request Transformation (Payments & Mandates) (Bank of America Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` or `SetupMandateRouterData` is transformed into a `BankOfAmericaPaymentsRequest`.
*   **`BankOfAmericaPaymentsRequest`**:
    *   `processing_information: ProcessingInformation`:
        *   `action_list: Option<Vec<BankOfAmericaActionsList>>`: e.g., `TokenCreate` for mandates.
        *   `action_token_types: Option<Vec<BankOfAmericaActionsTokenType>>`: e.g., `PaymentInstrument`, `Customer` for mandates.
    *   `payment_information: PaymentInformation`:
        *   **`PaymentInformation::ApplePay(Box<ApplePayPaymentInformation>)`**: (Used for Setup Mandate with Apple Pay token)
            *   `tokenized_card: TokenizedCard`: Contains decrypted Apple Pay token details like `number`, `expiration_month`, `expiration_year`, `cryptogram`.
*   **Mandate/Tokenization**:
    *   If `setup_future_usage` is `OffSession`, `action_list` includes `TokenCreate`, and `action_token_types` include `PaymentInstrument` and `Customer`. `authorization_options` are set for customer-initiated stored credentials.
    *   For zero-auth setup mandates (`SetupMandateRouterData`), the amount is "0".
*   **Core Response Transformation (Payments & Mandates)**:
    *   **`BankOfAmericaClientReferenceResponse`**:
        *   `token_information: Option<BankOfAmericaTokenInformation>`: Contains `payment_instrument.id` (connector mandate ID) if a token was created.
    *   **Mandate Reference**: If `token_information.payment_instrument.id` is present, it's used as `connector_mandate_id`.

## From grace/guides/integrations/integrations.md

### Mandates and Tokenization (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   **Stripe**: Uses `setup_future_usage` to create SetupIntents for tokenization.
    *   **Braintree**: Can vault payment methods during authorization/charge (`vaultPaymentMethodAfterTransacting`). Uses GraphQL mutations.
    *   **Globalpay**: Supports mandates for Card, Paypal, GooglePay, Ideal, Sofort, Eps, Giropay. The `GlobalpayPaymentsRequest` includes optional `initiator` (Merchant/Payer) and `stored_credential` (model: Recurring, sequence: First/Subsequent) fields, determined by `off_session` status and the presence of a `connector_mandate_id` (which populates `brand_reference` in the card data). The response can include a `brand_reference` in the card details, which is used as the `connector_mandate_id`.
    *   **Worldpay**: The `WorldpayPaymentsRequest.instruction` can include `token_creation` (type: Worldpay) and `customer_agreement` (type: Subscription/Unscheduled, usage: First/Subsequent, scheme_reference) for CIT/MIT flows. If a `connector_mandate_id` is provided, it's used to populate `PaymentInstrument::CardToken`. The `AuthorizedResponse.token.href` from the response is used as the `connector_mandate_id`.
    *   **Paypal**:
        *   For card payments, `setup_future_usage: OffSession` in `RouterData` translates to including `attributes.vault` with `store_in_vault: OnSuccess` and `usage_type: Merchant` in the `PaypalPaymentsRequest.payment_source.Card.CardRequestStruct`.
        *   For PayPal wallet payments, `setup_future_usage: OffSession` similarly adds `attributes.vault` to the `PaypalRedirectionStruct`.
        *   The `PaypalSetupMandatesResponse` (from `/v3/vault/payment-tokens/` endpoint) returns an `id` which is used as the `connector_mandate_id`.
        *   For subsequent payments using a token, the `connector_mandate_id` is sent in `PaymentSourceItem::Card(CardRequest::CardVaultStruct(...))` or `PaymentSourceItem::Paypal(PaypalRedirectionRequest::PaypalVaultStruct(...))`.
    *   **Cybersource**: Supports `TokenCreate` action list for creating payment instruments/customer tokens.
    *   **ACI**: Uses `registrations/{id}/payments` for subsequent payments with a stored token/mandate. `createRegistration: true` in initial payment.
    *   Connector-specific mandate IDs are stored and retrieved via `RouterData.request.mandate_id` and `MandateReference` in responses.

### Payments (Authorize, SetupMandate) (Connector Deep Dive: Adyen)
    *   The `AdyenPaymentRequest` is constructed as detailed above, handling various PMDs.
    *   Response handling involves parsing `AdyenPaymentResponse` and its variants.
    *   (Relevant parts of AdyenPaymentRequest for tokenization):
        *   `store_payment_method: Option<bool>`
        *   `recurring_processing_model: Option<AdyenRecurringModel>`
        *   `shopper_reference: Option<String>`
    *   (Relevant parts of AdyenPaymentResponse for tokenization):
        *   `additional_data`: Can contain `recurring_detail_reference` (mandate ID).
