# Airwallex Connector Audit

## Status Mapping Audit

| Feature | Connector Status | Hyperswitch Status | Notes |
| --- | --- | --- | --- |
| Payments | `Succeeded` | `Charged` | |
| Payments | `Failed` | `Failure` | |
| Payments | `Pending` | `Pending` | |
| Payments | `RequiresPaymentMethod` | `PaymentMethodAwaited` | |
| Payments | `RequiresCustomerAction` | `AuthenticationPending` / `DeviceDataCollectionPending` | Status depends on the `next_action` field. |
| Payments | `RequiresCapture` | `Authorized` | |
| Payments | `Cancelled` | `Voided` | |
| Refunds | `Succeeded` | `Success` | |
| Refunds | `Failed` | `Failure` | |
| Refunds | `Received` | `Pending` | |
| Refunds | `Accepted` | `Pending` | |

**Analysis:**

The status mapping for the Airwallex connector is handled in `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`.

*   **Payments:** The `get_payment_status` function maps the `AirwallexPaymentStatus` enum to the `common_enums::AttemptStatus` enum. The mapping is comprehensive and covers all documented Airwallex payment statuses. The logic for `RequiresCustomerAction` correctly differentiates between `AuthenticationPending` and `DeviceDataCollectionPending` based on the `next_action` provided in the response.

*   **Refunds:** The `From<RefundStatus> for enums::RefundStatus` implementation correctly maps the `RefundStatus` enum from Airwallex to the `common_enums::RefundStatus` enum in Hyperswitch.

## Reference ID Reconciliation Audit

| Flow | `connector_request_reference_id` | `connector_transaction_id` | `connector_response_reference_id` | Notes |
| --- | --- | --- | --- | --- |
| Pre-Authorize | ✅ | | | Used as `merchant_order_id` in the intent creation request. |
| Authorize | | ✅ | ✅ | `connector_transaction_id` is the payment intent ID. `connector_response_reference_id` is populated from the response `id`. |
| PSync | | ✅ | ✅ | Uses `connector_transaction_id` to fetch the payment intent. `connector_response_reference_id` is populated from the response `id`. |
| Capture | | ✅ | ✅ | Uses `connector_transaction_id` to capture the payment. `connector_response_reference_id` is populated from the response `id`. |
| Void | | ✅ | ✅ | Uses `connector_transaction_id` to cancel the payment. `connector_response_reference_id` is populated from the response `id`. |
| Refund Execute | | ✅ | | `connector_transaction_id` is used as `payment_intent_id` in the refund request. |
| Refund Sync | | | ✅ | Uses `connector_refund_id` to sync the refund. `connector_response_reference_id` is not applicable here. |

**Analysis:**

The reference ID handling is implemented correctly.

*   **`connector_request_reference_id`**: This is used to populate the `merchant_order_id` during the creation of the payment intent, which aligns with its purpose of being a merchant-side reference.
*   **`connector_transaction_id`**: This is consistently used to refer to the payment intent ID in all subsequent operations (Authorize, PSync, Capture, Void, Refund). This is the correct usage.
*   **`connector_response_reference_id`**: This is populated from the `id` field of the payment response, which is the unique identifier for the transaction from Airwallex. This is also correct.
*   **Refunds**: The `connector_refund_id` is used for refund sync operations, which is the correct identifier for refunds.

## Audit for unnecessary `set_body()` calls

| Flow | `build_request` contains `set_body()`? | `get_request_body` returns a body? | Status | Notes |
| --- | --- | --- | --- | --- |
| Pre-Authorize | ✅ | ✅ | ✅ | |
| Authorize | ✅ | ✅ | ✅ | |
| PSync | ❌ | ❌ | ✅ | `build_request` for PSync does not call `set_body` and there is no `get_request_body` function for PSync, as it's a GET request. |
| Complete Authorize | ✅ | ✅ | ✅ | |
| Capture | ✅ | ✅ | ✅ | |
| Void | ✅ | ✅ | ✅ | |
| Refund Execute | ✅ | ✅ | ✅ | |
| Refund Sync | ❌ | ❌ | ✅ | `build_request` for RSync does not call `set_body` and there is no `get_request_body` function for RSync, as it's a GET request. |

**Analysis:**

