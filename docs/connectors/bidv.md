# BIDV Connector Implementation

BIDV (Bank for Investment and Development of Vietnam) is a Vietnamese state-owned bank offering domestic payment initiation via their Open Banking API. The connector supports two account modes — **corporate/business** (paygate) and **personal/e-wallet** — selected at configuration time.

The connector is split across two files:

- [crates/hyperswitch_connectors/src/connectors/bidv.rs](../crates/hyperswitch_connectors/src/connectors/bidv.rs) — HTTP wiring and flow orchestration
- [crates/hyperswitch_connectors/src/connectors/bidv/transformers.rs](../crates/hyperswitch_connectors/src/connectors/bidv/transformers.rs) — data type conversions

---

## Configuration

### Connector settings (`connectors.bidv` in `config.toml`)

Operator-level config set once per deployment — not per merchant.

| Key | Type | Description |
|-----|------|-------------|
| `base_url` | `String` | Base URL for all BIDV API calls (e.g. `https://openapi.bidv.com.vn/bidv/`) |

### Auth config (`SignatureKey`)

Merchant fills these in when adding the BIDV connector via the dashboard.

| Hyperswitch field | BIDV usage |
|------------------|------------|
| `api_key` | `client_id` — OAuth2 client identifier |
| `api_secret` | `client_secret` — OAuth2 client secret |
| `key1` | `client_certificate` — value sent as `X-Client-Certificate` header |

### Connector metadata

Set per-merchant via the dashboard (passed in `metadata` on each payment request).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `account_type` | Select: `business` / `personal` | Yes | Selects corporate paygate or e-wallet flow |
| `account_number` | Text | Yes | Merchant's BIDV source/debit account number (sent as `appAcct` / `App_Acct` in payment request body) |

---

## `bidv.rs` — Main Connector

### `Bidv` Struct

```rust
pub struct Bidv {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMajorUnit> + Sync),
}
```

Holds an amount converter that converts Hyperswitch's internal minor-unit amounts to BIDV's expected `StringMajorUnit` string format.

---

### `ConnectorCommon`

| Method | Behaviour |
|--------|-----------|
| `id()` | Returns `"bidv"` |
| `base_url()` | Reads `connectors.bidv.base_url` |
| `get_currency_unit()` | `Base` — amounts in major units (e.g. `"10000"` for 10 000 VND) |
| `get_auth_header()` | Returns `[]` — no static credentials; auth is Bearer token per-request |
| `build_headers()` | Adds `Authorization: Bearer <access_token>` (from OAuth2) + `X-Client-Certificate` on every payment/refund request |
| `build_error_response()` | Deserialises BIDV error JSON into Hyperswitch's `ErrorResponse` |

---

### `ConnectorValidation`

