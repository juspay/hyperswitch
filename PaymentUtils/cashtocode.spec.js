// cypress/e2e/PaymentUtils/cashtocode.spec.js
import CashtoCode from './cashtocode';

describe('CashtoCode Payment Method', () => {
  it('should make a payment', () => {
    const cashtoCode = new CashtoCode();
    const amount = 10.99;
    const currency = 'USD';

    cy.wrap(cashtoCode.makePayment(amount, currency)).should('be.ok');
  });

  it('should get payment status', () => {
    const cashtoCode = new CashtoCode();
    const paymentId = '1234567890';

    cy.wrap(cashtoCode.getPaymentStatus(paymentId)).should('be.ok');
  });
});
