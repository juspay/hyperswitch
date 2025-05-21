# Connector Type Mappings

This document outlines the key data types and transformations used in the connector `transformers.rs` files for mapping Hyperswitch's generic payment models to connector-specific request and response formats.

---

# ACI Connector Type Mappings

This document outlines the key data types and transformations used in the ACI connector's `transformers.rs` file for mapping Hyperswitch's generic payment models to ACI-specific request and response formats.

## Core Request Transformation

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

## Core Response Transformation

ACI's payment responses (`AciPaymentsResponse` or `AciErrorResponse`) are converted back into Hyperswitch's generic `RouterData<F, T, PaymentsResponseData>`:

*   **`AciPaymentsResponse`**: Contains the outcome of a payment, including:
    *   `id`: The connector's transaction identifier.
    *   `registration_id`: The connector's mandate identifier, if applicable.
    *   `result: ResultCode`: An object containing ACI's `code` and `description` for the transaction outcome.
    *   `redirect: Option<AciRedirectionData>`: Details for any shopper redirection required.
*   **Status Mapping**:
    *   The `ResultCode.code` from ACI is first parsed into a local `AciPaymentStatus` enum (`Succeeded`, `Failed`, `Pending`, `RedirectShopper`) by checking against predefined lists of ACI codes (`SUCCESSFUL_CODES`, `PENDING_CODES`, `FAILURE_CODES`).
    *   This `AciPaymentStatus` is then mapped to Hyperswitch's standard `common_enums::AttemptStatus` (e.g., `Charged`, `Failure`, `Authorizing`, `AuthenticationPending`).
*   **Redirection**: If `AciRedirectionData` is present, it's converted into Hyperswitch's `RedirectForm` structure, providing the endpoint, method, and form fields for the redirect.
*   **Mandate Reference**: The `registration_id` from ACI is used to populate `MandateReference` in the Hyperswitch response.

## Refund Transformation

*   **Request**: Hyperswitch's `RefundsRouterData` is transformed into an `AciRefundRequest`, which includes amount, currency, entity ID, and sets `payment_type` to `AciPaymentType::Refund`.
*   **Response**: ACI's `AciRefundResponse` is mapped back to `RefundsRouterData`.
    *   The `ResultCode.code` is parsed into `AciRefundStatus` (`Succeeded`, `Failed`, `Pending`).
    *   `AciRefundStatus` is then mapped to Hyperswitch's `common_enums::RefundStatus` (`Success`, `Failure`, `Pending`).
    *   The `id` from `AciRefundResponse` becomes the `connector_refund_id`.

## Key Data Types Used

*   **`common_utils::types::StringMajorUnit`**: Used for representing monetary amounts.
*   **`masking::Secret<String>`**: Wraps sensitive data like API keys, card numbers, CVV, etc.
*   **`hyperswitch_domain_models`**: Source structures like `Card`, `WalletData`, `BankRedirectData` from this crate are the primary inputs for request transformation.
*   **`api_models::enums` and `common_enums`**: Enums from these crates (e.g., `CountryAlpha2`, `BankNames`, `AttemptStatus`, `RefundStatus`) are used for standardized field values and status mapping.

## Error Handling

Error information from ACI is primarily conveyed through the `ResultCode` object present in both successful and error responses. This object contains:
*   `code`: The ACI specific result code.
*   `description`: A human-readable description of the result.
*   `parameter_errors: Option<Vec<ErrorParameters>>`: A list of errors related to specific request parameters, if any.

This `ResultCode` is crucial for determining the appropriate Hyperswitch status and for providing debugging information.

---

# Adyen Connector Type Mappings

This document outlines the key data types and transformations used in the Adyen connector's `transformers.rs` file. It details how Hyperswitch's generic payment models are mapped to Adyen-specific request and response formats for various operations like payments, refunds, and payouts.

## Core Request Transformation (Payments)

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

## Core Response Transformation (Payments)

Adyen's payment responses (`AdyenPaymentResponse`, which is an enum itself) are converted into Hyperswitch's `RouterData<F, Req, PaymentsResponseData>`.

*   **`AdyenPaymentResponse` (enum)**: Can be one of:
    *   `Response(Box<AdyenResponse>)`: Standard synchronous response.
    *   `PresentToShopper(Box<PresentToShopperResponse>)`: For voucher/display methods.
    *   `QrCodeResponse(Box<QrCodeResponseResponse>)`: For QR code flows.
    *   `RedirectionResponse(Box<RedirectionResponse>)`: For redirect flows.
    *   `RedirectionErrorResponse(Box<RedirectionErrorResponse>)`: For errors in redirect flows.
    *   `WebhookResponse(Box<AdyenWebhookResponse>)`: For webhook-driven updates.

