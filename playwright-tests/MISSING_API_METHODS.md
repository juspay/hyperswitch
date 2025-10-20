# Missing API Helper Methods

This document lists all API helper methods referenced in the test files but not yet implemented in `ApiHelpers.ts`.

## Status: TO BE IMPLEMENTED

These methods need to be ported from Cypress `commands.js` to complete the test suite.

## Payment Operations

### Core Payment Methods

```typescript
// Create payment intent (without auto-confirm)
async createPaymentIntent(
  data: any,
  authType: string = 'HeaderKey'
): Promise<void>

// Confirm a created payment
async confirmPayment(
  confirmData: any,
  authType: string = 'HeaderKey'
): Promise<void>

// Capture an authorized payment
async capturePayment(
  captureData: any,
  paymentIntentId?: string,
  globalState?: State
): Promise<void>

// Void/cancel a payment
async voidPayment(
  voidData: any,
  authType: string = 'HeaderKey'
): Promise<void>

// Retrieve payment details
async retrievePayment(
  paymentId: string,
  authType: string = 'HeaderKey'
): Promise<void>

// Sync payment with connector
async paymentSync(authType: string = 'HeaderKey'): Promise<void>

// List payments
async listPayments(): Promise<void>
```

## Refund Operations

```typescript
// Create refund
async refundCall(
  refundData: any,
  expectedStatus: number = 200,
  authType: string = 'HeaderKey'
): Promise<void>

// Sync refund with connector
async refundSync(refundId: string): Promise<void>

// List refunds
async listRefunds(queryParams?: any): Promise<void>
```

## Mandate Operations

```typescript
// Create single-use mandate
async createMandate(
  mandateData: any,
  authType: string = 'HeaderKey'
): Promise<void>

// Create multi-use mandate
async createMultiuseMandate(
  mandateData: any
): Promise<void>

// Confirm mandate
async confirmMandate(
  confirmData: any
): Promise<void>

// List customer mandates
async listMandates(customerId: string): Promise<void>

// Revoke mandate
async revokeMandate(mandateId: string): Promise<void>

// Use saved payment method for mandate
async useSavedPaymentMethod(
  paymentMethodId: string
): Promise<void>
```

## Payment Method Operations

```typescript
// List payment methods for customer
async listPaymentMethods(
  customerId?: string
): Promise<void>

// List payment methods with required fields
async paymentMethodsListWithRequiredFields(
  data: any
): Promise<void>

// Save payment method (card tokenization)
async saveCard(
  cardData: any,
  customerId?: string
): Promise<void>

// Use saved card for payment
async useSavedCard(
  paymentMethodId: string,
  paymentData: any
): Promise<void>

// Delete payment method
async deletePaymentMethod(
  paymentMethodId: string
): Promise<void>
```

## Customer Operations

```typescript
// Update customer
async updateCustomer(
  customerId: string,
  updateData: any
): Promise<void>

// Delete customer
async deleteCustomer(
  customerId: string
): Promise<void>

// Retrieve customer
async getCustomer(customerId: string): Promise<void>
```

## Business Profile Operations

```typescript
// Update business profile
async updateBusinessProfile(
  profileId: string,
  updateData: any
): Promise<void>

// Retrieve business profile
async getBusinessProfile(profileId: string): Promise<void>
```

## Session Operations

```typescript
// Create payment session
async createSession(
  sessionData: any
): Promise<void>

// Retrieve session
async getSession(
  sessionId: string
): Promise<void>
```

## Alternative Payment Methods

### Bank Transfer

```typescript
// Create bank transfer payment
async createBankTransferPayment(
  paymentData: any
): Promise<void>

// Confirm bank transfer
async confirmBankTransfer(
  confirmData: any
): Promise<void>
```

### Bank Redirect

```typescript
// Create bank redirect payment
async createBankRedirectPayment(
  paymentData: any
): Promise<void>

// Confirm bank redirect
async confirmBankRedirect(
  confirmData: any
): Promise<void>
```

### UPI

```typescript
// Create UPI payment
async createUPIPayment(
  paymentData: any
): Promise<void>

// Confirm UPI payment
async confirmUPIPayment(
  confirmData: any
): Promise<void>
```

## Advanced Features

### Zero Auth

```typescript
// Create zero-auth mandate
async createZeroAuthMandate(
  mandateData: any
): Promise<void>

// Confirm zero-auth
async confirmZeroAuth(
  confirmData: any
): Promise<void>
```

### Incremental Authorization

