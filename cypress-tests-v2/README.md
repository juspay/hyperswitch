# Cypress Connector Tests

This directory contains end-to-end tests for payment connectors using Cypress.

## Running Tests Locally

1. Start Docker
2. Run the development environment:
   ```bash
   ./scripts/setup.sh
   # or
   docker compose -f docker-compose-development.yml up -d
   ```
3. Create `cypress.env.json` with your test credentials:
   ```json
   {
     "PAYMENT_BASE_URL": "http://localhost:8080",
     "PAYSTACK_PUBLIC_KEY": "pk_test_your_key",
     "PAYSTACK_SECRET_KEY": "sk_test_your_key",
     "CYPRESS_SKIP_DASHBOARD": true
   }
   ```
4. Run the tests:
   ```bash
   npx cypress run -C cypress-tests-v2 \
     --spec "e2e/connectors/paystack/eft_redirect.cy.ts" \
     --browser chrome --headless
   ```

## Adding New Connector Tests

- Place test files in `e2e/connectors/<connector>/`
- Use the pattern `<connector>_<method>.cy.ts`
- Include Cypress types: `/// <reference types="cypress" />`
- Use environment-based baseUrl:
  ```typescript
  const BASE = Cypress.env('PAYMENT_BASE_URL') || Cypress.config('baseUrl') || 'http://localhost:8080';
  ```
