# Router Routes Module

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Core Module](./core.md)
- [Payment Flows](../flows/payment_flows.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The `routes` module defines all the Actix Web API endpoints for the Hyperswitch platform. It serves as the interface between client applications and the core business logic. This module is responsible for handling HTTP requests, validating inputs, interacting with the core business logic, and formatting responses.

## Key Components

### Payments

The `payments` routes sub-module provides endpoints for payment operations:

- **`POST /payments`**: Creates a new payment intent
- **`GET /payments/{payment_id}`**: Retrieves a payment's details
- **`POST /payments/{payment_id}/confirm`**: Confirms a payment intent
- **`POST /payments/{payment_id}/capture`**: Captures an authorized payment
- **`POST /payments/{payment_id}/cancel`**: Cancels/voids a payment
- **`GET /payments`**: Lists payments with filtering options
- **`POST /payments/{payment_id}/sessions`**: Creates payment sessions for client-side integration
- **`POST /payments/sync`**: Synchronizes payment status with the payment processor

### Refunds

The `refunds` routes sub-module handles refund-related endpoints:

- **`POST /refunds`**: Creates a new refund
- **`GET /refunds/{refund_id}`**: Retrieves a refund's details
- **`GET /refunds`**: Lists refunds with filtering options
- **`POST /refunds/sync`**: Synchronizes refund status with the payment processor

### Customers

The `customers` routes sub-module manages customer data:

- **`POST /customers`**: Creates a new customer
- **`GET /customers/{customer_id}`**: Retrieves a customer's details
- **`POST /customers/{customer_id}`**: Updates a customer's information
- **`DELETE /customers/{customer_id}`**: Deletes a customer record

### Payment Methods

The `payment_methods` routes sub-module handles payment method management:

- **`POST /payment_methods`**: Creates/stores a payment method
- **`GET /payment_methods/{payment_method_id}`**: Retrieves a payment method's details
- **`POST /payment_methods/{payment_method_id}`**: Updates a payment method
- **`DELETE /payment_methods/{payment_method_id}`**: Deletes a payment method
- **`GET /customers/{customer_id}/payment_methods`**: Lists a customer's payment methods
- **`POST /customers/{customer_id}/payment_methods`**: Attaches a payment method to a customer
- **`POST /customers/{customer_id}/payment_methods/{payment_method_id}/detach`**: Detaches a payment method from a customer

### Mandates

The `mandates` routes sub-module manages payment mandates:

- **`POST /mandates`**: Creates a mandate for recurring payments
- **`GET /mandates/{mandate_id}`**: Retrieves a mandate's details
- **`POST /mandates/{mandate_id}/revoke`**: Revokes an active mandate

### Webhooks

The `webhooks` routes sub-module handles webhook endpoints:

- **`POST /webhooks/{connector_name}`**: Receives incoming webhooks from payment processors
- **`POST /webhooks/{connector_name}/verify`**: Verifies webhook configuration

### Health

The `health` routes provide endpoints for service health monitoring:

- **`GET /health`**: Basic health check endpoint
- **`GET /health/ready`**: Readiness check for the service
- **`GET /health/live`**: Liveness check for the service

### Merchant Configuration

The `merchants` routes handle merchant configuration:

- **`POST /merchants`**: Creates a new merchant account
- **`GET /merchants/{merchant_id}`**: Retrieves merchant configuration
- **`POST /merchants/{merchant_id}`**: Updates merchant configuration
- **`POST /merchants/{merchant_id}/connectors`**: Configures payment processor connectors for a merchant
- **`POST /merchants/{merchant_id}/routing`**: Configures routing rules for a merchant
- **`POST /merchants/{merchant_id}/keys`**: Manages API keys for a merchant

## Implementation Details

### Route Registration

Routes are registered with the Actix Web framework during application startup. A typical route definition includes:

1. **Path Pattern**: Defining the URL pattern for the route
2. **HTTP Method**: GET, POST, PUT, DELETE, etc.
3. **Authorization Requirements**: Whether the route requires authentication/authorization
4. **Request Handling Function**: The function that processes the request

### Request Processing

Each route handler typically follows this pattern:

1. **Request Parsing**: Parse and validate the incoming HTTP request
2. **Input Validation**: Validate request parameters and payload
3. **Authentication/Authorization**: Verify the requester has permission to perform the action
4. **Business Logic Invocation**: Call appropriate functions in the core module
5. **Response Formatting**: Format the response according to API standards
6. **Error Handling**: Catch and properly format any errors that occur

### Response Formatting

The routes module ensures consistent response formatting across all endpoints:

- Success responses follow a standard structure
- Error responses include appropriate HTTP status codes and detailed error information
- Pagination is handled consistently for list endpoints
- Serialization respects API version-specific formats

### Middleware Integration

The routes module integrates with various middleware components:

- **Authentication Middleware**: Verifies API keys and merchant permissions
- **Logging Middleware**: Logs request/response details
- **Metrics Middleware**: Collects performance metrics
- **CORS Middleware**: Handles cross-origin requests
- **Rate Limiting Middleware**: Enforces rate limits on API calls

## API Versioning

The routes module supports multiple API versions:

- Routes are prefixed with the API version (e.g., `/v1/payments`, `/v2/payments`)
- Each version can have different request/response formats
- New features are typically introduced in newer API versions
- Older versions are maintained for backward compatibility

## Dependencies

The routes module primarily depends on:

- **Actix Web**: For HTTP server and routing capabilities
- **Core Module**: For business logic implementation
- **API Models Crate**: For request/response data structures
- **Middleware Module**: For cross-cutting concerns

## See Also

- [Core Module Documentation](./core.md)
- [Payment Flows Documentation](../flows/payment_flows.md)
- [Services Module Documentation](./services.md)