The usage of `set_body()` is correct across all flows. For flows that involve sending data in the request body (POST requests), the `build_request` function correctly calls `set_body()` and the corresponding `get_request_body` function returns the request payload. For flows that do not require a request body (GET requests like PSync and RSync), `set_body()` is not called.

## Amount in Request Body Audit

| Flow | Amount Present in Request Body? | Status | Notes |
| --- | --- | --- | --- |
| Pre-Authorize | ✅ | ✅ | The `amount` is included in the `AirwallexIntentRequest`. |
| Authorize | ✅ | ✅ | The `amount` is included in the `AirwallexPaymentsRequest`. |
| PSync | N/A | ✅ | This is a GET request and does not have a request body. |
| Complete Authorize | ❌ | ✅ | The `AirwallexCompleteRequest` does not include the amount. |
| Capture | ✅ | ✅ | The `amount` is included in the `AirwallexPaymentsCaptureRequest`. |
| Void | ❌ | ✅ | The `AirwallexPaymentsCancelRequest` does not include the amount. |
| Refund Execute | ✅ | ✅ | The `amount` is included in the `AirwallexRefundRequest`. |
| Refund Sync | N/A | ✅ | This is a GET request and does not have a request body. |

**Analysis:**

The handling of the amount in the request body is mostly correct.

*   For `Pre-Authorize`, `Authorize`, `Capture`, and `Refund Execute`, the amount is correctly included in the request body.
*   For `PSync` and `Refund Sync`, no amount is sent as they are GET requests, which is correct.
*   For `Complete Authorize` and `Void`, the amount is not included in the request body. This is acceptable as these actions are performed on an existing transaction, and the amount is already known to the connector.

## Currency Unit Handling Audit

| Status | Notes |
| --- | --- |
| ✅ | The connector uses the `Base` currency unit, as defined in the `get_currency_unit` function in `crates/hyperswitch_connectors/src/connectors/airwallex.rs`. The `convert_amount` utility is used to convert the amount to the base unit before sending it to the connector. |

**Analysis:**

The currency unit is handled correctly. The connector specifies `Base` as its currency unit and uses the `convert_amount` utility to ensure that the amount is in the correct format before being sent to the connector.

## Dynamic Fields Audit

| Payment Method | Payment Method Type | Field Name | Root Path | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| Card | Credit | `card_number` | `payment_method_data.card.card_number` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Credit | `card_exp_month` | `payment_method_data.card.card_exp_month` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Credit | `card_exp_year` | `payment_method_data.card.card_exp_year` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Credit | `card_cvc` | `payment_method_data.card.card_cvc` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Debit | `card_number` | `payment_method_data.card.card_number` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Debit | `card_exp_month` | `payment_method_data.card.card_exp_month` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Debit | `card_exp_year` | `payment_method_data.card.card_exp_year` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Card | Debit | `card_cvc` | `payment_method_data.card.card_cvc` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Wallet | GooglePay | `tokenization_data` | `payment_method_data.wallet.tokenization_data` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Wallet | Paypal | `customer_name` / `billing.name` | `customer_name` / `payment_method_data.billing.name` | ✅ | Verified in `payment_connector_required_fields.rs`. The code uses `customer_name` if available, otherwise falls back to `get_billing_full_name()`. |
| Wallet | Paypal | `billing.address.country` | `payment_method_data.billing.address.country` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Wallet | Skrill | `customer_name` / `billing.name` | `customer_name` / `payment_method_data.billing.name` | ✅ | Verified in `payment_connector_required_fields.rs`. The code uses `customer_name` if available, otherwise falls back to `get_billing_full_name()`. |
| Wallet | Skrill | `billing.email` | `payment_method_data.billing.email` | ✅ | Verified in `payment_connector_required_fields.rs` |
| Wallet | Skrill | `billing.address.country` | `payment_method_data.billing.address.country` | ✅ | Verified in `payment_connector_required_fields.rs` |
| PayLater | Klarna | `billing.address.country` | `payment_method_data.billing.address.country` | ✅ | Verified in `payment_connector_required_fields.rs` |
| PayLater | Atome | `billing.phone.number` | `payment_method_data.billing.phone.number` | ✅ | Verified in `payment_connector_required_fields.rs` |
| PayLater | Atome | `billing.phone.country_code` | `payment_method_data.billing.phone.country_code` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankRedirect | Trustly | `billing.name` | `payment_method_data.billing.name` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankRedirect | Trustly | `billing.address.country` | `payment_method_data.billing.address.country` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankRedirect | Blik | `billing.name` | `payment_method_data.billing.name` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankRedirect | Ideal | `bank_name` | `payment_method_data.bank_redirect.ideal.bank_name` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankTransfer | IndonesianBankTransfer | `billing.name` | `payment_method_data.billing.name` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankTransfer | IndonesianBankTransfer | `billing.email` | `payment_method_data.billing.email` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankTransfer | IndonesianBankTransfer | `bank_name` | `payment_method_data.bank_transfer.indonesian_bank_transfer.bank_name` | ✅ | Verified in `payment_connector_required_fields.rs` |
| BankTransfer | IndonesianBankTransfer | `billing.address.country` | `payment_method_data.billing.address.country` | ✅ | Verified in `payment_connector_required_fields.rs` |

