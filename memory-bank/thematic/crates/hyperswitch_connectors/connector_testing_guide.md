---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Connector Testing Guide

---
**Parent:** [Hyperswitch Connectors Overview](./overview.md)  
**Related Files:**
- [Connector Interface Requirements](./connector_interface_guide.md)
- [Connector Implementation Guide](./connector_implementation_guide.md)
- [Connector Configuration Guide](./connector_configuration_guide.md)
---

## Overview

This guide provides comprehensive instructions for testing Hyperswitch connector implementations. It covers different testing approaches, from unit tests to end-to-end integration tests, and provides best practices for ensuring connector reliability and robustness.

## Testing Levels

Connector testing should be performed at multiple levels to ensure thorough coverage:

1. **Unit Tests**: Test individual components of the connector implementation in isolation
2. **Integration Tests**: Test the connector's interaction with the Hyperswitch system
3. **Mock API Tests**: Test connector implementation against mock API responses
4. **Sandbox Tests**: Test connector against the payment processor's sandbox environment
5. **Production Validation**: Validate connector functionality in a controlled production environment

## Unit Testing

Unit tests focus on testing individual components of your connector implementation in isolation, primarily the transformers and utility functions.

### Testing Transformers

Transformers convert between Hyperswitch domain models and connector-specific formats. Unit tests should verify these conversions are correct in both directions.

```rust
// Example unit test for a request transformer
#[test]
fn test_payments_request_transformer() {
    // Create a test router data object
    let router_data = router_data_with_values();
    
    // Transform to connector request
    let connector_request = YourConnectorPaymentsRequest::try_from(&router_data).unwrap();
    
    // Assert expected values
    assert_eq!(connector_request.amount, 1000);
    assert_eq!(connector_request.currency, "USD");
    assert_eq!(connector_request.description, "Test payment");
    // Test other fields...
}

// Example unit test for a response transformer
#[test]
fn test_payments_response_transformer() {
    // Create a test connector response
    let connector_response = YourConnectorPaymentsResponse {
        id: "test_123".to_string(),
        status: YourConnectorPaymentStatus::Success,
        // Other fields...
    };
    
    // Create response router data
    let response_router_data = ResponseRouterData {
        response: connector_response,
        data: router_data_with_values(),
        http_code: 200,
    };
    
    // Transform to router data
    let router_data = RouterData::try_from(response_router_data).unwrap();
    
    // Assert expected values
    assert_eq!(router_data.status, AttemptStatus::Charged);
    // Test other fields...
}
```

### Testing Status Mappings

Test that connector status values map correctly to Hyperswitch status values:

```rust
#[test]
fn test_payment_status_mapping() {
    // Test each possible status value
    assert_eq!(AttemptStatus::from(YourConnectorPaymentStatus::Success), AttemptStatus::Charged);
    assert_eq!(AttemptStatus::from(YourConnectorPaymentStatus::Pending), AttemptStatus::Pending);
    assert_eq!(AttemptStatus::from(YourConnectorPaymentStatus::Failed), AttemptStatus::Failure);
    assert_eq!(AttemptStatus::from(YourConnectorPaymentStatus::Processing), AttemptStatus::Pending);
    assert_eq!(AttemptStatus::from(YourConnectorPaymentStatus::Unknown), AttemptStatus::Pending);
}
```

### Testing Error Handling

Test that connector errors are properly mapped to Hyperswitch errors:

```rust
#[test]
fn test_error_handling() {
    // Create a test error response
    let error_response = YourConnectorErrorResponse {
        error: "invalid_request".to_string(),
        message: "Invalid payment information".to_string(),
    };
    
    // Create a response with error status
    let response = Response {
        status_code: 400,
        response: serde_json::to_value(error_response).unwrap(),
        headers: Default::default(),
        message: Default::default(),
    };
    
    // Call error handling function
    let error = YourConnector.build_error_response(response, None).unwrap();
    
    // Assert expected error values
    assert_eq!(error.status_code, 400);
    assert_eq!(error.message, "Invalid payment information");
}
```

## Integration Testing with Mock Server

Integration tests validate that your connector correctly interacts with the Hyperswitch system and properly handles API responses. Use a mock server to simulate the payment processor's API responses.

### Setting Up a Mock Server

Create a mock server for testing. You can use libraries like `wiremock` for Rust or standalone tools like Mockoon:

