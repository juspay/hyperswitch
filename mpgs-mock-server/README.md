# MPGS Mock Server

A comprehensive mock implementation of the Mastercard Payment Gateway Services (MPGS) API for testing and development purposes. This server implements the MPGS REST API endpoints with realistic request/response handling based on the official MPGS documentation.

## Features

- **Complete MPGS API Implementation**: Supports all MPGS operations (Authorize, Pay, Capture, Void, Refund, Verify, Disbursement)
- **Authentication**: HTTP Basic Auth with `merchant.{merchantId}:{password}` format
- **Comprehensive Test Cards**: Full test card suite from MPGS specification including regional cards
- **Request Validation**: Complete validation based on MPGS API specification
- **In-Memory Storage**: Tracks orders and transactions for retrieval operations
- **Standardized Types**: Well-defined request/response structures matching MPGS specification
- **Multi-Region Support**: Test cards for different regions (US, UK, Australia, UAE, Nigeria, etc.)
- **3DS Testing**: Support for 3D Secure authentication scenarios

## Quick Start

### Installation

```bash
cd mpgs-mock-server
npm install
```

### Start the Server

```bash
# Development mode with auto-reload
npm run dev

# Production mode
npm start
```

The server will start on `http://localhost:3001` by default.

### Run Tests

```bash
npm test
```

## API Endpoints

### Base URL
```
http://localhost:3001/api/rest/version/100
```

### Health & Documentation
- `GET /health` - Health check
- `GET /api/docs` - API documentation

### Payment Operations
- `PUT /merchant/{merchantId}/order/{orderId}/transaction/{transactionId}` - Process payment operations
- `GET /merchant/{merchantId}/order/{orderId}` - Get order details
- `GET /merchant/{merchantId}/order/{orderId}/transaction/{transactionId}` - Get transaction details

## Authentication

All API endpoints require HTTP Basic Authentication with the following format:

```
Username: merchant.{merchantId}
Password: {any_password}
```

Example:
```bash
curl -X PUT "http://localhost:3001/api/rest/version/100/merchant/TEST_MERCHANT/order/order-123/transaction/txn-123" \
  -H "Authorization: Basic bWVyY2hhbnQuVEVTVF9NRVJDSEFOVDp0ZXN0cGFzc3dvcmQ=" \
  -H "Content-Type: application/json" \
  -d '{
    "apiOperation": "PAY",
    "order": {
      "amount": "100.00",
      "currency": "USD"
    },
    "sourceOfFunds": {
      "type": "CARD",
      "provided": {
        "card": {
          "number": "4111111111111111",
          "expiry": {
            "month": "12",
            "year": "2025"
          },
          "securityCode": "123"
        }
      }
    }
  }'
```

## Test Card Numbers

The mock server supports comprehensive test card numbers based on the official MPGS specification:

### Standard Test Cards - Success Scenarios
- `5123450000000008` - Mastercard (Success)
- `2223000000000007` - Mastercard (Success)
- `5111111111111118` - Mastercard (Success)
- `4508750015741019` - Visa (Success)
- `4012000033330026` - Visa (Success)
- `30123400000000` - Diners Club (Success)
- `36259600000012` - Diners Club (Success)
- `3528000000000007` - JCB (Success)
- `6011003179988686` - Discover (Success)
- `5000000000000000005` - Maestro (Success)
- `135492354874528` - UATP (Success)

### Decline Scenarios
- `4000000000000002` - Generic decline
- `4000000000000119` - Insufficient funds
- `4000000000000127` - Declined due to CSC
- `4000000000000010` - Declined due to AVS
- `4000000000000259` - Declined - do not contact
- `4000000000000267` - Declined - invalid PIN
- `4000000000000275` - Declined - PIN required
- `4000000000000341` - Referred

### Error Scenarios
- `4000000000000069` - Expired card
- `4000000000000101` - Acquirer system error
- `4000000000000200` - Unspecified failure
- `4000000000000077` - Invalid CSC
- `4000000000000085` - System error
- `4000000000000093` - Not supported

### 3DS Authentication Scenarios
- `4000000000000044` - 3DS authentication required
- `6201089999995464` - UnionPay 3DS enrolled
- `6690109900000010` - Jaywan 3DS enrolled

### Pending Scenarios
- `4000000000000036` - Timed out
- `4000000000000051` - Pending
- `4000000000000184` - Submitted
- `4000000000000201` - Unknown

### Regional Test Cards

#### EFTPOS Australia
- `5555229999999975` - EFTPOS/Mastercard
- `4043409999991437` - EFTPOS/Visa

#### Verve Nigeria
- `5060990580000217499` - Verve
- `5079539999990592` - Verve

#### PayPak
- `2205459999997832` - PayPak
- `2205439999999541` - PayPak

#### UnionPay Non-3DS
- `6214239999999611` - UnionPay Non-3DS
- `6214239999999546` - UnionPay Non-3DS

#### Jaywan UAE Non-3DS
- `6690109000011057` - Jaywan Non-3DS
- `6690109000011065` - Jaywan Non-3DS

