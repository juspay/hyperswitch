const request = require('supertest');
const app = require('../server');

describe('MPGS Mock Server', () => {
  const testMerchantId = 'TEST_MERCHANT';
  const testOrderId = 'order-123';
  const testTransactionId = 'txn-123';
  const validAuth = Buffer.from(`merchant.${testMerchantId}:testpassword`).toString('base64');

  describe('Health and Documentation Endpoints', () => {
    test('GET /health should return OK status', async () => {
      const response = await request(app).get('/health');
      expect(response.status).toBe(200);
      expect(response.body.status).toBe('OK');
    });

    test('GET /api/docs should return API documentation', async () => {
      const response = await request(app).get('/api/docs');
      expect(response.status).toBe(200);
      expect(response.body.name).toBe('MPGS Mock Server');
    });
  });

  describe('Authentication', () => {
    test('should reject requests without authentication', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/${testTransactionId}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4111111111111111',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });

      expect(response.status).toBe(401);
      expect(response.body.error.cause).toBe('REQUEST_REJECTED');
    });

    test('should reject invalid authentication format', async () => {
      const invalidAuth = Buffer.from('invalid:format').toString('base64');
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/${testTransactionId}`)
        .set('Authorization', `Basic ${invalidAuth}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' }
        });

      expect(response.status).toBe(401);
      expect(response.body.error.explanation).toContain('Invalid authentication format');
    });
  });

  describe('Payment Operations', () => {
    test('should process successful PAY operation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/${testTransactionId}`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4111111111111111',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.response.gatewayCode).toBe('APPROVED');
      expect(response.body.transaction.type).toBe('PAYMENT');
      expect(response.body.order.amount).toBe(100);
      expect(response.body.order.currency).toBe('USD');
    });

    test('should process successful AUTHORIZE operation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/auth-123`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'AUTHORIZE',
          order: { amount: '50.00', currency: 'EUR' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '5555555555554444',
                expiry: { month: '06', year: '2026' },
                securityCode: '456'
              }
            }
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.transaction.type).toBe('AUTHORIZATION');
      expect(response.body.order.totalAuthorizedAmount).toBe(50);
    });

    test('should handle declined card', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/decline-123`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4000000000000002',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('FAILURE');
      expect(response.body.response.gatewayCode).toBe('DECLINED');
    });

    test('should handle insufficient funds', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/insufffunds-123`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4000000000000119',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('FAILURE');
      expect(response.body.response.gatewayCode).toBe('INSUFFICIENT_FUNDS');
    });

    test('should handle 3DS authentication', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/3ds-123`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'AUTHORIZE',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4000000000000044',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('PENDING');
      expect(response.body.response.gatewayCode).toBe('AUTHENTICATION_IN_PROGRESS');
    });
  });

  describe('Capture Operation', () => {
    beforeEach(async () => {
      // Create an authorization first
      await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/capture-order/auth-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'AUTHORIZE',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4111111111111111',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });
    });

    test('should process successful CAPTURE operation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/capture-order/capture-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'CAPTURE',
          transaction: {
            amount: '75.00',
            currency: 'USD'
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.transaction.type).toBe('CAPTURE');
      expect(response.body.transaction.amount).toBe(75);
    });
  });

  describe('Void Operation', () => {
    beforeEach(async () => {
      // Create an authorization first
      await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/void-order/auth-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'AUTHORIZE',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4111111111111111',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });
    });

    test('should process successful VOID operation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/void-order/void-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'VOID_AUTHORIZATION',
          transaction: {
            reference: 'auth-txn'
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.transaction.type).toBe('VOID_AUTHORIZATION');
    });
  });

  describe('Refund Operation', () => {
    beforeEach(async () => {
      // Create a payment first
      await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/refund-order/pay-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4111111111111111',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });
    });

    test('should process successful REFUND operation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/refund-order/refund-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'REFUND',
          transaction: {
            amount: '50.00',
            currency: 'USD'
          }
        });

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.transaction.type).toBe('REFUND');
      expect(response.body.transaction.amount).toBe(50);
    });
  });

  describe('Order and Transaction Retrieval', () => {
    beforeEach(async () => {
      // Create a payment first
      await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/get-order/pay-txn`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'PAY',
          order: { amount: '100.00', currency: 'USD' },
          sourceOfFunds: {
            type: 'CARD',
            provided: {
              card: {
                number: '4111111111111111',
                expiry: { month: '12', year: '2025' },
                securityCode: '123'
              }
            }
          }
        });
    });

    test('should retrieve order details', async () => {
      const response = await request(app)
        .get(`/api/rest/version/100/merchant/${testMerchantId}/order/get-order`)
        .set('Authorization', `Basic ${validAuth}`);

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.order.id).toBe('get-order');
      expect(response.body.order.amount).toBe(100);
    });

    test('should retrieve transaction details', async () => {
      const response = await request(app)
        .get(`/api/rest/version/100/merchant/${testMerchantId}/order/get-order/transaction/pay-txn`)
        .set('Authorization', `Basic ${validAuth}`);

      expect(response.status).toBe(200);
      expect(response.body.result).toBe('SUCCESS');
      expect(response.body.transaction.id).toBe('pay-txn');
    });

    test('should return 404 for non-existent order', async () => {
      const response = await request(app)
        .get(`/api/rest/version/100/merchant/${testMerchantId}/order/non-existent`)
        .set('Authorization', `Basic ${validAuth}`);

      expect(response.status).toBe(404);
      expect(response.body.error.explanation).toBe('Order not found');
    });
  });

  describe('Request Validation', () => {
    test('should reject request without apiOperation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/${testTransactionId}`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          order: { amount: '100.00', currency: 'USD' }
        });

      expect(response.status).toBe(400);
      expect(response.body.error.cause).toBe('INVALID_REQUEST');
      expect(response.body.error.field).toBe('apiOperation');
    });

    test('should reject request without order details', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/${testTransactionId}`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'PAY'
        });

      expect(response.status).toBe(400);
      expect(response.body.error.cause).toBe('INVALID_REQUEST');
      expect(response.body.error.field).toBe('order');
    });

    test('should reject unsupported API operation', async () => {
      const response = await request(app)
        .put(`/api/rest/version/100/merchant/${testMerchantId}/order/${testOrderId}/transaction/${testTransactionId}`)
        .set('Authorization', `Basic ${validAuth}`)
        .send({
          apiOperation: 'UNSUPPORTED_OPERATION',
          order: { amount: '100.00', currency: 'USD' }
        });

      expect(response.status).toBe(400);
      expect(response.body.error.cause).toBe('INVALID_REQUEST');
      expect(response.body.error.field).toBe('apiOperation');
    });
  });
});