```typescript
// Create incremental auth payment
async createIncrementalAuthPayment(
  paymentData: any
): Promise<void>

// Increment authorization amount
async incrementAuthorization(
  paymentId: string,
  incrementData: any
): Promise<void>
```

### Overcapture

```typescript
// Capture more than authorized amount
async overcapture(
  paymentId: string,
  captureData: any
): Promise<void>
```

## Testing Utilities

### Race Condition Testing

```typescript
// Test DDC server-side race condition
async ddcServerSideRaceCondition(
  paymentId: string,
  testData: any
): Promise<void>

// Test DDC client-side race condition
async ddcClientSideRaceCondition(
  paymentId: string,
  testData: any
): Promise<void>
```

### Retry Testing

```typescript
// Manually retry failed payment
async manualRetry(
  paymentId: string,
  retryData: any
): Promise<void>
```

### Variation Testing

```typescript
// Test payment with variations
async testPaymentVariation(
  variationData: any
): Promise<void>
```

## Implementation Notes

### Common Patterns

All methods should follow these patterns:

1. **Request Headers**:
```typescript
const headers: Record<string, string> = {};
if (authType === 'HeaderKey') {
  headers['api-key'] = this.state.get('publishableKey') || this.state.get('apiKey');
} else if (authType === 'PublishableKey') {
  headers['api-key'] = this.state.get('publishableKey');
}
headers['Content-Type'] = 'application/json';
```

2. **Response Handling**:
```typescript
const response = await this.request.post(url, { headers, data });
const body = await response.json();

// Save important IDs to state
if (body.payment_id) this.state.set('paymentId', body.payment_id);
if (body.client_secret) this.state.set('clientSecret', body.client_secret);
// etc.

// Assert response status
expect(response.status()).toBe(expectedStatus);
```

3. **Error Handling**:
```typescript
try {
  // API call
} catch (error) {
  console.error(`API call failed: ${error}`);
  throw error;
}
```

### State Management

Key state values to track:
- `paymentId` - Current payment ID
- `clientSecret` - Payment client secret
- `refundId` - Last created refund ID
- `mandateId` - Last created mandate ID
- `paymentMethodId` - Saved payment method ID
- `sessionId` - Session token ID
- `customerId` - Current customer ID
- `merchantId` - Merchant account ID
- `profileId` - Business profile ID

## Priority Implementation Order

Based on test file usage frequency:

1. **High Priority** (used in 10+ tests):
   - `createPaymentIntent`
   - `confirmPayment`
   - `capturePayment`
   - `refundCall`
   - `paymentSync`

2. **Medium Priority** (used in 5-10 tests):
   - `createMandate`
   - `confirmMandate`
   - `listMandates`
   - `revokeMandate`
   - `saveCard`
   - `listPaymentMethods`

3. **Low Priority** (used in < 5 tests):
   - Alternative payment methods
   - Advanced features
   - Testing utilities

## Porting from Cypress

To port a method from Cypress `commands.js`:

1. Find the Cypress.Commands.add() definition
2. Convert cy.request() to Playwright APIRequestContext
3. Convert cy.wrap() chains to async/await
4. Replace aliasing with state.set()
5. Replace assertions with expect()
6. Add TypeScript types

Example conversion:

```typescript
// Cypress
Cy.Commands.add('createPaymentIntent', (data) => {
  return cy.request({
    method: 'POST',
    url: `${baseUrl}/payments`,
    headers: { 'api-key': apiKey },
    body: data
  }).then((response) => {
    cy.wrap(response.body.payment_id).as('paymentId');
    expect(response.status).to.eq(200);
  });
});

// Playwright
async createPaymentIntent(data: any): Promise<void> {
  const baseUrl = this.state.get('baseUrl');
  const apiKey = this.state.get('apiKey');

  const response = await this.request.post(`${baseUrl}/payments`, {
    headers: { 'api-key': apiKey, 'Content-Type': 'application/json' },
    data: data,
  });

  const body = await response.json();
  this.state.set('paymentId', body.payment_id);
  expect(response.status()).toBe(200);
}
```

## Next Steps

1. Review Cypress `commands.js` file
2. Port methods one section at a time
3. Test each method with at least one test file
4. Update this document as methods are implemented
5. Mark tests as passing when their required methods are complete

## Related Files

- Implementation: `/tests/helpers/ApiHelpers.ts`
- Cypress source: `/cypress-tests/cypress/support/commands.js`
- Type definitions: `/tests/e2e/configs/ConnectorTypes.ts`
- Test fixtures: `/tests/fixtures/test-data.ts`
