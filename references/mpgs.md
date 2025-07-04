Request and Response Payloads for MPGS Transaction Flows
This document provides detailed request and response payloads for all major transaction flows supported by the Mastercard Payment Gateway Services (MPGS) API, focusing on version 100 as referenced in the provided URL. Due to restricted access to the official documentation, payloads are derived from reliable public sources, including third-party integrations (e.g., Spreedly), developer forums (e.g., Stack Overflow), and general payment gateway practices. These payloads are tailored for integration into Hyperswitch, an open-source payments switch written in Rust, ensuring alignment with its ConnectorCommon and ConnectorIntegration traits. For production-ready integration, developers must verify payloads against the official MPGS API reference, accessible via a Mastercard developer account.
Introduction
MPGS is a global payment gateway facilitating secure card payments across multiple channels, supporting flows like Purchase, Authorize, Capture, Refund, Void, Verify, Tokenization, Payment with Token, 3DS Authentication, and Batch Processing. Each flow involves specific API calls with defined request and response structures, typically in JSON format, using HTTP methods like POST, PUT, or GET. The payloads below are designed for card payment processing, with considerations for PCI compliance and Hyperswitch’s architecture.
Methodology
The payloads were compiled from:

Spreedly Documentation: Provides examples for Purchase, Authorize, Capture, Refund, Void, and Verify flows, including gateway-specific fields.
Stack Overflow: Offers a sample payload for the Pay operation with tokenization.
General Payment Gateway Practices: Used to infer payloads for flows like Batch Processing and Tokenization, where specific examples were unavailable.
Mastercard API Snippets: References to operations like CREATE_CHECKOUT_SESSION and batch processing from public sources.

Due to the inability to access the version 100 documentation directly, some payloads are generalized based on earlier versions (e.g., 57, 75) and industry standards. Developers should validate these against the official API reference for accuracy.
Supported Transaction Flows and Payloads
The following sections detail each transaction flow, including the endpoint, HTTP method, request payload, response payload, and notes for Hyperswitch integration. All amounts are in minor units (e.g., 100.00 USD = 10000 in Hyperswitch’s get_currency_unit).
1. Purchase (Pay)

Description: Combines authorization and capture in a single operation for immediate payment processing.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload (Tokenized, from Stack Overflow):{
  "apiOperation": "PAY",
  "order": {
    "amount": 10.55,
    "currency": "HKD"
  },
  "session": {
    "id": "SESSION0002249161342J64341132I3"
  },
  "sourceOfFunds": {
    "token": "5123456709720008",
    "type": "SCHEME_TOKEN",
    "provided": {
      "card": {
        "expiry": {
          "month": "01",
          "year": "39"
        },
        "storedOnFile": "TO_BE_STORED"
      }
    }
  },
  "transaction": {
    "source": "INTERNET"
  },
  "agreement": {
    "id": "m599944354",
    "type": "UNSCHEDULED"
  }
}


Response Payload (Inferred):{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "amount": 10.55,
    "currency": "HKD",
    "id": "ORDER123",
    "status": "CAPTURED",
    "totalAuthorizedAmount": 10.55,
    "totalCapturedAmount": 10.55
  },
  "transaction": {
    "id": "TXN123",
    "type": "PAYMENT",
    "authorizationCode": "123456"
  },
  "response": {
    "gatewayCode": "APPROVED",
    "acquirerCode": "00"
  }
}


Notes:
Maps to Hyperswitch’s PaymentStatus::Captured.
Use session.id for non-PCI compliant integrations via Hosted Session.
Validate order.amount matches response to ensure data integrity.



2. Authorize

Description: Reserves funds without capturing them, allowing later capture.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload (PCI-Compliant):{
  "apiOperation": "AUTHORIZE",
  "order": {
    "amount": "100.00",
    "currency": "USD"
  },
  "sourceOfFunds": {
    "provided": {
      "card": {
        "number": "4111111111111111",
        "expiry": {
          "month": "12",
          "year": "25"
        },
        "securityCode": "123"
      }
    },
    "type": "CARD"
  },
  "transaction": {
    "source": "INTERNET"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "amount": 100.00,
    "currency": "USD",
    "id": "ORDER123",
    "status": "AUTHORIZED"
  },
  "transaction": {
    "id": "TXN123",
    "type": "AUTHORIZATION",
    "authorizationCode": "123456"
  },
  "response": {
    "gatewayCode": "AUTHORIZED",
    "acquirerCode": "00"
  }
}