- **`validate_mandate_payment`** — Rejects card-based mandates (BIDV doesn't support cards)
- **`validate_psync_reference_id`** — Always returns `Ok(())`

---

### Flow Implementations

#### Supported Flows

| Flow | HTTP | Endpoint | Notes |
|------|------|----------|-------|
| **AccessToken** | `POST` | `{base_url}openapi/oauth2/token` | OAuth2 Client Credentials token exchange |
| **Authorize** | `POST` | `{base_url}open-banking/paygate/inittranscorpgw/v1` | `account_type = "business"` |
| **Authorize** | `POST` | `{base_url}open-banking/ewallet/init-tran/v1` | `account_type = "personal"` |
| **PSync** | `GET` | `{base_url}payment/v1/open-api/query/{txn_id}` | Polls payment status |
| **Refund Execute** | `POST` | `{base_url}payment/v1/open-api/refund` | Initiates a refund |
| **Refund RSync** | `GET` | `{base_url}payment/v1/open-api/refund/query/{txn_id}` | Polls refund status |

#### Unsupported Flows

| Flow | Error |
|------|-------|
| Capture | `NotImplemented` — BIDV is auto-capture |
| Void | `FlowNotSupported` |
| SetupMandate | `FlowNotSupported` |
| Webhooks | `WebhooksNotImplemented` |
| Session / PaymentMethodToken | Empty no-op |

---

### `ConnectorSpecifications` — Static Metadata

```
BIDV_SUPPORTED_PAYMENT_METHODS
  └── BankTransfer / LocalBankTransfer
        ├── Refunds: Supported
        ├── Mandates: NotSupported
        └── Capture: Automatic only

BIDV_CONNECTOR_INFO
  ├── display_name: "BIDV"
  ├── connector_type: PaymentGateway
  └── integration_status: Sandbox

BIDV_SUPPORTED_WEBHOOK_FLOWS: [] (none)
```

---

## `transformers.rs` — Data Shapes

### `BidvAuthType`

Parsed from `ConnectorAuthType::SignatureKey`:

```
api_key    → client_id           (OAuth2 client ID)
api_secret → client_secret       (OAuth2 client secret)
key1       → client_certificate  (X-Client-Certificate header value)
```

`account_type` and `account_number` are read at request time from `req.request.metadata`.

---

### OAuth2 Token — `BidvTokenRequest` / `BidvTokenResponse`

BIDV uses **OAuth2 Client Credentials**. Hyperswitch fetches and caches the token before any payment call.

**Token request** — `POST {base_url}openapi/oauth2/token`

| Where | Value |
|-------|-------|
| Content-Type | `application/x-www-form-urlencoded` |
| `Authorization` header | `Basic base64(client_id:client_secret)` |
| `X-Client-Certificate` header | value from `connectors.bidv.client_certificate` |
| body `grant_type` | `client_credentials` (fixed) |
| body `scope` | `ewallet` (fixed) |

**Token response:**

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiJ9...",
  "expires_in": 3600,
  "token_type": "Bearer",
  "scope": "ewallet"
}
```

The `access_token` is cached and injected as `Authorization: Bearer <token>` on all subsequent requests.

---

### Corporate Payment Request — `BidvCorpPaymentsRequest`

Used when `account_type = "business"`.

**POST** `{base_url}open-banking/paygate/inittranscorpgw/v1`

```json
{
  "body": {
    "serviceId":       "000003",
    "merchantId":      "MERCHANT001",
    "merchantName":    "My Company",
    "channelId":       "WEB",
    "rootRequestId":   "REF9988776655",
    "rootRequestDate": "260518",
    "extraInfo1":      null,
    "extraInfo2":      null,
    "extraInfo3":      null,
    "extraInfo4":      null,
    "extraInfo5":      null
  }
}
```

**Field sources:**

| Field | Source |
|-------|--------|
| `serviceId` | `metadata["service_id"]` (defaults to `"000003"`) |
| `merchantId` | `metadata["merchant_id"]` (required) |
| `merchantName` | `metadata["merchant_name"]` (required) |
| `channelId` | `metadata["channel_id"]` (required) |
| `rootRequestId` | `connector_request_reference_id` |
| `rootRequestDate` | Current UTC date as `YYMMDD` |
| `extraInfo1–5` | `metadata["extra_info1"]` … `metadata["extra_info5"]` (optional) |

**Response — `BidvCorpPaymentsResponse`:**

```json
{
  "msg": {
    "header": {
      "requestId":  12345,
      "errorCode":  "000",
      "errorDesc":  "Success"
    },
    "body": {
      "tranDate":    "20260518",
      "bankTransId": "TXN123456",
      "redirectUrl": "https://payment.bidv.com.vn/redirect/...",
      "extraInfo1":  null
    }
  }
}
```

**Conversion logic:**

- `header.errorCode != "000"` → `ErrorResponse`
- Otherwise → `AuthenticationPending` + redirect to `body.redirectUrl` via `RedirectForm::Form { method: GET }`; `bankTransId` stored as `connector_transaction_id`

---

### Personal (eWallet) Payment Request — `BidvEwalletPaymentsRequest`

Used when `account_type = "personal"`.

**POST** `{base_url}open-banking/ewallet/init-tran/v1`

```json
{
  "body": {
    "serviceId":    "000003",
    "merchantId":   "MERCHANT001",
    "merchantName": "My Company",
    "channelId":    "WEB",
    "Trandate":     "260518",
    "Trans_Id":     "REF9988776655",
    "Trans_Desc":   "Payment for order",
    "Amount":       "10000",
    "Curr":         "VND",
    "Payer_Id":     "0901234567",
    "Payer_Name":   "NGUYEN VAN A",
    "Payer_Addr":   null,
    "Type":         "809",
    "Custmer_Id":   null,
    "Customer_Name":null,
    "IssueDate":    null
  }
}
```

**Field sources:**

| Field | Source |
|-------|--------|
| `serviceId` | `metadata["service_id"]` (defaults to `"000003"`) |
| `merchantId` | `metadata["merchant_id"]` (required) |
| `merchantName` | `metadata["merchant_name"]` (required) |
| `channelId` | `metadata["channel_id"]` (required) |
| `Trandate` | Current UTC date as `YYMMDD` |
| `Trans_Id` | `connector_request_reference_id` |
| `Trans_Desc` | `request.statement_descriptor` (empty if absent) |
| `Amount` | Converted from `minor_amount` |
| `Curr` | `request.currency` |
| `Payer_Id` | `metadata["payer_id"]` (required) |
| `Payer_Name` | `request.customer_name` (required) |
| `Payer_Addr` | `metadata["payer_addr"]` (optional) |
| `Type` | `metadata["transaction_type"]` (defaults to `"809"`) |
| `Custmer_Id` | `metadata["customer_id"]` (optional) |
| `Customer_Name` | `metadata["customer_name"]` (optional) |
| `IssueDate` | `metadata["issue_date"]` (optional) |

**Response — `BidvEwalletPaymentsResponse`:**

```json
{
  "body": {
    "serviceId":   "000003",
    "merchantId":  "MERCHANT001",
    "tranDate":    "260518",
    "errorCode":   "0",
    "errorDesc":   "Success",
    "redirectUrl": "https://payment.bidv.com.vn/ewallet/..."
  },
  "errorResponse": null
}
```

**Conversion logic:**

1. If `errorResponse` is present and contains a non-empty `metadata.status.code` → `ErrorResponse`
2. If `body.errorCode != "0"` and non-empty → `ErrorResponse`
3. Otherwise → `AuthenticationPending` + redirect to `body.redirectUrl`; `body.serviceId` stored as `connector_transaction_id`

---

### Response type detection in `handle_response`

Since `connectors` config is not available in `handle_response`, the corp/ewallet response type is detected by scanning the raw response bytes for the key `"msg"`:

- Contains `"msg"` → parse as `BidvCorpPaymentsResponse`
- Otherwise → parse as `BidvEwalletPaymentsResponse`

---

### Payment Sync Response — `BidvPaymentsSyncResponse`

```json
{
  "responseCode":    "00",
  "responseMessage": "Success",
  "status":          "PAID",
  "transactionId":   "TXN123456",
  "orderId":         "ORD001"
}
```

**Status mapping:**

| BIDV `status` | Hyperswitch `AttemptStatus` |
|---------------|-----------------------------|
| `PAID` | `Charged` |
| `FAILED` | `Failure` |
| `EXPIRED` | `Failure` |
| `CANCELLED` | `Voided` |
| `PENDING` | `AuthenticationPending` |
| _(absent)_ | `AuthenticationPending` |

---

### Refund Request — `BidvRefundRequest`

```json
{
  "transactionId":      "TXN123456",
  "refundAmount":       "10000",
  "refundDescription":  "Customer requested refund"
}
```

### Refund Response — `RefundResponse`

```json
{
  "responseCode":         "00",
  "responseMessage":      "Success",
  "refundTransactionId":  "REFTXN001",
  "status":               "SUCCESS"
}
```

**Refund status mapping:**

| BIDV `status` | Hyperswitch `RefundStatus` |
|---------------|---------------------------|
| `SUCCESS` | `Success` |
| `FAILED` | `Failure` |
| `PENDING` | `Pending` |
| _(absent)_ | `Pending` |

The `connector_refund_id` is set to `refundTransactionId`, falling back to `responseCode` if absent.

---

## Overall Payment Flow

```
0. OAuth2 Client Credentials (fetched once, cached until expires_in)
      POST {base_url}openapi/oauth2/token
      Authorization: Basic base64(client_id:client_secret)
      X-Client-Certificate: <cert>
      body: grant_type=client_credentials&scope=ewallet
      ← { access_token, expires_in }
             ↓
1. Authorize
      POST {base_url}open-banking/paygate/inittranscorpgw/v1   (business)
      POST {base_url}open-banking/ewallet/init-tran/v1          (personal)
      Authorization: Bearer <access_token>
      X-Client-Certificate: <cert>
      → BIDV returns redirectUrl + transaction ID
      → Status: AuthenticationPending
      → Customer is redirected to BIDV payment page
             ↓
2. PSync  (poll until terminal)
      GET {base_url}payment/v1/open-api/query/{txn_id}
      → PAID      → Charged ✓
      → FAILED    → Failure ✗
      → EXPIRED   → Failure ✗
      → CANCELLED → Voided
             ↓
3. (Optional) Refund Execute
      POST {base_url}payment/v1/open-api/refund
             ↓
4. (Optional) Refund RSync
      GET {base_url}payment/v1/open-api/refund/query/{txn_id}
      → SUCCESS → RefundStatus::Success ✓
```

> The connector is **async/polling-based** — payments start at `AuthenticationPending` (customer redirected to BIDV) and advance to a terminal state after the sync endpoint confirms the outcome. There is no webhook support.

