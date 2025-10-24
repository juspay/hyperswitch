/// <reference types="cypress" />
// cypress-tests-v2/e2e/connectors/paystack/eft_redirect.cy.ts

const BASE = Cypress.env('PAYMENT_BASE_URL') || 'http://localhost:8080';
const PK   = Cypress.env('PAYSTACK_PUBLIC_KEY') || 'pk_test_x';
const SK   = Cypress.env('PAYSTACK_SECRET_KEY') || 'sk_test_x';

describe('Paystack EFT bank_redirect', () => {
  it('happy path: completes redirect and returns reference', () => {
    cy.log('Create intent');
    cy.request('POST', `${BASE}/payments`, {
      amount: 1000, currency: 'NGN', capture_method: 'automatic',
      confirm: true, payment_method: { type: 'bank_redirect', provider: 'paystack' },
      metadata: { test: 'eft' }
    }).then((r) => {
      expect(r.status).to.eq(200);
      const { next_action } = r.body;
      expect(next_action?.redirect_to_url?.url, 'redirect url').to.be.a('string');

      cy.visit(next_action.redirect_to_url.url);

      // Stub/short-circuit the dashboard page in CI if needed
      if (Cypress.env('CYPRESS_SKIP_DASHBOARD')) {
        cy.intercept('GET', '**paystack**', { statusCode: 200, body: '<html>stub</html>' });
      }

      // After redirect completes, Hyperswitch should hit our return_url with a reference
      cy.location('href', { timeout: 30_000 }).should('include', 'reference')
    });
  });

  it('negative: invalid client secret is rejected', () => {
    cy.request({
      method: 'POST',
      url: `${BASE}/payments/confirm`,
      failOnStatusCode: false,
      body: { client_secret: '' }
    }).its('status').should('be.oneOf', [400, 401, 422]);
  });
});