Notes:
Maps to Hyperswitch’s PaymentStatus::Authorized.
Use session.id instead of sourceOfFunds.provided.card for non-PCI compliant setups.
May trigger 3DS authentication, requiring a redirect.



3. Capture

Description: Settles funds from a previously authorized transaction.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "CAPTURE",
  "transaction": {
    "amount": "100.00",
    "currency": "USD"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "amount": 100.00,
    "currency": "USD",
    "id": "ORDER123",
    "status": "CAPTURED",
    "totalCapturedAmount": 100.00
  },
  "transaction": {
    "id": "TXN123_CAPTURE",
    "type": "CAPTURE"
  },
  "response": {
    "gatewayCode": "APPROVED"
  }
}


Notes:
Use transactionId from the Authorize response.
Supports partial captures by specifying a lower amount.
Maps to Hyperswitch’s PaymentStatus::Captured.



4. Refund

Description: Returns funds to the customer for a captured transaction.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{newTransactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "REFUND",
  "transaction": {
    "amount": "50.00",
    "currency": "USD"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "amount": 100.00,
    "currency": "USD",
    "id": "ORDER123",
    "status": "REFUNDED",
    "totalRefundedAmount": 50.00
  },
  "transaction": {
    "id": "TXN123_REFUND",
    "type": "REFUND"
  },
  "response": {
    "gatewayCode": "APPROVED"
  }
}


Notes:
Requires a unique newTransactionId.
Supports partial refunds.
Maps to Hyperswitch’s PaymentStatus::Refunded.



5. Void

Description: Cancels an authorized transaction before capture.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "VOID",
  "transaction": {
    "amount": "100.00",
    "currency": "USD"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "amount": 100.00,
    "currency": "USD",
    "id": "ORDER123",
    "status": "VOIDED"
  },
  "transaction": {
    "id": "TXN123_VOID",
    "type": "VOID"
  },
  "response": {
    "gatewayCode": "APPROVED"
  }
}


Notes:
Only applicable for non-captured authorizations.
Maps to Hyperswitch’s PaymentStatus::Cancelled.



6. Verify

Description: Validates a card without processing a payment.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "VERIFY",
  "sourceOfFunds": {
    "provided": {
      "card": {
        "number": "4111111111111111",
        "expiry": {
          "month": "12",
          "year": "25"
        },
        "securityCode": "123"
      }
    },
    "type": "CARD"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "id": "ORDER123",
    "status": "VERIFIED"
  },
  "transaction": {
    "id": "TXN123_VERIFY",
    "type": "VERIFY"
  },
  "response": {
    "gatewayCode": "APPROVED"
  }
}


Notes:
Used for card validation without fund reservation.
Maps to Hyperswitch’s PaymentStatus::Verified.



7. Tokenization

Description: Creates a secure token for card details, reducing PCI scope.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/token/{tokenId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "CREATE_TOKEN",
  "sourceOfFunds": {
    "provided": {
      "card": {
        "number": "4111111111111111",
        "expiry": {
          "month": "12",
          "year": "25"
        },
        "securityCode": "123"
      }
    },
    "type": "CARD"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "token": {
    "id": "TOKEN123456789",
    "status": "ACTIVE"
  }
}


Notes:
Use for recurring payments or one-click checkout.
Maps to Hyperswitch’s PaymentMethodToken.



8. Payment with Token

Description: Processes a payment using a stored token.
Endpoint: PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
Method: PUT
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "PAY",
  "order": {
    "amount": "100.00",
    "currency": "USD"
  },
  "sourceOfFunds": {
    "token": "TOKEN123456789",
    "type": "SCHEME_TOKEN"
  },
  "transaction": {
    "source": "INTERNET"
  }
}


Response Payload:{
  "result": "SUCCESS",
  "merchant": "your_merchant_id",
  "order": {
    "amount": 100.00,
    "currency": "USD",
    "id": "ORDER123",
    "status": "CAPTURED"
  },
  "transaction": {
    "id": "TXN123",
    "type": "PAYMENT"
  },
  "response": {
    "gatewayCode": "APPROVED"
  }
}


Notes:
Reduces PCI scope by avoiding raw card data.
Maps to Hyperswitch’s PaymentStatus::Captured.



9. 3DS Authentication

Description: Secures high-risk transactions with 3D Secure authentication.
Endpoint (Session Creation): POST /api/rest/version/100/merchant/{merchantId}/session
Method: POST
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "CREATE_CHECKOUT_SESSION",
  "interaction": {
    "operation": "PURCHASE"
  },
  "order": {
    "currency": "USD",
    "id": "ORDER123",
    "amount": 100.00
  }
}


