---
title: API Index
last_updated: 2025-05-27
position: 1
---

# API Index

This index catalogs all APIs, interfaces, and endpoints documented in the Hyperswitch Memory Bank, providing a comprehensive reference for developers working with the system.

## Public REST APIs

### Merchant APIs

#### Payment APIs
- `POST /payments` - Create a new payment
- `GET /payments/{payment_id}` - Retrieve a payment by ID
- `POST /payments/{payment_id}/captures` - Capture an authorized payment
- `POST /payments/{payment_id}/cancels` - Cancel a payment
- `GET /payments` - List payments
- **Documentation**: [Payment Flows](../thematic/crates/router/flows/payment_flows.md)

#### Refund APIs
- `POST /refunds` - Create a new refund
- `GET /refunds/{refund_id}` - Retrieve a refund by ID
- `GET /refunds` - List refunds
- **Documentation**: [Refund Flows](../thematic/crates/router/flows/refund_flows.md)

#### Payment Method APIs
- `POST /payment_methods` - Create a new payment method
- `GET /payment_methods/{payment_method_id}` - Retrieve a payment method
- `DELETE /payment_methods/{payment_method_id}` - Delete a payment method
- **Documentation**: [Payment Methods](../thematic/crates/payment_methods/overview.md)

#### Customer APIs
- `POST /customers` - Create a new customer
- `GET /customers/{customer_id}` - Retrieve a customer
- `POST /customers/{customer_id}` - Update a customer
- `DELETE /customers/{customer_id}` - Delete a customer
- **Documentation**: [Router Routes](../thematic/crates/router/modules/routes.md)

#### Merchant Configuration APIs
- `POST /merchant_connectors` - Create a merchant connector
- `GET /merchant_connectors` - List merchant connectors
- `DELETE /merchant_connectors/{merchant_connector_id}` - Delete a merchant connector
- **Documentation**: [Router Routes](../thematic/crates/router/modules/routes.md)

### Admin APIs

#### Merchant Management
- `POST /merchants` - Create a merchant account
- `GET /merchants/{merchant_id}` - Retrieve a merchant account
- `POST /merchants/{merchant_id}` - Update a merchant account
- **Documentation**: [Router Routes](../thematic/crates/router/modules/routes.md)

#### Connector Management
- `POST /connectors` - Create/register a connector
- `GET /connectors` - List available connectors
- **Documentation**: [Router Routes](../thematic/crates/router/modules/routes.md)

#### Analytics APIs
- `GET /analytics/payments` - Payment analytics
- `GET /analytics/refunds` - Refund analytics
- **Documentation**: [Analytics](../thematic/crates/analytics/overview.md)

### Webhook APIs
- `POST /webhooks/{connector_name}` - Receive webhooks from payment processors
- **Documentation**: [Webhook Flows](../thematic/crates/router/flows/webhook_flows.md)

## Internal APIs

### Scheduler APIs
- Task creation and management APIs
- **Documentation**: [Scheduler](../thematic/crates/scheduler/overview.md)

### Health Check APIs
- `GET /health` - System health check
- **Documentation**: [Router Routes](../thematic/crates/router/modules/routes.md)

## Connector APIs

### Connector Interface
- `trait<Connector>` - Base connector trait
- `trait<ConnectorCommon>` - Common connector functionality
- `trait<PaymentAuthorize>` - Payment authorization interface
- `trait<PaymentSync>` - Payment synchronization interface
- `trait<PaymentCapture>` - Payment capture interface
- `trait<PaymentVoid>` - Payment void/cancel interface
- `trait<Refund>` - Refund interface
- `trait<RefundSync>` - Refund synchronization interface
- `trait<IncomingWebhook>` - Webhook handling interface
- **Documentation**: [Hyperswitch Interfaces](../thematic/crates/hyperswitch_interfaces/overview.md), [Connector Integration](../thematic/crates/hyperswitch_interfaces/connector_integration.md)

### Implemented Connectors
- Adyen
- Stripe
- PayPal
- Checkout.com
- Authorize.net
- [Additional connectors...]
- **Documentation**: [Hyperswitch Connectors](../thematic/crates/hyperswitch_connectors/overview.md)

## OpenAPI Specification

The complete OpenAPI specification for Hyperswitch API is maintained in the OpenAPI crate.

- **OpenAPI JSON/YAML**: Generated API specification
- **API Models**: [API Models](../thematic/crates/api_models/overview.md)
- **API Documentation Generation**: [OpenAPI](../thematic/crates/openapi/overview.md)

## API Request Models

### Payment Models
- `PaymentsRequest` - Payment creation request
- `PaymentsResponse` - Payment creation response
- `PaymentsCaptureRequest` - Payment capture request
- `PaymentsCaptureResponse` - Payment capture response
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

### Refund Models
- `RefundsRequest` - Refund creation request
- `RefundsResponse` - Refund creation response
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

### Customer Models
- `CustomerRequest` - Customer creation request
- `CustomerResponse` - Customer creation response
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

### Payment Method Models
- `PaymentMethodCreate` - Payment method creation request
- `PaymentMethodResponse` - Payment method creation response
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

### Webhook Models
- `IncomingWebhookRequestDetails` - Incoming webhook request
- `OutgoingWebhook` - Outgoing webhook notification
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

## API Response Handling

### Error Responses
- `ErrorResponse` - Standard error response format
- `ApiErrorResponse` - API-specific error response
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

### Pagination
- `PaginationParams` - Pagination request parameters
- `PaginatedResponse` - Paginated response wrapper
- **Documentation**: [API Models](../thematic/crates/api_models/overview.md)

## API Security

### Authentication Methods
- API Key Authentication
- JWT Token Authentication
- OAuth Authentication
- **Documentation**: [Router Middleware](../thematic/crates/router/modules/middleware.md)

### API Access Control
- Role-based access control
- Permission-based access control
- **Documentation**: [Router Middleware](../thematic/crates/router/modules/middleware.md)

## API Versioning

Information about API versioning strategy and backward compatibility.

- Version management
- Deprecation policies
- Migration guides
- **Documentation**: [Router Overview](../thematic/crates/router/overview.md)

## API Integration Guides

- Merchant integration guide
- Connector integration guide
- Webhook integration guide
- **Documentation**: [Hyperswitch Interfaces](../thematic/crates/hyperswitch_interfaces/connector_integration.md)

## API Testing

- API test utilities
- Mock server implementation
- Test fixtures
- **Documentation**: [Test Utils](../thematic/crates/test_utils/overview.md)

## Related Resources

- [Global Topic Index](./global_topic_index.md) - Complete topic index
- [Crate Functionality Index](./crate_functionality_index.md) - Crate index by functionality
- [Pattern Index](./pattern_index.md) - Design pattern index
- [Configuration Option Index](./configuration_option_index.md) - Configuration options
