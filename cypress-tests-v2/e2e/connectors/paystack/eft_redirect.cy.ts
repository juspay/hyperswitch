/// <reference types="cypress" />
// Cypress E2E: Paystack EFT bank_redirect flow

const BASE =
  Cypress.env('PAYMENT_BASE_URL') ||
  Cypress.config('baseUrl') ||
  'http://localhost:8080';

describe('Paystack EFT bank_redirect', () => {
  it('happy path: completes bank_redirect and returns reference', () => {
    cy.request('POST', `${BASE}/payments`, {
      amount: 100,
      currency: 'NGN',
      payment_method: {
        payment_method_type: 'bank_redirect',
        bank_redirect: {
          paystack: {
            eft: {}
          }
        }
      },
      payment_method_data: {
        billing: {
          address: {
            country: 'NG'
          }
        }
      },
      return_url: 'http://example.com',
      merchant_id: 'merchant_123'
    }).then((response) => {
      expect(response.status).to.eq(200)
      const { next_action } = response.body
      expect(next_action).to.have.property('redirect_to_url')
      
      if (Cypress.env('CYPRESS_SKIP_DASHBOARD')) {
        cy.intercept('GET', '**/dashboard/**', {
          statusCode: 200,
          body: '<div>Dashboard Stub</div>',
        }).as('dash');
      }
      
      cy.visit(next_action.redirect_to_url.url)
      if (Cypress.env('CYPRESS_SKIP_DASHBOARD')) cy.wait('@dash')
      cy.location('href').should('include', 'reference')
    })
  })

  it('negative: invalid client_secret returns error', () => {
    cy.request({
      method: 'POST',
      url: `${BASE}/payments/confirm`,
      failOnStatusCode: false,
      body: {
        client_secret: ''
      }
    }).its('status').should('be.oneOf', [400, 401, 422])
  })
})