```rust
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_connector_payment_authorize() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Mock successful payment response
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "id": "pay_123456789",
                "status": "success",
                // Other response fields...
            }))
        )
        .mount(&mock_server)
        .await;
    
    // Configure connector to use mock server URL
    let connector_config = ConnectorConfig {
        base_url: mock_server.uri(),
        api_key: "test_key".to_string(),
    };
    
    // Create test payment request
    let request = create_test_payment_request();
    
    // Call connector
    let response = YourConnector::authorize_payment(request, &connector_config).await.unwrap();
    
    // Assert expected response
    assert_eq!(response.status, AttemptStatus::Charged);
    // Other assertions...
}
```

### Testing Different Scenarios

Use the mock server to test different scenarios, including success, failure, and edge cases:

```rust
#[tokio::test]
async fn test_connector_payment_failure() {
    // Start a mock server
    let mock_server = MockServer::start().await;
    
    // Mock failure response
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(400)
            .set_body_json(json!({
                "error": "invalid_card",
                "message": "The card was declined",
            }))
        )
        .mount(&mock_server)
        .await;
    
    // Configure connector and make request
    // ...
    
    // Assert expected error response
    // ...
}
```

### Testing Webhook Handling

Test webhook handling with mock webhook requests:

```rust
#[test]
fn test_webhook_parsing() {
    // Create a mock webhook request
    let webhook_body = r#"{
        "id": "evt_123",
        "type": "payment.succeeded",
        "data": {
            "id": "pay_123",
            "status": "success"
        }
    }"#.as_bytes();
    
    let mut headers = HeaderMap::new();
    headers.insert("X-Signature", "test_signature".parse().unwrap());
    
    let request = webhooks::IncomingWebhookRequestDetails {
        body: webhook_body,
        headers: &headers,
        method: "POST",
    };
    
    // Test webhook event type extraction
    let event_type = YourConnector.get_webhook_event_type(&request).unwrap();
    assert_eq!(event_type, api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess);
    
    // Test object reference ID extraction
    let object_reference = YourConnector.get_webhook_object_reference_id(&request).unwrap();
    assert!(matches!(object_reference, 
        api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(ref id)
        ) if id == "pay_123"
    ));
}
```

## Sandbox Testing

Sandbox testing involves testing your connector against the payment processor's actual sandbox environment. This validates that your implementation correctly interacts with the payment processor's API.

### Setting Up Sandbox Credentials

1. Obtain sandbox API credentials from the payment processor
2. Configure the connector with sandbox credentials
3. Update test configuration to use sandbox environment

```rust
#[tokio::test]
#[ignore] // Mark as ignored by default as it needs external API access
async fn test_sandbox_payment_flow() {
    // Initialize connector with sandbox credentials
    let connector = YourConnector;
    let auth_type = ConnectorAuthType::HeaderKey {
        api_key: Secret::new("sandbox_api_key".to_string()),
    };
    
    // Create test request
    let request = create_test_payment_request();
    
    // Execute request
    let authorize_response = execute_authorize(&connector, &request, &auth_type).await.unwrap();
    
    // Get connector transaction ID for sync
    let connector_tx_id = get_connector_transaction_id(&authorize_response);
    
    // Create sync request
    let sync_request = create_sync_request(connector_tx_id);
    
    // Execute sync request
    let sync_response = execute_sync(&connector, &sync_request, &auth_type).await.unwrap();
    
    // Assert expected status
    assert!(matches!(sync_response.status, AttemptStatus::Charged | AttemptStatus::Pending));
}
```

These tests can be run manually or as part of a CI/CD pipeline that has access to sandbox credentials.

### Testing Complete Payment Flows

Test complete payment flows including authorization, synchronization, and potentially webhooks:

1. **Authorization**: Initiate a payment
2. **Synchronization**: Check payment status
3. **Capture**: Capture a previously authorized payment (if applicable)
4. **Refund**: Refund a payment (if applicable)
5. **Webhook**: Verify webhook is received and processed correctly (if applicable)

## End-to-End Testing

End-to-end tests validate the connector's integration with the entire Hyperswitch system. These tests typically run in a staging environment.

### Creating E2E Test Scenarios

Create test scenarios that cover the complete payment lifecycle:

```rust
#[tokio::test]
#[ignore] // Run manually or in specific CI environments
async fn test_e2e_payment_flow() {
    // Initialize Hyperswitch client with test merchant credentials
    let client = HyperswitchClient::new("test_merchant", "test_api_key");
    
    // Create payment with test connector
    let payment_request = PaymentRequest {
        amount: 1000,
        currency: "USD",
        payment_method: PaymentMethodData::Card(test_card_data()),
        connector: "your_connector",
        // Other fields...
    };
    
    // Initiate payment
    let payment = client.payments().create(payment_request).await.unwrap();
    
    // Check initial status
    assert_eq!(payment.status, "pending");
    
    // Retrieve payment to check updated status
    // May need to retry for async operations
    let updated_payment = retry_until_status_changes(client, payment.id).await;
    
    // Verify final status
    assert_eq!(updated_payment.status, "succeeded");
    
    // Verify connector-specific details
    let connector_details = updated_payment.connector_details.unwrap();
    assert_eq!(connector_details.connector, "your_connector");
    assert!(connector_details.transaction_id.is_some());
}
```

### Testing Multiple Payment Scenarios

Create tests for different payment scenarios:

1. **Happy Path**: Successful payment with immediate authorization
2. **Asynchronous Authorization**: Payment that requires additional steps
3. **Declined Payment**: Payment that is declined by the processor
4. **3DS Flow**: Payment that requires 3D Secure authentication
5. **Alternative Payment Methods**: If supported by your connector

## Testing Best Practices

### 1. Test Data Management

- Use constants for test data to ensure consistency
- Create helper functions to generate test data
- Isolate test data for different scenarios

```rust
// Example of test data helpers
fn test_card_data() -> CardData {
    CardData {
        number: "4111111111111111".to_string(),
        expiry_month: "12".to_string(),
        expiry_year: "2030".to_string(),
        cvc: "123".to_string(),
        name: Some("Test User".to_string()),
    }
}

fn router_data_with_values() -> PaymentsAuthorizeRouterData {
    // Create router data with test values
    // ...
}
```

### 2. Error Testing

Test error handling for various scenarios:

- Invalid API credentials
- Invalid payment data
- Network errors
- Timeouts
- Rate limiting
- Unexpected response formats

### 3. Logging and Debugging

Implement comprehensive logging in tests to aid debugging:

```rust
// Set up test logging
fn setup() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_with_logging() {
    setup();
    
    // Test code with log statements
    log::info!("Starting test");
    // ...
    log::info!("Test complete");
}
```

### 4. Test Organization

Organize tests by functionality and type:

```
tests/
├── unit/
│   ├── transformers.rs
│   ├── status_mapping.rs
│   └── error_handling.rs
├── integration/
│   ├── mock_api.rs
│   └── webhook_handling.rs
└── sandbox/
    ├── payment_flows.rs
    └── refund_flows.rs
```

### 5. CI/CD Integration

Integrate tests into your CI/CD pipeline:

- Run unit and mock integration tests on every commit
- Run sandbox tests on a schedule or pre-release
- Run end-to-end tests in staging environment before release

## Test Coverage Requirements

To ensure your connector is robust and reliable, aim for the following test coverage:

1. **Unit Tests**:
   - All transformers and utility functions
   - All status mappings
   - All error handling scenarios
   - 90%+ code coverage

2. **Integration Tests**:
   - All API endpoints (authorize, capture, refund, etc.)
   - All success and error scenarios
   - Webhook processing

3. **Sandbox Tests**:
   - Complete payment flow
   - Refund flow (if applicable)
   - 3DS flow (if applicable)
   - Alternative payment methods (if applicable)

## Troubleshooting Common Issues

### Debugging Transformer Issues

If transformers are not working as expected:

1. Enable debug logging to see the input and output of transformations
2. Compare the generated request with the expected format from the API documentation
3. Use a tool like Postman to manually test the API with the same data

### Debugging Authentication Issues

If authentication is failing:

1. Verify API credentials are correct
2. Check that the authentication header format matches the API requirements
3. Look for any missing or incorrect parameters in the request

### Debugging Webhook Issues

If webhook handling is not working:

1. Verify the webhook signature verification logic
2. Check the format of the webhook payload against the API documentation
3. Use a tool like ngrok to debug webhooks locally

## Conclusion

Comprehensive testing is essential for ensuring your connector is reliable, robust, and maintainable. By following the testing approaches and best practices outlined in this guide, you can build high-quality connectors that seamlessly integrate with Hyperswitch and provide a smooth payment experience for users.

## Next Steps

After testing your connector:

1. Review the [Connector Configuration Guide](./connector_configuration_guide.md) to ensure proper configuration
2. Submit your connector for review and integration
3. Monitor the connector in production and address any issues that arise

## See Also

- [Connector Interface Requirements](./connector_interface_guide.md)
- [Connector Implementation Guide](./connector_implementation_guide.md)
- [Connector Configuration Guide](./connector_configuration_guide.md)