*   **Key fields in `AdyenResponse` (standard synchronous response)**:
    *   `psp_reference: String` (Connector's transaction ID).
    *   `result_code: AdyenStatus` (Adyen's status enum).
    *   `merchant_reference: String` (Hyperswitch's `connector_request_reference_id`).
    *   `refusal_reason: Option<String>`, `refusal_reason_code: Option<String>`.
    *   `additional_data: Option<AdditionalData>` (can contain `recurring_detail_reference` for mandates, `network_tx_reference`).
    *   `splits: Option<Vec<AdyenSplitData>>` (for split payment responses).

*   **Status Mapping**:
    *   `AdyenStatus` (e.g., `Authorised`, `Refused`, `RedirectShopper`, `ChallengeShopper`, `Cancelled`, `Received`, `Pending`) is mapped to Hyperswitch's `storage_enums::AttemptStatus`.
    *   The mapping depends on whether manual capture is enabled (`is_manual_capture`). For example, `AdyenStatus::Authorised` maps to `AttemptStatus::Authorized` if manual capture, else `AttemptStatus::Charged`.
    *   Webhook status `AdyenWebhookStatus` also has its own mapping to `storage_enums::AttemptStatus`.

*   **Redirection**:
    *   If `RedirectionResponse` is received, its `action: AdyenRedirectAction` (containing `url`, `method`, `data`) is converted into Hyperswitch's `RedirectForm`.
    *   `QrCodeResponseResponse`'s `action: AdyenQrCodeAction` (with `qr_code_data`, `qr_code_url`) is used to populate `QrCodeInformation` in `connector_metadata`.
    *   `PresentToShopperResponse`'s `action: AdyenPtsAction` (with `reference`, `expires_at`, `download_url`) is used for `VoucherNextStepData` or `BankTransferInstructions` in `connector_metadata`.

*   **Mandate Reference**: `additional_data.recurring_detail_reference` is used to populate `MandateReference`.
*   **Error Handling**: `refusal_reason` and `refusal_reason_code` from Adyen responses are used to populate `ErrorResponse`. `additional_data` can also contain `refusal_code_raw` and `refusal_reason_raw`. `merchant_advice_code` from `additional_data` is also used.

## Refund Transformation

*   **Request**: `AdyenRouterData<&RefundsRouterData<F>>` is transformed into `AdyenRefundRequest`.
    *   Includes `merchant_account`, `amount`, `merchant_refund_reason`, and `reference` (Hyperswitch's `refund_id`).
    *   Supports `splits` for Adyen's split refund feature.
*   **Response**: `AdyenRefundResponse` (containing `psp_reference`, `status`) is mapped to `RefundsRouterData`.
    *   Adyen's refund `status` (typically "received") is mapped to `storage_enums::RefundStatus::Pending`, as the final outcome is usually via webhook.

## Capture Transformation

*   **Request**: `AdyenRouterData<&PaymentsCaptureRouterData>` is transformed into `AdyenCaptureRequest`.
    *   Includes `merchant_account`, `amount`, and `reference`. The `reference` is the `capture_id` for multiple captures or `attempt_id` for single captures.
*   **Response**: `AdyenCaptureResponse` is mapped to `PaymentsCaptureRouterData`.
    *   Status is typically set to `storage_enums::AttemptStatus::Pending`.
    *   `psp_reference` (for multiple captures) or `payment_psp_reference` (for single capture) becomes the `resource_id`.

## Cancellation (Void) Transformation

*   **Request**: `PaymentsCancelRouterData` is transformed into `AdyenCancelRequest`.
    *   Includes `merchant_account` and `reference` (Hyperswitch's `connector_request_reference_id`).
*   **Response**: `AdyenCancelResponse` is mapped to `PaymentsCancelRouterData`.
    *   Status is typically set to `storage_enums::AttemptStatus::Pending`.

## Payouts Transformation (if `payouts` feature is enabled)

*   **Create Request**: `AdyenRouterData<&PayoutsRouterData<F>>` is transformed into `AdyenPayoutCreateRequest`.
    *   Includes `amount`, `recurring` (contract type, usually `Payout`), `merchant_account`, `payment_data` (bank or wallet details), `reference`, `shopper_reference`, and shopper details.
    *   `PayoutPaymentMethodData` can be `PayoutBankData` (with `PayoutBankDetails` like IBAN, owner name) or `PayoutWalletData` (e.g., for PayPal with email).
*   **Create Response**: `AdyenPayoutResponse` is mapped to `PayoutsRouterData`.
    *   `psp_reference` becomes `connector_payout_id`.
    *   Status is derived from `result_code` or `response` fields, mapping to `storage_enums::PayoutStatus`.
*   **Fulfill Request**: `AdyenRouterData<&PayoutsRouterData<F>>` is transformed into `AdyenPayoutFulfillRequest`.
    *   Can be `GenericFulfillRequest` (with `original_reference`) for bank/wallet payouts or `Card` (with `PayoutFulfillCardRequest` containing card details) for card payouts.
*   **Cancel Request**: `PayoutsRouterData<F>` is transformed into `AdyenPayoutCancelRequest`.
    *   Includes `original_reference` (connector payout ID) and `merchant_account`.
*   **Eligibility Request**: `AdyenRouterData<&PayoutsRouterData<F>>` is transformed into `AdyenPayoutEligibilityRequest`.
    *   Checks if a card is eligible for payouts.

## Dispute Handling Transformation

*   **Accept Dispute Request**: `AcceptDisputeRouterData` is transformed into `AdyenAcceptDisputeRequest`.
    *   Includes `dispute_psp_reference` and `merchant_account_code`.
*   **Accept Dispute Response**: `AdyenDisputeResponse` (boolean `success` field) is mapped to `AcceptDisputeResponse`.
    *   Sets `dispute_status` to `DisputeAccepted` if successful.
*   **Defend Dispute Request**: `DefendDisputeRouterData` is transformed into `AdyenDefendDisputeRequest`.
    *   Includes `dispute_psp_reference`, `merchant_account_code`, and a fixed `defense_reason_code` ("SupplyDefenseMaterial").
*   **Defend Dispute Response**: `AdyenDisputeResponse` is mapped to `DefendDisputeResponse`.
    *   Sets `dispute_status` to `DisputeChallenged` if successful.
*   **Submit Evidence Request**: `SubmitEvidenceRouterData` is transformed into `Evidence`.
    *   `Evidence` contains `defense_documents` (derived from various evidence fields in Hyperswitch), `merchant_account_code`, and `dispute_psp_reference`.
    *   `DefenseDocuments` includes base64 encoded `content`, `content_type`, and `defense_document_type_code`.
*   **Submit Evidence Response**: `AdyenDisputeResponse` is mapped to `SubmitEvidenceResponse`.
    *   Sets `dispute_status` to `DisputeChallenged` if successful.

## Webhook Transformation

*   **Incoming Webhook**: `AdyenIncomingWebhook` (containing `notification_items`) is processed. Each `AdyenNotificationRequestItemWH` is the core of a webhook event.
*   **`AdyenNotificationRequestItemWH`**: Contains `psp_reference`, `original_reference`, `event_code` (`WebhookEventCode` enum), `merchant_account_code`, `merchant_reference`, `success` (boolean as string), `amount`, `additional_data` (with `hmac_signature`, `dispute_status`, `chargeback_reason_code`, `recurring_detail_reference`, etc.).
*   **Event Mapping**: `WebhookEventCode` (e.g., `Authorisation`, `Refund`, `Capture`, `NotificationOfChargeback`, `Chargeback`) along with the `success` field and `dispute_status` are used to determine the Hyperswitch `IncomingWebhookEvent` (e.g., `PaymentIntentSuccess`, `RefundFailure`, `DisputeOpened`, `DisputeWon`).
*   The `AdyenNotificationRequestItemWH` is also transformed into an `AdyenWebhookResponse` struct which is then used to update the payment status similar to synchronous responses.

## Key Data Types and Enums

*   **`Amount`**: Struct with `currency` (Hyperswitch `storage_enums::Currency`) and `value` (`MinorUnit`).
*   **`Address`**: Adyen's address structure with fields like `city`, `country`, `house_number_or_name`, `postal_code`.
*   **`AdyenStatus`**: Enum for Adyen's synchronous payment statuses.
*   **`AdyenWebhookStatus`**: Enum for statuses derived from webhook events.
*   **`PaymentType`**: Adyen's enum for various payment method identifiers (e.g., `Scheme`, `Klarna`, `Ideal`, `SepaDirectDebit`).
*   **`CardBrand`**: Adyen's enum for card brands (e.g., `Visa`, `MC`, `Amex`).
*   **`OnlineBanking*` enums**: Specific bank issuers for online banking methods in various countries (e.g., `OnlineBankingCzechRepublicBanks`, `OnlineBankingPolandBanks`).
*   **`RiskData`**: A struct to hold various risk-related fields that can be sent in `additional_data`.
*   **`AdyenSplitData`**: Represents an item in a split payment request/response.

This extensive set of transformations allows Hyperswitch to interface with Adyen's comprehensive API for a wide range of payment operations.

---

# Adyenplatform Connector Type Mappings

This document outlines the key data types and transformations used in the Adyenplatform connector, primarily focusing on payouts. It leverages some existing types from the main Adyen connector.

## Authentication and General Structure

*   **`AdyenplatformAuthType`**: Uses `ConnectorAuthType::HeaderKey { api_key }` for authentication.
*   **`AdyenPlatformRouterData<T>`**: A generic wrapper for router data, pairing it with an `amount` of type `MinorUnit`.
*   **`AdyenPlatformConnectorMetadataObject`**: Parsed from connector metadata, it can contain `source_balance_account: Option<Secret<String>>`, which is crucial for specifying the source of funds for payouts.

## Payout Request Transformation (Bank Transfers)

The primary focus is on transforming Hyperswitch's `PayoutsRouterData` into an `AdyenTransferRequest` for bank payouts. Card and Wallet payouts are currently not implemented for Adyenplatform.

*   **`AdyenTransferRequest`**: The main request payload for initiating a transfer (payout).
    *   `amount: adyen::Amount`: Reuses the `Amount` struct (value and currency) from the main Adyen connector.
    *   `balance_account_id: Secret<String>`: The ID of the source balance account from which the funds will be transferred. This is typically configured in the connector metadata.
    *   `category: AdyenPayoutMethod`: Specifies the payout method category. Currently, only `Bank` is supported.
    *   `counterparty: AdyenPayoutMethodDetails`: Contains details of the recipient's bank account.
    *   `priority: AdyenPayoutPriority`: Enum indicating the desired speed of the payout (e.g., `Instant`, `Fast`, `Regular`). This is derived from `request.priority`.
    *   `reference: String`: Hyperswitch's `connector_request_reference_id`.
    *   `reference_for_beneficiary: String`: Hyperswitch's `payout_id`, visible to the recipient.
    *   `description: Option<String>`: Optional description for the payout.

*   **`AdyenPayoutMethodDetails`**:
    *   `bank_account: AdyenBankAccountDetails`: Contains the recipient's bank account information.

*   **`AdyenBankAccountDetails`**:
    *   `account_holder: AdyenBankAccountHolder`: Details of the bank account holder.
    *   `account_identification: AdyenBankAccountIdentification`: Specifies the type and details of the bank account identifier (e.g., IBAN for SEPA).

*   **`AdyenBankAccountHolder`**:
    *   `address: Option<adyen::Address>`: Recipient's address, reusing the `Address` struct from the main Adyen connector.
    *   `full_name: Secret<String>`: Recipient's full name.
    *   `customer_id: Option<String>`: Hyperswitch's customer ID.
    *   `entity_type: Option<EntityType>`: Enum (`Individual`, `Organization`, `Unknown`) derived from `request.entity_type`.

*   **`AdyenBankAccountIdentification`**:
    *   `bank_type: String`: The type of bank identifier, e.g., "iban".
    *   `account_details: AdyenBankAccountIdentificationDetails`: An enum holding specific account details. Currently, only `Sepa(SepaDetails)` is shown.

*   **`SepaDetails`**:
    *   `iban: Secret<String>`: The recipient's IBAN for SEPA transfers.

*   **Transformation Logic**:
    *   The `TryFrom` implementation for `AdyenTransferRequest` takes `AdyenPlatformRouterData<&types::PayoutsRouterData<F>>`.
    *   It expects `PayoutMethodData::Bank` and specifically `payouts::Bank::Sepa`. ACH and BACS are not supported for Adyenplatform payouts.
    *   `source_balance_account` is retrieved from connector metadata.
    *   `priority` is mapped from Hyperswitch's `PayoutSendPriority`.

## Payout Response Transformation

Adyenplatform's `AdyenTransferResponse` is converted back into Hyperswitch's `types::PayoutsRouterData<F>`.

*   **`AdyenTransferResponse`**:
    *   `id: String`: The connector's unique identifier for the transfer (payout).
    *   `status: AdyenTransferStatus`: Adyenplatform's status for the transfer (e.g., `Authorised`, `Refused`, `Error`).
    *   `reason: String`: A reason string, particularly relevant if the status is `Refused` or `Error`.
    *   Other fields include `amount`, `account_holder` details, `balance_account` details, `category`, `reference`, etc.

*   **Status Mapping (`AdyenTransferStatus` to `enums::PayoutStatus`)**:
    *   `Authorised` -> `Initiated` (indicating the payout has been successfully initiated by Adyenplatform).
    *   `Refused` -> `Ineligible` (payout was refused, often with a reason in the `reason` field).
    *   `Error` -> `Failed` (an error occurred during processing).
*   The `id` from `AdyenTransferResponse` becomes the `connector_payout_id`.
*   If the status maps to `Ineligible`, the `reason` from Adyenplatform is populated into `error_code`.

## Webhook Transformation (Payouts)

Adyenplatform uses webhooks to provide updates on payout status.

*   **`AdyenplatformIncomingWebhook`**: The top-level structure for incoming webhooks.
    *   `data: AdyenplatformIncomingWebhookData`: Contains the core webhook information.
    *   `webhook_type: AdyenplatformWebhookEventType`: An enum indicating the type of event (e.g., `PayoutCreated`, `PayoutUpdated`).

*   **`AdyenplatformIncomingWebhookData`**:
    *   `status: AdyenplatformWebhookStatus`: The status of the payout according to the webhook (e.g., `Authorised`, `Booked`, `Pending`, `Failed`, `Returned`, `Received`).
    *   `reference: String`: The Hyperswitch `connector_request_reference_id` for the payout.
    *   `tracking: Option<AdyenplatformInstantStatus>`: Optional field for instant payouts, providing finer-grained status like `Credited` or `Pending` and `estimated_arrival_time`.

*   **Event Mapping (`AdyenplatformWebhookEventType`, `AdyenplatformWebhookStatus`, `AdyenplatformInstantStatus` to `webhooks::IncomingWebhookEvent`)**:
    *   `PayoutCreated` event type generally maps to `webhooks::IncomingWebhookEvent::PayoutCreated`.
    *   `PayoutUpdated` event type:
        *   If `tracking.status` is `Credited` or `tracking.estimated_arrival_time` is present, it maps to `webhooks::IncomingWebhookEvent::PayoutSuccess`.
        *   Otherwise, based on `AdyenplatformWebhookStatus`:
            *   `Authorised`, `Booked`, `Received` -> `PayoutCreated`.
            *   `Pending` -> `PayoutProcessing`.
            *   `Failed` -> `PayoutFailure`.
            *   `Returned` -> `PayoutReversed`.

## Key Data Types and Enums (Adyenplatform Specific for Payouts)

*   **`AdyenPayoutMethod`**: Enum, currently only `Bank`.
*   **`AdyenPayoutPriority`**: Enum for payout speed (`Instant`, `Fast`, `Regular`, `Wire`, `CrossBorder`, `Internal`).
*   **`EntityType`**: Enum for account holder type (`Individual`, `Organization`, `Unknown`).
*   **`AdyenTransferStatus`**: Enum for synchronous payout response status (`Authorised`, `Refused`, `Error`).
*   **`AdyenplatformWebhookEventType`**: Enum for webhook event types (`PayoutCreated`, `PayoutUpdated`).
*   **`AdyenplatformWebhookStatus`**: Enum for payout statuses received via webhook.
*   **`AdyenplatformInstantStatus`**: Nested struct within webhook data for instant payout tracking.

This connector primarily focuses on SEPA payouts via Adyen's platform capabilities, distinct from the main Adyen connector's payment processing.

---

# Airwallex Connector Type Mappings

This document outlines the key data types and transformations used in the Airwallex connector's `transformers.rs` file. It details how Hyperswitch's generic payment models are mapped to Airwallex-specific request and response formats for payments, refunds, and authentication, with a focus on card payments.

## Authentication

*   **`AirwallexAuthType`**: Extracts `x_api_key` and `x_client_id` from Hyperswitch's `ConnectorAuthType::BodyKey`. These are used as headers for API calls.
*   **Access Token Management**: Airwallex uses bearer tokens for payment processing.
    *   An initial request is made to obtain an access token.
    *   `AirwallexAuthUpdateResponse` (containing `token` and `expires_at`) is transformed into Hyperswitch's `AccessToken` model.

## Core Request Transformation (Payments)

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

## 3DS Continue Transformation

*   For 3DS continuation flows, `PaymentsCompleteAuthorizeRouterData` is transformed into `AirwallexCompleteRequest`.
    *   `request_id`: A new UUID.
    *   `three_ds: AirwallexThreeDsData`: Contains `acs_response` (the payload from the 3DS redirect).
    *   `three_ds_type: AirwallexThreeDsType::ThreeDSContinue`.

## Core Response Transformation (Payments)

Airwallex's payment responses (`AirwallexPaymentsResponse` or `AirwallexPaymentsSyncResponse`) are converted into Hyperswitch's `RouterData<F, T, PaymentsResponseData>`.

*   **`AirwallexPaymentsResponse` / `AirwallexPaymentsSyncResponse`**:
    *   `status: AirwallexPaymentStatus`: Airwallex's status enum.
    *   `id: String`: Connector's transaction ID (PaymentIntent ID).
    *   `payment_consent_id: Option<Secret<String>>`: ID of any payment consent created.
    *   `next_action: Option<AirwallexPaymentsNextAction>`: Contains details for redirection if required.

*   **Status Mapping (`AirwallexPaymentStatus` to `enums::AttemptStatus`)**:
    *   `Succeeded` -> `Charged`.
    *   `Failed` -> `Failure`.
    *   `Pending` -> `Pending`.
    *   `RequiresPaymentMethod` -> `PaymentMethodAwaited`.
    *   `RequiresCustomerAction`:
        *   If `next_action.stage` is `WaitingDeviceDataCollection` -> `DeviceDataCollectionPending`.
        *   If `next_action.stage` is `WaitingUserInfoInput` -> `AuthenticationPending`.
        *   If no `next_action` or stage, defaults to `AuthenticationPending`.
        *   Special handling to prevent infinite loops if Airwallex returns `RequiresCustomerAction` when Hyperswitch is already in `AuthenticationPending` or `DeviceDataCollectionPending`.
    *   `RequiresCapture` -> `Authorized`.
    *   `Cancelled` -> `Voided`.

*   **Redirection (`AirwallexPaymentsNextAction`)**:
    *   `url: Url`, `method: Method`, `data: AirwallexRedirectFormData`, `stage: AirwallexNextActionStage`.
    *   The `data` (containing `JWT`, `threeDSMethodData`, `token`, etc.) and `url`/`method` are used to construct Hyperswitch's `RedirectForm`.

## Capture Transformation

*   `PaymentsCaptureRouterData` is transformed into `AirwallexPaymentsCaptureRequest`.
    *   `request_id`: A new UUID.
    *   `amount: Option<String>`: Amount to capture, converted to base unit string.

## Void (Cancel) Transformation

*   `PaymentsCancelRouterData` is transformed into `AirwallexPaymentsCancelRequest`.
    *   `request_id`: A new UUID.
    *   `cancellation_reason: Option<String>`.

## Refund Transformation

*   **Request**: `AirwallexRouterData<&types::RefundsRouterData<F>>` is transformed into `AirwallexRefundRequest`.
    *   `request_id`: A new UUID.
    *   `amount: Option<String>`: Refund amount as a string.
    *   `reason: Option<String>`.
    *   `payment_intent_id: String`: The connector transaction ID of the original payment.
*   **Response**: `RefundResponse` from Airwallex is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   `status: RefundStatus` (Airwallex's enum: `Succeeded`, `Failed`, `Received`, `Accepted`) is mapped to Hyperswitch's `enums::RefundStatus` (`Success`, `Failure`, `Pending`).

## Webhook Transformation

*   **`AirwallexWebhookData`**: Contains `name` (`AirwallexWebhookEventType`), `data` (`AirwallexObjectData` which holds the actual event object).
*   **Event Mapping (`AirwallexWebhookEventType` to `api_models::webhooks::IncomingWebhookEvent`)**:
    *   `PaymentAttemptFailedToProcess` -> `PaymentIntentFailure`.
    *   `PaymentAttemptAuthorized` -> `PaymentIntentSuccess`.
    *   `RefundSucceeded` -> `RefundSuccess`.
    *   `RefundFailed` -> `RefundFailure`.
    *   Dispute events (`DisputeAccepted`, `DisputeWon`, `DisputeLost`, etc.) are mapped accordingly.
    *   Many other Airwallex events are mapped to `EventNotSupported`.
*   **Dispute Object (`AirwallexDisputeObject`)**: Contains `payment_intent_id`, `dispute_amount`, `dispute_currency`, `stage` (`AirwallexDisputeStage`), `dispute_id`, etc.
*   **Dispute Stage Mapping (`AirwallexDisputeStage` to `api_models::enums::DisputeStage`)**:
    *   `Rfi` -> `PreDispute`.
    *   `Dispute` -> `Dispute`.
    *   `Arbitration` -> `PreArbitration`.

## Key Data Types and Enums (Airwallex Specific)

*   **`AirwallexPaymentStatus`**: Enum for Airwallex payment statuses.
*   **`AirwallexPaymentType`**: Enum, currently `Card` and `Googlepay`.
*   **`AirwallexNextActionStage`**: Enum for stages within a customer action (`WaitingDeviceDataCollection`, `WaitingUserInfoInput`).
*   **`RefundStatus`**: Airwallex's enum for refund statuses.
*   **`AirwallexWebhookEventType`**: Comprehensive enum for various webhook events.
*   **`AirwallexDisputeStage`**: Enum for dispute stages.

This structure allows Hyperswitch to handle Airwallex's API nuances, including its intent-based flow and detailed device data requirements.

---

# Amazonpay Connector Type Mappings

This document outlines the key data types and transformations used in the Amazonpay connector's `transformers.rs` file, focusing on card payments.

## Authentication

*   **`AmazonpayAuthType`**: Extracts `api_key` from Hyperswitch's `ConnectorAuthType::HeaderKey`. This API key is used for authenticating requests.

## Core Request Transformation (Card Payments)

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

## Core Response Transformation (Payments)

Amazonpay's `AmazonpayPaymentsResponse` is converted back into Hyperswitch's `RouterData<F, T, PaymentsResponseData>`.

*   **`AmazonpayPaymentsResponse`**:
    *   `status: AmazonpayPaymentStatus`: Amazonpay's status for the payment.
    *   `id: String`: Connector's transaction ID.

*   **Status Mapping (`AmazonpayPaymentStatus` to `common_enums::AttemptStatus`)**:
    *   `Succeeded` -> `Charged`.
    *   `Failed` -> `Failure`.
    *   `Processing` -> `Authorizing`.

*   The `id` from `AmazonpayPaymentsResponse` becomes the `resource_id` in `PaymentsResponseData::TransactionResponse`.
*   Redirection data and mandate references are currently set to `None`.

## Refund Transformation (Card Payments)

*   **Request**: `AmazonpayRouterData<&RefundsRouterData<F>>` is transformed into `AmazonpayRefundRequest`.
    *   `amount: StringMinorUnit`: The amount to be refunded.
*   **Response**: `RefundResponse` from Amazonpay is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   `status: RefundStatus` (Amazonpay's enum: `Succeeded`, `Failed`, `Processing`) is mapped to Hyperswitch's `enums::RefundStatus` (`Success`, `Failure`, `Pending`).

## Error Handling

*   **`AmazonpayErrorResponse`**:
    *   `status_code: u16`: HTTP status code of the error.
    *   `code: String`: Connector-specific error code.
    *   `message: String`: Error message.
    *   `reason: Option<String>`: Optional reason for the error.

## Key Data Types and Enums (Amazonpay Specific for Card Payments)

*   **`AmazonpayPaymentStatus`**: Enum for Amazonpay payment statuses (`Succeeded`, `Failed`, `Processing`).
*   **`RefundStatus`**: Enum for Amazonpay refund statuses (`Succeeded`, `Failed`, `Processing`).
*   **`StringMinorUnit`**: Used for representing monetary amounts as strings.

This connector implementation for Amazonpay appears to be primarily focused on basic card payment and refund functionalities.

---

# Authorize.Net Connector Type Mappings

This document outlines the key data types and transformations used in the Authorize.Net connector's `transformers.rs` file, focusing on card payments, and also covering mandates (customer profiles) and other supported payment methods like Google Pay, Apple Pay, and PayPal.

## Authentication

*   **`AuthorizedotnetAuthType`**: Extracts `name` (API Login ID) and `transaction_key` from Hyperswitch's `ConnectorAuthType::BodyKey`.

## Core Request Transformation (Payments)

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

## Mandate (Customer Profile) Creation

*   `SetupMandateRouterData` is transformed into `CreateCustomerProfileRequest`.
*   This request creates a customer profile and a payment profile in Authorize.Net.
*   `validation_mode`: `TestMode` or `LiveMode` based on Hyperswitch `test_mode`.
*   The payment profile includes card details or opaque data for wallets.

## Core Response Transformation (Payments)

Authorize.Net's `AuthorizedotnetPaymentsResponse` is converted into Hyperswitch's `RouterData`.

*   **`AuthorizedotnetPaymentsResponse`**:
    *   `transaction_response: Option<TransactionResponse>`: Contains the outcome of the transaction.
        *   **`TransactionResponse::AuthorizedotnetTransactionResponse(Box<AuthorizedotnetTransactionResponse>)`**:
            *   `response_code: AuthorizedotnetPaymentStatus`.
            *   `transaction_id: String` (Connector's transaction ID).
            *   `network_trans_id: Option<Secret<String>>`.
            *   `account_number: Option<Secret<String>>` (masked card number, stored in metadata).
            *   `errors: Option<Vec<ErrorMessage>>`.
            *   `secure_acceptance: Option<SecureAcceptance>` (for 3DS, contains `secure_acceptance_url`).
        *   **`TransactionResponse::AuthorizedotnetTransactionResponseError`**: For certain error structures.
    *   `profile_response: Option<AuthorizedotnetNonZeroMandateResponse>`: Contains customer profile IDs if a profile was created.
    *   `messages: ResponseMessages`: Contains `result_code` (`Ok` or `Error`) and a list of `ResponseMessage` (code and text).

*   **Status Mapping (`AuthorizedotnetPaymentStatus` to `enums::AttemptStatus`)**:
    *   `Approved` -> `Charged` (if auto-capture) or `Authorized` (if manual capture).
    *   `Declined`, `Error` -> `Failure`.
    *   `RequiresAction` -> `AuthenticationPending` (typically for 3DS).
    *   `HeldForReview` -> `Pending`.

*   **Redirection**: If `secure_acceptance_url` is present, a `RedirectForm` is created.
*   **Mandate Reference**: If `profile_response` is present, `customer_profile_id` and `customer_payment_profile_id_list` are combined to form the `connector_mandate_id`.
*   **Error Handling**: If `messages.result_code` is `Error`, or if `transaction_response.errors` is present, an `ErrorResponse` is constructed.

## Capture Transformation

*   `PaymentsCaptureRouterData` is transformed into `CancelOrCaptureTransactionRequest` wrapping `AuthorizedotnetPaymentCancelOrCaptureRequest`.
*   `transaction_type` is `priorAuthCaptureTransaction`.
*   `ref_trans_id` is the original connector transaction ID.
*   Response is similar to payment response, status mapped accordingly.

## Void (Cancel) Transformation

*   `PaymentsCancelRouterData` is transformed into `CancelOrCaptureTransactionRequest`.
*   `transaction_type` is `voidTransaction`.
*   `ref_trans_id` is the original connector transaction ID.
*   Response (`AuthorizedotnetVoidResponse`) status (`AuthorizedotnetVoidStatus`) is mapped to `enums::AttemptStatus` (`Voided`, `VoidFailed`, `VoidInitiated`).

## Refund Transformation

*   **Request**: `RefundsRouterData` is transformed into `CreateRefundRequest` wrapping `AuthorizedotnetRefundRequest`.
    *   `transaction_type` is `refundTransaction`.
    *   `payment`: Contains card details (masked number, dummy expiry) retrieved from `connector_metadata` of the original charge.
    *   `refTransId`: Original connector transaction ID.
*   **Response**: `AuthorizedotnetRefundResponse` is mapped to `RefundsRouterData`.
    *   `transaction_response.response_code` (`AuthorizedotnetRefundStatus`) is mapped to `enums::RefundStatus` (`Success`, `Failure`, `Pending`).

## Sync (PSync / RSync) Transformation

*   `PaymentsSyncRouterData` or `RefundsRouterData<RSync>` is transformed into `AuthorizedotnetCreateSyncRequest`.
    *   This request calls the `getTransactionDetailsRequest` API.
*   Response (`AuthorizedotnetSyncResponse` for payments, `AuthorizedotnetRSyncResponse` for refunds) contains `transaction_status` which is mapped to Hyperswitch's `AttemptStatus` or `RefundStatus`.

## Webhook Transformation

*   **`AuthorizedotnetWebhookObjectId`**: Contains `event_type` (`AuthorizedotnetWebhookEvent`) and `payload` (with `id` - the transaction ID).
*   **Event Mapping (`AuthorizedotnetWebhookEvent` to `IncomingWebhookEvent`)**:
    *   Events like `AuthorizationCreated`, `PriorAuthCapture`, `AuthCapCreated`, `CaptureCreated`, `VoidCreated` map to `PaymentIntentSuccess`.
    *   `RefundCreated` maps to `RefundSuccess`.
*   The webhook payload is also used to construct a `SyncStatus` which can update the internal payment/refund status.

## Key Data Types and Enums (Authorize.Net Specific)

*   **`TransactionType`**: Enum for various transaction types (Payment, Authorization, Capture, Refund, Void, etc.).
*   **`PaymentDetails`**: Enum for payment instruments (CreditCard, OpaqueData for wallets, PayPal).
*   **`ProfileDetails`**: Enum for customer profile operations.
*   **`AuthorizedotnetPaymentStatus`**: Enum for synchronous payment response codes.
*   **`AuthorizedotnetRefundStatus`**: Enum for synchronous refund response codes.
*   **`ResponseMessages`, `ResultCode`, `ResponseMessage`**: Structures for handling overall success/failure and detailed messages from the API.
*   **`SyncStatus`, `RSyncStatus`**: Enums for transaction statuses returned by the GetTransactionDetails API.
*   **`AuthorizedotnetWebhookEvent`**: Enum for webhook event types.

This connector supports a variety of operations including direct card payments, wallet payments (Google Pay, Apple Pay), PayPal, customer profile creation and usage, and standard payment lifecycle operations.

---

# Bambora Connector Type Mappings

This document outlines the key data types and transformations used in the Bambora connector's `transformers.rs` file, focusing on card payments and 3D Secure flows.

## Authentication

*   **`BamboraAuthType`**: Constructs an `api_key` for the `Authorization` header. It takes `api_key` (Merchant ID) and `key1` (API Passcode) from Hyperswitch's `ConnectorAuthType::BodyKey`. The format is "Passcode " followed by the Base64 encoded string of "merchant_id:api_passcode".

## Core Request Transformation (Card Payments)

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

## 3DS Continue Transformation

*   `CompleteAuthorizeData` (after 3DS redirect) is transformed into `BamboraThreedsContinueRequest`.
*   `payment_method` is "credit_card".
*   `card_response: CardResponse`: Contains `cres` (the 3DS CRes payload from the redirect).

## Core Response Transformation (Payments)

Bambora's responses (`BamboraResponse`, which can be `NormalTransaction` or `ThreeDsResponse`) are mapped to Hyperswitch's `RouterData`.

*   **`BamboraResponse::NormalTransaction(Box<BamboraPaymentsResponse>)`**:
    *   `id: String`: Connector's transaction ID.
    *   `approved: String` ("1" for approved, "0" for declined).
    *   `message_id: String`, `message: String`: Result message details.
    *   `auth_code: String`.
    *   `card: CardData`: Masked card details, AVS/CVD results.
*   **`BamboraResponse::ThreeDsResponse(Box<Bambora3DsResponse>)`**:
    *   `three_d_session_data: Secret<String>`: Data to be stored in metadata for the continue step.
    *   `contents: String`: HTML content for redirection.
*   **Status Mapping**:
    *   If `approved == "1"`:
        *   `Charged` if auto-capture.
        *   `Authorized` if manual capture.
    *   If `approved != "1"`:
        *   `Failure` if auto-capture.
        *   `AuthorizationFailed` if manual capture.
    *   For `ThreeDsResponse`, status is `AuthenticationPending`.
*   **Redirection**: For `ThreeDsResponse`, `RedirectForm::Html` is created using `contents`.
*   **Metadata**: For `ThreeDsResponse`, `three_d_session_data` is stored in `connector_metadata`.

## Capture Transformation

*   `PaymentsCaptureRouterData` is transformed into `BamboraPaymentsCaptureRequest`.
*   `amount: f64`.
*   `payment_method: PaymentMethod::Card`.
*   Response is `BamboraPaymentsResponse`, status mapped to `Charged` or `Failure`.

## Void (Cancel) Transformation

*   `PaymentsCancelRouterData` is transformed into `BamboraVoidRequest`.
*   `amount: f64` (original authorized amount).
*   Response is `BamboraPaymentsResponse`, status mapped to `Voided` or `VoidFailed`.

## Refund Transformation

*   **Request**: `RefundsRouterData` is transformed into `BamboraRefundRequest`.
    *   `amount: f64`.
*   **Response**: `RefundResponse` (similar structure to `BamboraPaymentsResponse`) is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   If `approved == "1"`, status is `Success`, else `Failure`.

## Sync (PSync / RSync) Transformation

*   PSync uses `BamboraPaymentsResponse`. Status mapping considers auto-capture.
*   RSync uses `RefundResponse`. Status mapping is `Success` or `Failure`.

## Error Handling

*   **`BamboraErrorResponse`**: Contains `code`, `category`, `message`, `reference`, and optional `details` (field-specific errors) or `validation` (card validation errors).

## Key Data Types and Enums (Bambora Specific)

*   **`PaymentMethod`**: Enum, primarily `Card` for this flow.
*   **`BamboraCard`**: Detailed card information including 3DS specifics.
*   **`ThreeDSecure`**: Structure for 3DS parameters.
*   **`BamboraBrowserInfo`**: Detailed browser fingerprint for 3DS.
*   **`AddressData`**: Billing/shipping address structure.
*   `approved` field (string "1" or "0") in responses is key for status determination.

This connector focuses on card payments with support for 3D Secure authentication.

---

# Bambora APAC Connector Type Mappings

This document outlines the key data types and transformations used in the Bambora APAC connector's `transformers.rs` file. This connector uses SOAP XML for its requests and responses and primarily handles card payments, including tokenization for future use (mandates).

## Authentication

*   **`BamboraapacAuthType`**:
    *   Requires `username`, `password`, and `account_number` from Hyperswitch's `ConnectorAuthType::SignatureKey` (where `api_key` maps to username, `api_secret` to password, and `key1` to account number).
    *   These are embedded within the `<Security>` tags in the SOAP XML request body.

## Core Request Transformation (Card Payments & Mandates)

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

*   **`<TokeniseCreditCard>` (for Setup Mandate/Tokenization)**:
    *   Used when `SetupMandateRouterData` is processed.
    *   Contains `CardNumber`, `ExpM`, `ExpY`, `CardHolderName`, `TokeniseAlgorithmID` (set to `2`), and `Security` details.

## Core Response Transformation (Payments & Mandates)

Bambora APAC's SOAP XML responses are parsed into Rust structs.

*   **`BamboraapacPaymentsResponse` (for Payments)**:
    *   Nested structure: `Envelope` -> `Body` -> `SubmitSinglePaymentResponse` -> `SubmitSinglePaymentResult` -> `PaymentResponse`.
    *   **`PaymentResponse`**:
        *   `ResponseCode: u8` (`0` for success, `1` for failure, others for pending/errors).
        *   `Receipt: String`: Connector's transaction ID.
        *   `CreditCardToken: Option<String>`: Token for the card if tokenization was requested.
        *   `DeclinedCode: Option<String>`, `DeclinedMessage: Option<String>`: Error details if declined.

*   **`BamboraapacMandateResponse` (for Setup Mandate)**:
    *   Nested structure: `Envelope` -> `Body` -> `TokeniseCreditCardResponse` -> `TokeniseCreditCardResult` -> `MandateResponseBody`.
    *   **`MandateResponseBody`**:
        *   `ReturnValue: u8` (`0` for success).
        *   `Token: Option<String>`: The generated card token (connector mandate ID).

*   **Status Mapping (Payments)**:
    *   `ResponseCode == 0`:
        *   `Charged` if auto-capture.
        *   `Authorized` if manual capture.
    *   `ResponseCode == 1`: `Failure`.
    *   Other codes: `Pending`.
*   **Status Mapping (Mandates)**:
    *   `ReturnValue == 0`: `Charged` (as it's a successful tokenization, treated like a successful setup).
    *   Otherwise: `Failure`.
*   **Mandate Reference**: For successful payment with tokenization or successful mandate setup, `CreditCardToken` or `Token` is used as `connector_mandate_id`.

## Capture Transformation

*   SOAP request for `<dts:SubmitSingleCapture>`.
*   Contains `Receipt` (original auth transaction ID), `Amount`, and `Security` details.
*   Response (`BamboraapacCaptureResponse`) structure is similar to payment response.
*   Status: `Charged` if `ResponseCode == 0`, else `Failure`.
*   `authorize_id` (original auth receipt) is stored in `connector_metadata`.

## Refund Transformation

*   **Request**: SOAP request for `<dts:SubmitSingleRefund>`.
    *   Contains `CustRef` (refund ID), `Receipt` (original capture/payment transaction ID), `Amount`, and `Security`.
*   **Response**: `BamboraapacRefundsResponse` structure is similar to payment response.
    *   Status: `Success` if `ResponseCode == 0`, `Failure` if `ResponseCode == 1`, else `Pending`.

## Sync (PSync / RSync) Transformation

*   SOAP request for `<dts:QueryTransaction>`.
*   Criteria include `AccountNumber`, date range, and `Receipt` (for PSync) or `CustRef` (for RSync - refund ID).
*   Response (`BamboraapacSyncResponse`) structure is similar to payment response.
*   Status mapping for PSync is similar to authorize response.
*   Status mapping for RSync is similar to refund response.

## Error Handling

*   Failures are typically indicated by `ResponseCode != 0`.
*   `DeclinedCode` and `DeclinedMessage` provide error details.
*   `BamboraapacErrorResponse` struct is defined but primarily used for parsing top-level SOAP fault errors if the XML structure itself is invalid or there's an auth issue before reaching transaction processing.

## Key Data Types and Enums (Bambora APAC Specific)

*   All requests and responses are SOAP XML strings, parsed into nested Rust structs.
*   `ResponseCode: u8` is the primary indicator of success/failure.
*   `TransactionType` (internal mapping to `1` or `2`) for distinguishing Sale vs. Auth.
*   `TokeniseAlgorithmID` (`2`) used for requesting tokenization.

This connector uses a legacy SOAP-based integration, requiring careful construction and parsing of XML payloads.

---

# Bank of America Connector Type Mappings

This document outlines the key data types and transformations used in the Bank of America connector's `transformers.rs` file. It primarily focuses on card payments, including 3DS and mandates (tokenization), and also supports wallet payments like Google Pay, Apple Pay, and Samsung Pay.

## Authentication

*   **`BankOfAmericaAuthType`**: Extracts `api_key`, `merchant_account` (key1), and `api_secret` from Hyperswitch's `ConnectorAuthType::SignatureKey`.

## Core Request Transformation (Payments & Mandates)

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

*   **Mandate/Tokenization**:
    *   If `setup_future_usage` is `OffSession`, `action_list` includes `TokenCreate`, and `action_token_types` include `PaymentInstrument` and `Customer`. `authorization_options` are set for customer-initiated stored credentials.
    *   For payments using a `connector_mandate_id`, `PaymentInformation::MandatePayment` is used.
    *   For zero-auth setup mandates (`SetupMandateRouterData`), the amount is "0".

## Core Response Transformation (Payments & Mandates)

Bank of America's responses (`BankOfAmericaPaymentsResponse` or `BankOfAmericaSetupMandatesResponse`, which are enums themselves) are mapped to Hyperswitch's `RouterData`.

*   **`BankOfAmericaPaymentsResponse` / `BankOfAmericaSetupMandatesResponse`**:
    *   Can be `ClientReferenceInformation(Box<BankOfAmericaClientReferenceResponse>)` for successful/pending responses or `ErrorInformation(Box<BankOfAmericaErrorInformationResponse>)` for top-level errors.
    *   **`BankOfAmericaClientReferenceResponse`**:
        *   `id: String`: Connector's transaction ID.
        *   `status: BankofamericaPaymentStatus`.
        *   `token_information: Option<BankOfAmericaTokenInformation>`: Contains `payment_instrument.id` (connector mandate ID) if a token was created.
        *   `processor_information: Option<ClientProcessorInformation>`: Contains AVS, CVV results, network transaction ID, approval code.
        *   `consumer_authentication_information: Option<ConsumerAuthenticationInformation>`: Contains 3DS results like ECI, CAVV.
        *   `error_information: Option<BankOfAmericaErrorInformation>`: Detailed error if the transaction failed at a lower level.

*   **Status Mapping (`BankofamericaPaymentStatus` to `enums::AttemptStatus`)**:
    *   `Authorized`, `AuthorizedPendingReview`: `Charged` (if auto-capture) or `Authorized`.
    *   `Pending`: `Charged` (if auto-capture) or `Pending`.
    *   `Succeeded`, `Transmitted`: `Charged`.
    *   `Voided`, `Reversed`, `Cancelled`: `Voided`.
    *   `Failed`, `Declined`, `AuthorizedRiskDeclined`, `InvalidRequest`, `Rejected`, `ServerError`: `Failure`.
    *   `PendingAuthentication`: `AuthenticationPending`.
    *   `PendingReview`, `Challenge`, `Accepted`: `Pending`.
    *   For zero-auth mandates, `Authorized` status is mapped to `Charged` to signify successful setup.

*   **Mandate Reference**: If `token_information.payment_instrument.id` is present, it's used as `connector_mandate_id`.
*   **Error Handling**: If the top-level response indicates an error, or if `BankOfAmericaClientReferenceResponse.error_information` is present, or if the mapped status is a failure, an `ErrorResponse` is constructed.

## Capture Transformation

*   `PaymentsCaptureRouterData` is transformed into `BankOfAmericaCaptureRequest`.
*   Includes `order_information` (with amount and currency) and `client_reference_information`.
*   Response is `BankOfAmericaPaymentsResponse`, status mapped accordingly (typically `Charged` or `Failure`).

## Void (Cancel) Transformation

*   `PaymentsCancelRouterData` is transformed into `BankOfAmericaVoidRequest`.
*   Includes `client_reference_information` and `reversal_information` (with amount, currency, and reason).
*   Response is `BankOfAmericaPaymentsResponse`, status mapped accordingly (e.g., `Voided`, `Failure`).

## Refund Transformation

*   **Request**: `RefundsRouterData` is transformed into `BankOfAmericaRefundRequest`.
    *   Includes `order_information` (amount, currency) and `client_reference_information` (refund ID).
*   **Response**: `BankOfAmericaRefundResponse` is mapped to `RefundsRouterData`.
    *   `id: String` becomes `connector_refund_id`.
    *   `status: BankofamericaRefundStatus` is mapped to `enums::RefundStatus` (`Success`, `Failure`, `Pending`).
        *   `TwoZeroOne` status with `PROCESSOR_DECLINED` reason maps to `Failure`.

## Sync (PSync / RSync) Transformation

*   `PaymentsSyncRouterData` or `RefundsRouterData<RSync>` is transformed to request transaction details.
*   Response (`BankOfAmericaTransactionResponse` for PSync, `BankOfAmericaRsyncResponse` for RSync) contains `application_information.status` which is mapped to Hyperswitch's `AttemptStatus` or `RefundStatus`.

## Key Data Types and Enums (Bank of America Specific)

*   **`ProcessingInformation`**: Controls transaction processing aspects like tokenization, capture, and initiator type.
*   **`PaymentInformation`**: Enum for different payment methods (Card, GooglePay, ApplePay, Mandate).
*   **`BankofamericaPaymentStatus` / `BankofamericaRefundStatus`**: Enums for BoA's transaction/refund statuses.
*   **`BankOfAmericaActionsList`, `BankOfAmericaActionsTokenType`**: Used for mandate creation.
*   **`BankOfAmericaPaymentInitiatorTypes`**: e.g., `Customer`.
*   Error responses are detailed, with `ErrorInformation` containing `reason`, `message`, and `details`.

This connector handles a range of payment instruments and flows, including sophisticated mandate and recurring payment scenarios.