**Analysis:**

The dynamic field handling for the Airwallex connector is correctly implemented. The `ConnectorSpecifications` in `airwallex.rs` lists all the supported payment methods. For each of these payment methods, the required fields are correctly defined in `crates/payment_methods/src/configs/payment_connector_required_fields.rs`. The transformers in `airwallex/transformers.rs` correctly populate the request structs from the `RouterData`, and the root paths for these fields are correctly marked as required.

## Handling of merchant metadata

| Flow | Metadata Used? | Status | Notes |
| --- | --- | --- | --- |
| All | ❌ | ✅ | The connector does not use merchant metadata in any of the flows. |

**Analysis:**

The Airwallex connector does not currently use any merchant-specific metadata. The `connector_metadata` field in the `RouterData` is not accessed in the `transformers.rs` file.

## Config changes Audit

| File | Base URL | Status |
| --- | --- | --- |
| `config/config.example.toml` | `airwallex.base_url = "https://api-demo.airwallex.com/"` | ✅ |
| `config/development.toml` | `airwallex.base_url = "https://api-demo.airwallex.com/"` | ✅ |
| `config/docker_compose.toml` | `airwallex.base_url = "https://api-demo.airwallex.com/"` | ✅ |
| `config/deployments/integration_test.toml` | `airwallex.base_url = "https://api-demo.airwallex.com/"` | ✅ |
| `config/deployments/sandbox.toml` | `airwallex.base_url = "https://api-demo.airwallex.com/"` | ✅ |
| `config/deployments/production.toml` | `airwallex.base_url = "https://api.airwallex.com/"` | ✅ |

**Analysis:**

The base URL for Airwallex is correctly configured in all the relevant TOML files. The sandbox/demo URL is used for all non-production environments, and the live URL is used for production.

## Endpoint Audit

| Environment | Endpoint |
| --- | --- |
| Sandbox | `https://api-demo.airwallex.com/` |
| Production | `https://api.airwallex.com/` |

**Analysis:**

The sandbox and production endpoints are correctly configured.

## Check API Contracts

| Payment Method | Supported in Contract | Implemented in Transformer | Status | Notes |
| --- | --- | --- | --- | --- |
| Cards | ✅ | ✅ | ✅ | |
| Google Pay | ✅ | ✅ | ✅ | |
| Skrill | ✅ | ✅ | ✅ | |
| iDEAL | ✅ | ✅ | ✅ | |
| Indonesian Bank Transfer | ✅ | ✅ | ✅ | |
| PayPal | ✅ | ✅ | ✅ | |
| Klarna | ✅ | ✅ | ✅ | |
| Trustly | ✅ | ✅ | ✅ | |
| BLIK | ✅ | ✅ | ✅ | |
| Atome | ✅ | ✅ | ✅ | |

**Analysis:**

The API contracts are correctly implemented. The `transformers.rs` file includes structs and implementations for all the payment methods mentioned in the `airwallex.md` contract file. The required fields for each payment method are also correctly defined and handled in the transformer.

A detailed analysis of the API contract audit is as follows:

**Payment Intent Request (`AirwallexIntentRequest`)**
- `request_id`: Correctly implemented as a `String`.
- `amount`: Correctly implemented as a `StringMajorUnit`.
- `currency`: Correctly implemented as an `enums::Currency`.
- `merchant_order_id`: Correctly implemented as a `String`.
- `return_url`: This field is not present in the `AirwallexIntentRequest` struct, but it is mentioned as a required field in the `airwallex.md` file for some payment methods. This is a discrepancy that should be addressed.
- `order`: This is an optional field, and the implementation correctly handles the `Some` and `None` cases. The `AirwallexOrderData` struct contains `products` and `shipping` information, which are correctly typed.

**Confirm Payment Intent Request (`AirwallexPaymentsRequest`)**
- `request_id`: Correctly implemented as a `String`.
- `payment_method`: This is an enum `AirwallexPaymentMethod` that correctly handles different payment methods like `Card`, `Wallets`, `PayLater`, etc.
- `payment_method_options`: This is an optional enum `AirwallexPaymentOptions` that correctly handles different payment method options.
- `return_url`: Correctly implemented as an optional `String`.
- `device_data`: The `DeviceData` struct correctly implements all the required device-related fields.

**Payment Method Structs**
- **Cards (`AirwallexCard`)**: The `AirwallexCard` struct in `transformers.rs` correctly implements the required fields for card payments as specified in the `airwallex.md` contract. The fields `number`, `expiry_month`, `expiry_year`, and `cvc` are all present and correctly typed.
- **Google Pay (`GooglePayData`)**: The `GooglePayData` struct in `transformers.rs` correctly implements the required fields for Google Pay payments. The `encrypted_payment_token` is correctly handled as a `Secret`.
- **Skrill (`SkrillData`)**: The `SkrillData` struct in `transformers.rs` correctly implements the required fields for Skrill payments. The `shopper_name`, `shopper_email`, and `country_code` are all present and correctly typed.
- **iDEAL (`IdealData`)**: The `IdealData` struct in `transformers.rs` correctly implements the required fields for iDEAL payments. The `bank_name` is correctly handled as an optional field.
- **Indonesian Bank Transfer (`IndonesianBankTransferData`)**: The `IndonesianBankTransferData` struct in `transformers.rs` correctly implements the required fields for Indonesian Bank Transfer payments. The `shopper_name`, `shopper_email`, `bank_name`, and `country_code` are all present and correctly typed.
- **PayPal (`PaypalData`)**: The `PaypalData` struct in `transformers.rs` correctly implements the required fields for PayPal payments. The `shopper_name` and `country_code` are all present and correctly typed.
- **Klarna (`KlarnaData`)**: The `KlarnaData` struct in `transformers.rs` correctly implements the required fields for Klarna payments. The `country_code`, `language`, and `billing` details are all present and correctly typed.
- **Trustly (`TrustlyData`)**: The `TrustlyData` struct in `transformers.rs` correctly implements the required fields for Trustly payments. The `shopper_name` and `country_code` are all present and correctly typed.
- **BLIK (`BlikData`)**: The `BlikData` struct in `transformers.rs` correctly implements the required fields for BLIK payments. The `shopper_name` is correctly handled.
- **Atome (`AtomeData`)**: The `AtomeData` struct in `transformers.rs` correctly implements the required fields for Atome payments. The `shopper_phone` is correctly handled as a `Secret`.

## Response Struct Field Value Audit

| Struct Name | Field Name | Masking Status | Notes |
| --- | --- | --- | --- |
| `AirwallexAuthUpdateResponse` | `token` | ✅ | Masked with `masking::Secret`. |
| `AirwallexPaymentsResponse` | `payment_consent_id` | ✅ | Masked with `masking::Secret`. |
| `AirwallexRedirectResponse` | `payment_consent_id` | ✅ | Masked with `masking::Secret`. |
| `AirwallexPaymentsSyncResponse` | `payment_consent_id` | ✅ | Masked with `masking::Secret`. |
| `AirwallexRedirectFormData` | `jwt` | ✅ | Masked with `masking::Secret`. |
| `AirwallexRedirectFormData` | `three_ds_method_data` | ✅ | Masked with `masking::Secret`. |
| `AirwallexRedirectFormData` | `token` | ✅ | Masked with `masking::Secret`. |
| `AirwallexWebhookObjectResource` | `object` | ✅ | Masked with `masking::Secret`. |

**Analysis:**

All sensitive fields in the response structs are correctly masked using `masking::Secret`. This ensures that sensitive data is not logged or exposed in other parts of the system.

## Response Code Audit

