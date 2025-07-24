# Bluesnap Connector Audit

## Currency Unit Handling Audit

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |
| Capture | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |
| PSync | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |
| Void | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |
| Refund | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |
| RSync | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |
| Complete Authorize | ✅ | The connector consistently uses `api::CurrencyUnit::Base` and the `convert_amount` utility to handle currency conversion across all payment flows. |

## Status Mapping Audit

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |
| Capture | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |
| PSync | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |
| Void | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |
| Refund | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |
| RSync | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |
| Complete Authorize | ✅ | The connector correctly maps `BluesnapProcessingStatus` and `BluesnapTxnType` to `enums::AttemptStatus` and `enums::RefundStatus`, covering all possible states. |

## Reconcile with Reference ID Audit

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |
| Capture | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |
| PSync | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |
| Void | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |
| Refund | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |
| RSync | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |
| Complete Authorize | ✅ | The connector correctly uses `connector_request_reference_id` for the outgoing request and maps the `transaction_id` from the response to both `connector_transaction_id` and `connector_response_reference_id`. |

## Audit for Handle connector_request_reference_id and connector_response_reference_id

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The connector uses `connector_request_reference_id` to populate the `merchant_transaction_id` in the request. |
| Capture | ✅ | The connector uses `connector_request_reference_id` to populate the `merchant_transaction_id` in the request. |
| PSync | ✅ | PSync can be performed using `connector_request_reference_id` (as `merchant_transaction_id`) when `connector_transaction_id` is not available. |
| Void | ✅ | The connector uses `connector_request_reference_id` to populate the `merchant_transaction_id` in the request. |
| Refund | ✅ | The connector uses `connector_request_reference_id` to populate the `merchant_transaction_id` in the request. |
| RSync | ✅ | RSync can be performed using `connector_request_reference_id` (as `merchant_transaction_id`) in certain scenarios. |
| Complete Authorize | ✅ | The connector uses `connector_request_reference_id` to populate the `merchant_transaction_id` in the request. |

## Audit for unnecessary set_body() calls

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The `set_body()` call is necessary as `get_request_body` returns a request body. |
| Capture | ✅ | The `set_body()` call is necessary as `get_request_body` returns a request body. |
| PSync | ✅ | No `set_body()` call is present, which is correct for a GET request. |
| Void | ✅ | The `set_body()` call is necessary as `get_request_body` returns a request body. |
| Refund | ✅ | The `set_body()` call is necessary as `get_request_body` returns a request body. |
| RSync | ✅ | No `set_body()` call is present, which is correct for a GET request. |
| Complete Authorize | ✅ | The `set_body()` call is necessary as `get_request_body` returns a request body. |

## Amount in Request Body Audit

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The amount is present in the request body. |
| Capture | ✅ | The amount is present in the request body. |
| PSync | N/A | This is a GET request and does not have a request body. |
| Void | ✅ | The request body does not contain an amount, which is correct for a void operation. |
| Refund | ✅ | The amount is present in the request body. |
| RSync | N/A | This is a GET request and does not have a request body. |
| Complete Authorize | ✅ | The amount is present in the request body. |

## Dynamic Fields Audit

| Payment Method | Status | Notes |
| --- | --- | --- |
| Card | ✅ | The required fields for card payments are correctly defined. |
| Google Pay | ✅ | The connector is configured for Google Pay with no specific required fields. |
| Apple Pay | ❌ | No entry for Apple Pay was found in the `payment_connector_required_fields.rs` file. |

## Handling of merchant metadata if metadata is present

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | ✅ | The connector supports sending merchant metadata in the `Authorize` flow. |
| Capture | N/A | Metadata is not used in this flow. |
| PSync | N/A | Metadata is not used in this flow. |
| Void | N/A | Metadata is not used in this flow. |
| Refund | N/A | Metadata is not used in this flow. |
| RSync | N/A | Metadata is not used in this flow. |
| Complete Authorize | ✅ | The connector supports sending merchant metadata in the `Complete Authorize` flow. |

## Config changes Audit

| File | Status |
| --- | --- |
| `config.example.toml` | ✅ |
| `integration_test.toml` | ✅ |
| `production.toml` | ✅ |
| `sandbox.toml` | ✅ |
| `development.toml` | ✅ |
| `docker_compose.toml` | ✅ |

## Endpoint Audit

| File | Endpoint |
| --- | --- |
| `production.toml` | `https://ws.bluesnap.com/` |
| `sandbox.toml` | `https://sandbox.bluesnap.com/` |

## Response Struct Field Value Audit

| Status | Notes |
| --- | --- |
| ✅ | All sensitive fields in the response structs are properly masked using `masking::Secret`. |

## MCA metadata validation

| Feature | Status | Notes |
| --- | --- | --- |
| Authorize | N/A | The connector does not perform MCA metadata validation. |
| Capture | N/A | The connector does not perform MCA metadata validation. |
| PSync | ✅ | The connector uses the `merchant_id` from the connector metadata to build the request URL. |
| Void | N/A | The connector does not perform MCA metadata validation. |
| Refund | N/A | The connector does not perform MCA metadata validation. |
| RSync | ✅ | The connector uses the `merchant_id` from the connector metadata to build the request URL. |
| Complete Authorize | N/A | The connector does not perform MCA metadata validation. |

## Response code Audit

| Feature | Scenario | Status | Notes |
| --- | --- | --- | --- |
| | | | |