## API Operations

### 1. Payment (PAY)

Creates a payment that combines authorization and capture in a single step.

```json
{
  "apiOperation": "PAY",
  "order": {
    "amount": "100.00",
    "currency": "USD"
  },
  "sourceOfFunds": {
    "type": "CARD",
    "provided": {
      "card": {
        "number": "5123450000000008",
        "expiry": {
          "month": "12",
          "year": "2025"
        },
        "securityCode": "123"
      }
    }
  }
}
```

### 2. Authorization (AUTHORIZE)

Authorizes a payment without capturing funds.

```json
{
  "apiOperation": "AUTHORIZE",
  "order": {
    "amount": "100.00",
    "currency": "USD"
  },
  "sourceOfFunds": {
    "type": "CARD",
    "provided": {
      "card": {
        "number": "5123450000000008",
        "expiry": {
          "month": "12",
          "year": "2025"
        },
        "securityCode": "123"
      }
    }
  }
}
```

### 3. Capture (CAPTURE)

Captures funds from a previously authorized payment.

```json
{
  "apiOperation": "CAPTURE",
  "transaction": {
    "amount": "75.00",
    "currency": "USD"
  }
}
```

### 4. Void Operations

Voids a previously authorized payment, captured payment, or refund.

```json
{
  "apiOperation": "VOID_AUTHORIZATION",
  "transaction": {
    "targetTransactionId": "original-auth-txn-id"
  }
}
```

Supported void operations:
- `VOID_AUTHORIZATION` - Void an authorization
- `VOID_PAYMENT` - Void a payment
- `VOID_CAPTURE` - Void a capture
- `VOID_REFUND` - Void a refund

### 5. Refund (REFUND)

Refunds a previously captured payment.

```json
{
  "apiOperation": "REFUND",
  "transaction": {
    "amount": "50.00",
    "currency": "USD"
  }
}
```

### 6. Verify (VERIFY)

Verifies cardholder's account before processing financial transaction.

```json
{
  "apiOperation": "VERIFY",
  "order": {
    "currency": "USD"
  },
  "sourceOfFunds": {
    "type": "CARD",
    "provided": {
      "card": {
        "number": "5123450000000008",
        "expiry": {
          "month": "12",
          "year": "2025"
        },
        "securityCode": "123"
      }
    }
  }
}
```

### 7. Disbursement (DISBURSEMENT)

Pay out funds to a payer (e.g., gaming winnings, credit card bill payment).

```json
{
  "apiOperation": "DISBURSEMENT",
  "disbursementType": "GAMING_WINNINGS",
  "order": {
    "amount": "250.00",
    "currency": "USD"
  },
  "sourceOfFunds": {
    "type": "CARD",
    "provided": {
      "card": {
        "number": "5123450000000008",
        "expiry": {
          "month": "12",
          "year": "2025"
        },
        "securityCode": "123"
      }
    }
  }
}
```

Supported disbursement types:
- `GAMING_WINNINGS` - For gambling transactions (MCC 7995)
- `CREDIT_CARD_BILL_PAYMENT` - For MoneySend transactions (MCC 6536)

## Response Format

### Success Response

```json
{
  "result": "SUCCESS",
  "merchant": "TEST_MERCHANT",
  "order": {
    "id": "order-123",
    "amount": 100.00,
    "currency": "USD",
    "creationTime": "2025-01-22T11:37:06.000Z",
    "lastUpdatedTime": "2025-01-22T11:37:06.000Z",
    "totalAuthorizedAmount": 100.00,
    "totalCapturedAmount": 100.00,
    "totalRefundedAmount": 0.00
  },
  "response": {
    "gatewayCode": "APPROVED"
  },
  "transaction": {
    "id": "txn-123",
    "type": "PAYMENT",
    "amount": 100.00,
    "currency": "USD",
    "acquirer": {
      "id": "TEST_ACQUIRER",
      "merchantId": "TEST_MERCHANT",
      "transactionId": "txn-123"
    }
  },
  "timeOfRecord": "2025-01-22T11:37:06.000Z",
  "version": "100"
}
```

### Error Response

```json
{
  "error": {
    "cause": "INVALID_REQUEST",
    "explanation": "API operation is required",
    "field": "apiOperation",
    "validationType": "MISSING"
  },
  "result": "ERROR"
}
```

## Gateway Codes & Results

The mock server supports the complete MPGS gateway code specification:

### Success Codes
| Gateway Code | Result | Description |
|--------------|--------|-------------|
| `APPROVED` | `SUCCESS` | Transaction approved |
| `APPROVED_AUTO` | `SUCCESS` | Automatically approved by gateway |
| `APPROVED_PENDING_SETTLEMENT` | `SUCCESS` | Approved - pending batch settlement |

