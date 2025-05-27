# Router Core Module

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete  
**Related Files:**
- [Routes Module](./routes.md)
- [Payment Flows](../flows/payment_flows.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The `core` module is the heart of the router crate, containing the essential business logic for payment processing. It implements the domain-specific logic for handling payments, refunds, webhooks, and other critical processes that form the foundation of the payment orchestration platform.

## Key Components

### Payments

The `payments` sub-module implements the central payment processing functionality:

- **Processing Flows**: Implements complete payment lifecycles including:
  - Payment authorization
  - Payment capture
  - Payment sync (status polling)
  - Payment confirmation
  - Payment cancellation/void
  
- **Operations**: Contains functions for:
  - Creating payment intents
  - Updating payment details
  - Processing payment confirmations
  - Handling payment status transitions
  - Executing payment captures

- **Data Transformations**: 
  - Converts API request models to domain models
  - Transforms domain models to connector-specific formats
  - Normalizes connector responses to domain models

### Payment Methods

The `payment_methods` sub-module handles logic related to different payment methods:

- **Cards**: Processing credit and debit card payments
- **Wallets**: Integration with digital wallets (Apple Pay, Google Pay, etc.)
- **Bank Transfers**: ACH, SEPA, Faster Payments, etc.
- **Buy-Now-Pay-Later (BNPL)**: Affirm, Klarna, AfterPay, etc.
- **Crypto**: Cryptocurrency payment methods
- **Method Selection**: Logic to determine valid payment methods based on merchant configuration

### Webhooks

The `webhooks` sub-module processes asynchronous notifications from payment processors:

- **Incoming Webhooks**: 
  - Validates authenticity and integrity of webhook payloads
  - Processes webhook events (payment status changes, disputes, etc.)
  - Updates payment/refund records based on webhook data
  
- **Outgoing Webhooks**:
  - Generates webhook payloads for merchant notifications
  - Manages webhook delivery and retry logic
  - Tracks webhook delivery status

### Errors

The `errors` sub-module defines custom error types and handling mechanisms:

- **Error Types**: Defines structured error types specific to payment operations
- **Error Mapping**: Maps connector-specific errors to standardized error codes
- **Error Handling**: Provides mechanisms for consistent error handling throughout the system
- **Error Formatting**: Ensures errors are formatted consistently in API responses

### Routing

The `routing` sub-module implements payment routing logic:

- **Strategy Definition**: Defines various routing strategies (cost-based, success-rate, fallback)
- **Rule Management**: Allows creating and managing routing rules
- **Rule Evaluation**: Uses the `euclid` crate's DSL for defining rules and its decision engine for evaluating rules
- **Connector Selection**: Selects appropriate payment processors based on configured rules
- **Retry Logic**: Manages failed payment retry flows with alternative connectors

### Authentication

The `authentication` sub-module handles merchant authentication:

- **API Key Management**: Creation, validation, and rotation of API keys
- **Authentication Flows**: Validates merchant credentials during API requests
- **Session Management**: Handles session state for authenticated users
- **Permission Models**: Defines access control permissions for different API operations

### Mandates

The `mandates` sub-module manages recurring payment mandates:

- **Mandate Creation**: Processes requests to create payment mandates
- **Mandate Storage**: Stores mandate details securely
- **Mandate Usage**: Validates and applies mandates for recurring payments
- **Mandate Lifecycle**: Handles mandate activation, expiration, and revocation

## Implementation Details

The core module is designed with clean separation of concerns:

- Business logic is separated from API handling
- Domain models are used internally for all operations
- Connector-specific details are abstracted away from core business logic
- Errors are handled consistently and mapped to appropriate responses

## Dependencies

The core module interacts with several other crates:

- `hyperswitch_domain_models`: For domain-specific entities and operations
- `storage_impl`: For database access through abstract repositories
- `hyperswitch_connectors`: For payment processor integrations
- `redis_interface`: For caching and distributed operations
- `euclid`: For rule-based routing decisions

## See Also

- [Payment Flows Documentation](../flows/payment_flows.md)
- [Refund Flows Documentation](../flows/refund_flows.md)
- [Webhook Flows Documentation](../flows/webhook_flows.md)
- [Routes Module Documentation](./routes.md)

## Document History

| Date | Changes |
|------|---------|
| 2025-05-27 | Updated metadata to include documentation status |
| 2025-05-20 | Initial version |