Response Payload:{
  "result": "SUCCESS",
  "session": {
    "id": "SESSION0002713374821S8232930L51",
    "version": "1.0"
  }
}


Notes:
Use session.id with hosted-session.js for card data collection.
If 3DS is required, the subsequent PAY response includes a redirectUrl:{
  "result": "PENDING",
  "interaction": {
    "redirectUrl": "[invalid url, do not cite]
  }
}


Implement in Hyperswitch’s RedirectablePayment trait.



10. Batch Processing

Description: Processes multiple transactions (e.g., captures, refunds) in a batch.
Endpoint: POST /api/rest/version/100/merchant/{merchantId}/batch
Method: POST
Headers:
Authorization: Basic <base64_encoded_credentials>
Content-Type: application/json
Content-Length: <length_of_payload>


Request Payload:{
  "apiOperation": "BATCH_CAPTURE",
  "batch": {
    "operations": [
      {
        "orderId": "ORDER123",
        "transactionId": "TXN123",
        "amount": "100.00",
        "currency": "USD"
      },
      {
        "orderId": "ORDER456",
        "transactionId": "TXN456",
        "amount": "200.00",
        "currency": "USD"
      }
    ]
  }
}


Response Payload:{
  "result": "SUCCESS",
  "batch": {
    "status": "PROCESSED",
    "operations": [
      {
        "orderId": "ORDER123",
        "status": "CAPTURED"
      },
      {
        "orderId": "ORDER456",
        "status": "CAPTURED"
      }
    ]
  }
}


Notes:
Used for high-volume merchants.
Implement in Hyperswitch’s ConnectorIntegration::batch trait.



Hyperswitch Integration Notes

Connector Architecture:
Implement ConnectorCommon for configuration (e.g., merchant_id, api_base_url).
Implement ConnectorIntegration for flows: pre_processing, payment, capture, refund, void, verify, authentication, batch.
Map MPGS statuses (AUTHORIZED, CAPTURED, REFUNDED, VOIDED, VERIFIED) to Hyperswitch’s PaymentStatus enum.


Edge Cases:
Currency Handling: Use get_currency_unit for minor unit conversion.
3DS Redirects: Handle via RedirectablePayment trait.
Rate Limits: Implement retry logic for 429 errors in Retryable trait.
Timeouts: Resubmit identical requests after 60 seconds if no response.


Testing:
Use sandbox environment ([invalid url, do not cite]) with test card 1234567890123456`.
Test all flows using MPGS’s Postman collection.


Code Example (Rust):use hyperswitch::core::errors;
use hyperswitch::core::payments::{self, PaymentStatus};

pub struct MpgsConnector;

impl payments::ConnectorCommon for MpgsConnector {
    fn get_name(&self) -> &'static str {
        "mpgs"
    }
    fn get_config(&self) -> Result<Config, errors::ConnectorError> {
        Ok(Config {
            merchant_id: env::var("MPGS_MERCHANT_ID")?,
            api_username: env::var("MPGS_API_USERNAME")?,
            api_password: env::var("MPGS_API_PASSWORD")?,
            api_base_url: env::var("MPGS_API_BASE_URL")?,
            api_version: "100".to_string(),
            webhook_secret: env::var("MPGS_WEBHOOK_SECRET")?,
        })
    }
}

impl payments::ConnectorIntegration for MpgsConnector {
    fn payment(&self, req: payments::PaymentRequest) -> Result<payments::PaymentResponse, errors::ConnectorError> {
        // Implement PAY logic with tokenized card details
        Ok(payments::PaymentResponse {
            status: PaymentStatus::Captured,
            ..Default::default()
        })
    }
}



Missing Information and Follow-Up Actions

Exact Payloads for Version 100: Payloads are inferred from earlier versions and third-party sources. Obtain the official API reference from `[invalid url, do not cite].
Test Cards for Errors: Request test cards for declined transactions and specific errors from MPGS support.
Batch Processing Details: Confirm endpoint and limitations for batch operations.
3DS Support: Verify 3DS integration details for version 100.
Action:
Register for a Mastercard developer account to access the full API reference.
Join Hyperswitch community (Slack/GitHub) for implementation support.
Contact Mastercard support for region-specific details and test data.



References

[Spreedly MPGS Guide]([invalid url, do not cite])
[Stack Overflow: MPGS Pay with Token]([invalid url, do not cite])
[Mastercard API Reference]([invalid url, do not cite])
[MPGS Integration Guides]([invalid url, do not cite])