### Decline Codes
| Gateway Code | Result | Description |
|--------------|--------|-------------|
| `DECLINED` | `FAILURE` | Transaction declined |
| `DECLINED_AVS` | `FAILURE` | Declined due to address verification |
| `DECLINED_CSC` | `FAILURE` | Declined due to card security code |
| `DECLINED_AVS_CSC` | `FAILURE` | Declined due to address verification and CSC |
| `DECLINED_DO_NOT_CONTACT` | `FAILURE` | Declined - do not contact issuer |
| `DECLINED_INVALID_PIN` | `FAILURE` | Declined due to invalid PIN |
| `DECLINED_PIN_REQUIRED` | `FAILURE` | Declined - PIN required |
| `DECLINED_PAYMENT_PLAN` | `FAILURE` | Declined due to payment plan |
| `INSUFFICIENT_FUNDS` | `FAILURE` | Insufficient funds |
| `EXPIRED_CARD` | `FAILURE` | Expired card |
| `REFERRED` | `FAILURE` | Declined - refer to issuer |

### Error Codes
| Gateway Code | Result | Description |
|--------------|--------|-------------|
| `ACQUIRER_SYSTEM_ERROR` | `FAILURE` | Acquirer system error |
| `SYSTEM_ERROR` | `FAILURE` | Internal system error |
| `UNSPECIFIED_FAILURE` | `FAILURE` | Transaction could not be processed |
| `INVALID_CSC` | `FAILURE` | Invalid card security code |
| `NOT_SUPPORTED` | `FAILURE` | Transaction type not supported |
| `TIMED_OUT` | `FAILURE` | Gateway timed out |

### Pending Codes
| Gateway Code | Result | Description |
|--------------|--------|-------------|
| `PENDING` | `PENDING` | Transaction pending |
| `SUBMITTED` | `PENDING` | Successfully submitted to acquirer |
| `AUTHENTICATION_IN_PROGRESS` | `PENDING` | 3DS authentication required |
| `UNKNOWN` | `UNKNOWN` | Transaction result unknown |

### Special Codes
| Gateway Code | Result | Description |
|--------------|--------|-------------|
| `ABORTED` | `FAILURE` | Transaction aborted by payer |
| `CANCELLED` | `FAILURE` | Transaction cancelled by payer |
| `BLOCKED` | `FAILURE` | Blocked due to risk rules |
| `LOCK_FAILURE` | `FAILURE` | Order locked - another transaction in progress |
| `EXCEEDED_RETRY_LIMIT` | `FAILURE` | Transaction retry limit exceeded |
| `DUPLICATE_BATCH` | `FAILURE` | Declined due to duplicate batch |
| `NOT_ENROLLED_3D_SECURE` | `FAILURE` | Cardholder not enrolled in 3D Secure |
| `AUTHENTICATION_FAILED` | `FAILURE` | Payer authentication failed |

## Project Structure

```
mpgs-mock-server/
├── package.json              # Dependencies and scripts
├── server.js                 # Main server application
├── types/
│   └── index.js             # Request/response type definitions
├── middleware/
│   └── auth.js              # Authentication middleware
├── tests/
│   └── server.test.js       # Comprehensive test suite
└── README.md                # This file
```

## Environment Variables

- `PORT` - Server port (default: 3001)

## Development

### Adding New Test Scenarios

To add new test card scenarios, modify the `TEST_CARDS` object in `middleware/auth.js`:

```javascript
const TEST_CARDS = {
  'your_test_card_number': { 
    behavior: 'success|decline|error|3ds|pending', 
    gatewayCode: 'GATEWAY_CODE' 
  }
};
```

Available behaviors:
- `success` - Successful transaction with `APPROVED` result
- `decline` - Failed transaction with specified gateway code
- `error` - Error response with specified gateway code  
- `3ds` - 3D Secure authentication required
- `pending` - Transaction in pending state

### Adding New API Operations

1. Add the operation to `ApiOperation` enum in `types/index.js`
2. Add corresponding `TransactionType` if needed
3. Update validation logic in `validatePaymentRequest()`
4. Implement the handler function in `server.js`
5. Add routing in the main switch statement
6. Add test cases in `tests/server.test.js`

### Supported API Operations

The mock server implements the complete MPGS API specification:

- `AUTHORIZE` - Authorization transactions
- `PAY` - Payment (combined authorize + capture)
- `CAPTURE` - Capture authorized funds
- `REFUND` - Refund captured funds
- `VOID` - Generic void operation
- `VOID_AUTHORIZATION` - Void authorization
- `VOID_PAYMENT` - Void payment
- `VOID_CAPTURE` - Void capture
- `VOID_REFUND` - Void refund
- `VERIFY` - Card verification
- `DISBURSEMENT` - Fund disbursement

## Integration with Hyperswitch

This mock server is designed to work seamlessly with the Hyperswitch MPGS connector. The request/response formats match exactly with the MPGS connector implementation requirements.

### Configuration

When integrating with Hyperswitch, use the following configuration:

```toml
[connectors.mpgs]
base_url = "http://localhost:3001/api/rest/version/100"
merchant_id = "TEST_MERCHANT"
api_key = "testpassword"
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Support

For issues and questions:
- Check the test suite for usage examples
- Review the API documentation at `/api/docs`
- Consult the MPGS official documentation
