# Router Refund Flows

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Core Module](../modules/core.md)
- [Payment Flows](./payment_flows.md)
- [Webhook Flows](./webhook_flows.md)
---

[← Back to Router Overview](../overview.md)

## Overview

Refund flows represent the processes for returning funds to customers after a successful payment. The Hyperswitch platform provides comprehensive refund capabilities, supporting full and partial refunds, as well as multiple refunds against a single payment. This document details the refund flows implemented in the router crate.

## Key Refund Flows

### Refund Creation Flow

The refund creation flow initiates a refund for a previously captured payment:

1. **API Request Reception**: 
   - Client submits a `POST /refunds` request with refund details
   - Request includes payment ID, amount, and optional reason
   - Request is validated and parsed

2. **Payment Verification**:
   - System verifies the referenced payment exists
   - Checks payment is in a refundable state (typically `CAPTURED`)
   - Validates merchant owns the payment

3. **Refund Amount Validation**:
   - Validates refund amount doesn't exceed remaining refundable amount
   - Checks currency matches the original payment
   - For partial refunds, ensures amount is within valid ranges

4. **Refund Record Creation**:
   - Creates a refund record in the database
   - Generates a unique refund ID
   - Links refund to the original payment
   - Sets initial status to `PENDING`

5. **Connector Selection**:
   - Uses the same connector that processed the original payment
   - Retrieves connector details and credentials

6. **Refund Request Preparation**:
   - Transforms refund request to connector-specific format
   - Includes original payment reference
   - Adds refund-specific metadata

7. **Connector Communication**:
   - Sends refund request to payment processor
   - Receives and processes response
   - Handles connector-specific error mapping

8. **Refund Status Update**:
   - Updates refund status based on connector response
   - Successful refunds: status set to `SUCCEEDED` or `PROCESSING`
   - Failed refunds: status set to `FAILED` with appropriate error codes

9. **Payment Update**:
   - Updates the original payment's refunded amount
   - Updates payment's refund status if fully refunded

10. **Response Formatting**:
    - Returns refund result to client
    - Includes refund ID, status, and amount

This flow is implemented in the `refunds` module of the core component.

### Refund Status Sync Flow

For synchronizing refund status with the payment processor:

1. **Sync Request Reception**:
   - Client submits a `POST /refunds/sync` request with refund ID
   - Request is validated and parsed

2. **Connector Selection**:
   - Identifies connector used for the original refund
   - Retrieves connector details and credentials

3. **Sync Request Preparation**:
   - Transforms request to connector-specific format
   - Includes original refund reference

4. **Connector Communication**:
   - Sends status inquiry to payment processor
   - Receives and processes response

5. **Refund Status Update**:
   - Updates refund status if changed in the processor's system
   - Records latest refund details

6. **Response Formatting**:
   - Returns current refund status to client
   - Includes complete refund details

This flow allows merchants to explicitly check the current status of a refund with the payment processor, which is useful for resolving discrepancies or handling cases where webhook notifications might have been missed.

### Multiple Refunds Flow

For processing multiple refunds against a single payment:

1. **Refund Request Reception**:
   - Same as standard refund creation
   - System recognizes this is not the first refund for the payment

2. **Cumulative Refund Validation**:
   - Calculates total previously refunded amount
   - Ensures new refund plus previous refunds don't exceed payment amount
   - Validates against merchant's multiple refund policies

3. **Refund Processing**:
   - Processes the refund similar to standard refund flow
   - Updates payment with additional refund information

4. **Aggregate Refund Tracking**:
   - Maintains a record of all refunds against the payment
   - Updates payment's refunded amount and refund count
   - Updates payment status to `FULLY_REFUNDED` if applicable

This flow extends the standard refund creation flow to handle the complexities of multiple refunds for a single payment.

## Asynchronous Refund Processing

Many payment processors handle refunds asynchronously, which requires special handling:

1. **Initial Acceptance**:
   - Processor acknowledges refund request but doesn't process immediately
   - System sets refund status to `PROCESSING`

2. **Status Monitoring**:
   - System periodically checks refund status with processor
   - Alternatively, waits for webhook notification of status change

3. **Final Status Update**:
   - Once processor completes refund, status is updated to `SUCCEEDED` or `FAILED`
   - Merchant is notified of final status via webhook (if configured)

This asynchronous flow ensures proper tracking of refunds that may take time to process, particularly for certain payment methods like bank transfers or some digital wallets.

## Error Handling in Refund Flows

Refund flows implement comprehensive error handling:

- **Validation Errors**: Errors in refund parameters or constraints
- **Payment State Errors**: Errors when payment is in a non-refundable state
- **Connector Errors**: Errors from the payment processor
- **Timing Errors**: Errors related to refund time limits
- **Amount Errors**: Errors related to refund amount constraints

Each error type is handled appropriately with clear error messages and status codes.

## Refund Time Limits

Refund flows enforce time limits based on:

1. **Connector Constraints**: Respects the payment processor's refund time limits
2. **Merchant Configuration**: Applies merchant-specific refund window settings
3. **Regulatory Requirements**: Complies with relevant financial regulations

The system validates refund requests against these time constraints before processing.

## Dependencies

Refund flows depend on several key components:

- **Core Refund Logic**: Implements the business rules for refund processing
- **Connector Implementations**: Provides communication with payment processors
- **Database Services**: Stores and retrieves refund state
- **Payment Services**: Accesses and updates payment records
- **Domain Models**: Defines the data structures for refund operations

## See Also

- [Core Module Documentation](../modules/core.md)
- [Payment Flows Documentation](./payment_flows.md)
- [Webhook Flows Documentation](./webhook_flows.md)
