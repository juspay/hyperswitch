# ACH Payment Tests

This directory contains comprehensive Cypress tests for ACH (Automated Clearing House) payment methods in Hyperswitch.

## Overview

The ACH payment tests cover the following connectors that support ACH payments:
- **Dwolla** - Specialized ACH processor with micro-deposit verification
- **Stripe** - ACH Bank Debit & Transfer support
- **GoCardless** - International ACH via bank debit

## Test Structure

### Test File
- `00030-ACH-Payments.cy.js` - Main ACH payment test specification

### Fixtures
- `ach-payment-body.json` - ACH payment intent request body
- `ach-confirm-body.json` - ACH payment confirmation request body
- `ach-mandate-body.json` - ACH mandate setup request body

### Connector Configurations
- `Dwolla.js` - Dwolla-specific ACH test configurations
- `Stripe.js` - Stripe ACH configurations (added to existing file)
- `GoCardless.js` - GoCardless ACH configurations

### Custom Commands
The following ACH-specific Cypress commands are available:
- `createAchPaymentIntent()` - Creates ACH payment intent
- `confirmAchPayment()` - Confirms ACH payment with bank details
- `setupAchMandate()` - Sets up ACH mandate for recurring payments
- `verifyAchBankAccount()` - Verifies bank account via micro-deposits
- `paymentSyncTest()` - Syncs ACH payment status (important for async payments)

## Test Scenarios

### 1. ACH Bank Debit Payments
- Create customer
- Create ACH payment intent
- Confirm payment with bank account details
- Sync payment status (ACH payments are typically async)
- Process refund

### 2. ACH Mandate Payments
- Create customer
- Setup ACH mandate for recurring payments
- Create payment using existing mandate
- List customer payment methods

### 3. Bank Account Verification
- Create unverified bank account
- Initiate micro-deposit verification
- Complete verification with micro-deposit amounts

### 4. Error Handling
- Test insufficient funds scenarios
- Test invalid account number handling
- Test various ACH return codes

## Running the Tests

### Prerequisites
1. Ensure you have valid connector credentials configured
2. Set up the appropriate environment variables
3. Configure test bank account numbers for each connector

### Run All ACH Tests
```bash
npx cypress run --spec "cypress/e2e/spec/Payment/00030-ACH-Payments.cy.js"
```

### Run with Specific Connector
```bash
# For Dwolla
CYPRESS_CONNECTOR_ID=dwolla npx cypress run --spec "cypress/e2e/spec/Payment/00030-ACH-Payments.cy.js"

# For Stripe
CYPRESS_CONNECTOR_ID=stripe npx cypress run --spec "cypress/e2e/spec/Payment/00030-ACH-Payments.cy.js"

# For GoCardless
CYPRESS_CONNECTOR_ID=gocardless npx cypress run --spec "cypress/e2e/spec/Payment/00030-ACH-Payments.cy.js"
```

### Run in Headed Mode (for debugging)
```bash
npx cypress open --spec "cypress/e2e/spec/Payment/00030-ACH-Payments.cy.js"
```

## Test Data

### Bank Account Details
The tests use the following test bank account numbers:

**Valid Test Account:**
- Account Number: `000123456789`
- Routing Number: `110000000`
- Account Type: `checking`

**Insufficient Funds Test Account:**
- Account Number: `000000000002`
- Routing Number: `110000000`
- Account Type: `checking`

**Note:** These are test account numbers. Never use real bank account information in tests.

## Important Considerations

### Asynchronous Nature
ACH payments are inherently asynchronous and may take several business days to complete. The tests account for this by:
- Using `paymentSyncTest()` to check payment status
- Expecting `processing` status for many ACH payments
- Handling various intermediate statuses

### Connector-Specific Behavior
Each connector has different ACH implementation details:

**Dwolla:**
- Specializes in ACH processing
- Supports micro-deposit verification
- Provides real-time bank account verification
- Handles ACH return codes comprehensively

**Stripe:**
- Supports both ACH debit and transfer
- Integrates with existing card payment flows
- Provides mandate support for recurring payments

**GoCardless:**
- Focuses on international bank debit including ACH
- Strong mandate management capabilities
- Webhook support for payment events

### Security Best Practices
- Never use real bank account numbers in tests
- Mask sensitive data in test logs
- Use environment variables for API keys
- Implement proper data cleanup after tests

## Debugging

### Common Issues
1. **Payment Status Stuck in Processing**: This is normal for ACH payments. Use sync calls to check status.
2. **Micro-deposit Verification Fails**: Ensure the connector supports this feature and test amounts are correct.
3. **Invalid Account Numbers**: Use connector-specific test account numbers.

### Logging
The tests include comprehensive logging:
- Request/response IDs for tracing
- Payment status updates
- Error messages and codes
- Micro-deposit verification responses

### Test Data Cleanup
Tests automatically clean up:
- Customer records
- Payment method records
- Mandate records

## Contributing

When adding new ACH test scenarios:
1. Follow the existing test structure
2. Add appropriate connector configurations
3. Include error handling scenarios
4. Document any connector-specific behavior
5. Ensure tests work with all supported connectors

## Support

For issues with ACH payment tests:
1. Check connector-specific documentation
2. Verify test account numbers are valid
3. Ensure proper environment configuration
4. Review payment status and error codes
