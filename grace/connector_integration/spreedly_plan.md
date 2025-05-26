# Spreedly Connector Integration Implementation Plan

## Initial Setup and Boilerplate Generation
- [x] Step 1: Generate boilerplate code using add_connector script
  - **Task**: Run the add_connector.sh script to generate the initial Spreedly connector structure
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Main connector implementation
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Transformer module
    - `crates/hyperswitch_connectors/src/connectors/spreedly/test.rs`: Test file (to be moved)
  - **Step Dependencies**: None
  - **User Instructions**: Run `./scripts/add_connector.sh spreedly https://core.spreedly.com/v1` from the project root

- [x] Step 2: Move test file to correct location
  - **Task**: Move the generated test file from hyperswitch_connectors to the router tests directory
  - **Files**: 
    - Move from: `crates/hyperswitch_connectors/src/connectors/spreedly/test.rs`
    - Move to: `crates/router/tests/connectors/spreedly.rs`
  - **Step Dependencies**: Step 1
  - **User Instructions**: Use file system operations to move the test file

## Authentication Implementation
- [x] Step 3: Implement Spreedly authentication structure
  - **Task**: Define the authentication type for Spreedly using HTTP Basic Auth with Environment Key and Access Secret
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Add authentication struct and implementation
  - **Step Dependencies**: Step 1
  - **User Instructions**: None

## Request/Response Type Definitions
- [x] Step 4: Define Spreedly authorize request and response types
  - **Task**: Create structs for SpreedlyAuthorizeRequest with credit_card object and SpreedlyAuthorizeResponse with transaction details
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add authorize request/response structs
  - **Step Dependencies**: Step 1
  - **User Instructions**: None

- [x] Step 5: Define capture, refund, and sync types
  - **Task**: Create structs for capture requests/responses, refund requests/responses, and transaction sync responses
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add capture, refund, and sync structs
  - **Step Dependencies**: Step 4
  - **User Instructions**: None

- [x] Step 6: Define error response types
  - **Task**: Create error response structures to handle Spreedly's error format with proper deserialization
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add error response structs
  - **Step Dependencies**: Step 4
  - **User Instructions**: None

## Transformer Implementations
- [x] Step 7: Implement authorize request transformation
  - **Task**: Implement TryFrom trait to convert PaymentsAuthorizeRouterData to SpreedlyAuthorizeRequest, including card data extraction and amount conversion to minor units
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Implement authorize request transformation
  - **Step Dependencies**: Step 4
  - **User Instructions**: None

- [x] Step 8: Implement authorize response transformation
  - **Task**: Implement TryFrom trait to convert SpreedlyAuthorizeResponse to PaymentsResponseData, mapping transaction token and status
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Implement authorize response transformation
  - **Step Dependencies**: Step 4, Step 7
  - **User Instructions**: None

- [x] Step 9: Implement capture transformations
  - **Task**: Implement request and response transformations for capture flow, extracting transaction token and handling capture responses
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Implement capture transformations
  - **Step Dependencies**: Step 5
  - **User Instructions**: None

- [x] Step 10: Implement refund transformations
  - **Task**: Implement request and response transformations for refund flow, handling both full and partial refunds
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Implement refund transformations
  - **Step Dependencies**: Step 5
  - **User Instructions**: None

- [x] Step 11: Implement sync response transformation
  - **Task**: Implement transformation for payment sync responses, mapping Spreedly transaction status to Hyperswitch payment status
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Implement sync transformation
  - **Step Dependencies**: Step 5
  - **User Instructions**: None

## Connector Trait Implementations
- [x] Step 12: Implement base connector traits
  - **Task**: Implement Connector trait with basic methods like get_id, get_base_url, and get_auth_header using HTTP Basic Auth
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement Connector trait
  - **Step Dependencies**: Step 3
  - **User Instructions**: None

- [x] Step 13: Implement Payment trait for authorize flow
  - **Task**: Implement Payment trait for Authorize type, including URL construction, request building, and response handling
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement Payment<Authorize> trait
  - **Step Dependencies**: Step 7, Step 8, Step 12
  - **User Instructions**: None

- [x] Step 14: Implement Payment trait for capture flow
  - **Task**: Implement Payment trait for Capture type with proper URL construction using transaction token
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement Payment<Capture> trait
  - **Step Dependencies**: Step 9, Step 12
  - **User Instructions**: None

- [x] Step 15: Implement Payment trait for sync flow
  - **Task**: Implement Payment trait for PSync type for retrieving transaction status
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement Payment<PSync> trait
  - **Step Dependencies**: Step 11, Step 12
  - **User Instructions**: None

- [x] Step 16: Implement Refund trait
  - **Task**: Implement Refund trait for Execute type to handle refund operations
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement Refund<Execute> trait
  - **Step Dependencies**: Step 10, Step 12
  - **User Instructions**: None

- [x] Step 17: Implement RefundSync trait
  - **Task**: Implement RefundSync trait for RSync type to check refund status
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement RefundSync<RSync> trait
  - **Step Dependencies**: Step 10, Step 12
  - **User Instructions**: None

## Webhook Implementation
- [x] Step 18: Define webhook types
  - **Task**: Create webhook event structures for Spreedly's webhook payload format
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add webhook types
  - **Step Dependencies**: Step 4
  - **User Instructions**: None

- [x] Step 19: Implement webhook verification
  - **Task**: Implement IncomingWebhook trait for webhook source verification and event parsing
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Implement IncomingWebhook trait
  - **Step Dependencies**: Step 18
  - **User Instructions**: None

## Error Handling and Utilities
- [x] Step 20: Implement error response handling
  - **Task**: Implement error parsing and mapping from Spreedly error responses to Hyperswitch error types
  - **Files**: 
    - `hyperswitch_connectors/src/connectors/spreedly.rs`: Add error handling methods
  - **Step Dependencies**: Step 6, Step 12
  - **User Instructions**: None

## Testing Implementation
- [x] Step 21: Implement connector tests
  - **Task**: Write integration tests for authorize, capture, refund, and sync flows using test credentials
  - **Files**: 
    - `crates/router/tests/connectors/spreedly.rs`: Implement test cases
  - **Step Dependencies**: All previous steps
  - **User Instructions**: Configure test environment key and access secret in test configuration

## Documentation and Cleanup
- [x] Step 22: Add connector to the main module
  - **Task**: Export the Spreedly connector module in the main connectors module
  - **Files**: 
    - `hyperswitch_connectors/src/connectors.rs`: Add spreedly module export
  - **Step Dependencies**: All implementation steps
  - **User Instructions**: None

- [x] Step 23: Update connector documentation
  - **Task**: Update the guides and learning documents with insights from Spreedly integration
  - **Files**: 
    - `grace/guides/integrations/integrations.md`: Add Spreedly integration notes
    - `grace/guides/learning/learning.md`: Add lessons learned
    - `grace/guides/patterns/patterns.md`: Add any new patterns discovered
  - **Step Dependencies**: All steps
  - **User Instructions**: Document any special considerations or gotchas encountered

## Summary

This implementation plan covers the complete integration of Spreedly as a payment connector in Hyperswitch. The plan follows a logical progression from initial setup through implementation of all required payment flows (authorize, capture, refund, sync) and webhook handling.

Key considerations for implementation:
1. **Authentication**: HTTP Basic Auth with Environment Key as username and Access Secret as password
2. **Amount Handling**: Always convert to minor units (cents) using existing MinorUnit utilities
3. **Token Management**: Gateway token required for authorization, transaction token for subsequent operations
4. **Rate Limiting**: Be aware of 30 requests/minute limit, implement appropriate retry logic
5. **Error Handling**: Map Spreedly-specific errors to Hyperswitch error types appropriately

The implementation should follow existing connector patterns while using Spreedly's specific API structures and requirements.
