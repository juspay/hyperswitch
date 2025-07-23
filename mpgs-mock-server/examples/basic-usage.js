/**
 * Basic Usage Examples for MPGS Mock Server
 * 
 * This file demonstrates how to use the MPGS Mock Server
 * for testing various payment scenarios.
 */

const https = require('https');
const http = require('http');

// Configuration
const BASE_URL = 'http://localhost:3001/api/rest/version/100';
const MERCHANT_ID = 'TEST_MERCHANT';
const API_PASSWORD = 'testpassword';

// Create Basic Auth header
const authString = `merchant.${MERCHANT_ID}:${API_PASSWORD}`;
const authHeader = `Basic ${Buffer.from(authString).toString('base64')}`;

/**
 * Helper function to make HTTP requests
 */
function makeRequest(method, url, data = null) {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(url);
    
    const options = {
      hostname: urlObj.hostname,
      port: urlObj.port || 80,
      path: urlObj.pathname,
      method: method,
      headers: {
        'Authorization': authHeader,
        'Content-Type': 'application/json'
      }
    };

    if (data) {
      const jsonData = JSON.stringify(data);
      options.headers['Content-Length'] = Buffer.byteLength(jsonData);
    }

    const req = http.request(options, (res) => {
      let responseData = '';
      
      res.on('data', (chunk) => {
        responseData += chunk;
      });
      
      res.on('end', () => {
        try {
          const jsonResponse = JSON.parse(responseData);
          resolve({
            statusCode: res.statusCode,
            data: jsonResponse
          });
        } catch (e) {
          resolve({
            statusCode: res.statusCode,
            data: responseData
          });
        }
      });
    });

    req.on('error', (err) => {
      reject(err);
    });

    if (data) {
      req.write(JSON.stringify(data));
    }
    
    req.end();
  });
}

/**
 * Example 1: Successful Payment (PAY)
 */
async function example1_SuccessfulPayment() {
  console.log('\n=== Example 1: Successful Payment ===');
  
  const orderId = `order-${Date.now()}`;
  const transactionId = `txn-${Date.now()}`;
  
  const url = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${transactionId}`;
  
  const requestData = {
    apiOperation: 'PAY',
    order: {
      amount: '100.00',
      currency: 'USD'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '4111111111111111',
          expiry: {
            month: '12',
            year: '2025'
          },
          securityCode: '123'
        }
      }
    }
  };

  try {
    const response = await makeRequest('PUT', url, requestData);
    console.log('Status:', response.statusCode);
    console.log('Response:', JSON.stringify(response.data, null, 2));
    return response.data;
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Example 2: Authorization followed by Capture
 */
async function example2_AuthorizeAndCapture() {
  console.log('\n=== Example 2: Authorize and Capture ===');
  
  const orderId = `order-${Date.now()}`;
  const authTxnId = `auth-${Date.now()}`;
  const captureTxnId = `capture-${Date.now()}`;
  
  // Step 1: Authorize
  console.log('Step 1: Authorization');
  const authUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${authTxnId}`;
  
  const authRequest = {
    apiOperation: 'AUTHORIZE',
    order: {
      amount: '150.00',
      currency: 'EUR'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '5555555555554444',
          expiry: {
            month: '06',
            year: '2026'
          },
          securityCode: '456'
        }
      }
    }
  };

  try {
    const authResponse = await makeRequest('PUT', authUrl, authRequest);
    console.log('Auth Status:', authResponse.statusCode);
    console.log('Auth Response:', JSON.stringify(authResponse.data, null, 2));

    // Step 2: Capture
    console.log('\nStep 2: Capture');
    const captureUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${captureTxnId}`;
    
    const captureRequest = {
      apiOperation: 'CAPTURE',
      transaction: {
        amount: '150.00',
        currency: 'EUR'
      }
    };

    const captureResponse = await makeRequest('PUT', captureUrl, captureRequest);
    console.log('Capture Status:', captureResponse.statusCode);
    console.log('Capture Response:', JSON.stringify(captureResponse.data, null, 2));
    
    return { authResponse: authResponse.data, captureResponse: captureResponse.data };
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Example 3: Declined Payment
 */
async function example3_DeclinedPayment() {
  console.log('\n=== Example 3: Declined Payment ===');
  
  const orderId = `order-${Date.now()}`;
  const transactionId = `txn-${Date.now()}`;
  
  const url = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${transactionId}`;
  
  const requestData = {
    apiOperation: 'PAY',
    order: {
      amount: '100.00',
      currency: 'USD'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '4000000000000002', // Test card that gets declined
          expiry: {
            month: '12',
            year: '2025'
          },
          securityCode: '123'
        }
      }
    }
  };

  try {
    const response = await makeRequest('PUT', url, requestData);
    console.log('Status:', response.statusCode);
    console.log('Response:', JSON.stringify(response.data, null, 2));
    return response.data;
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Example 4: Insufficient Funds
 */
async function example4_InsufficientFunds() {
  console.log('\n=== Example 4: Insufficient Funds ===');
  
  const orderId = `order-${Date.now()}`;
  const transactionId = `txn-${Date.now()}`;
  
  const url = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${transactionId}`;
  
  const requestData = {
    apiOperation: 'PAY',
    order: {
      amount: '100.00',
      currency: 'USD'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '4000000000000119', // Test card for insufficient funds
          expiry: {
            month: '12',
            year: '2025'
          },
          securityCode: '123'
        }
      }
    }
  };

  try {
    const response = await makeRequest('PUT', url, requestData);
    console.log('Status:', response.statusCode);
    console.log('Response:', JSON.stringify(response.data, null, 2));
    return response.data;
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Example 5: 3DS Authentication Required
 */
async function example5_3DSAuthentication() {
  console.log('\n=== Example 5: 3DS Authentication Required ===');
  
  const orderId = `order-${Date.now()}`;
  const transactionId = `txn-${Date.now()}`;
  
  const url = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${transactionId}`;
  
  const requestData = {
    apiOperation: 'AUTHORIZE',
    order: {
      amount: '100.00',
      currency: 'USD'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '4000000000000044', // Test card for 3DS
          expiry: {
            month: '12',
            year: '2025'
          },
          securityCode: '123'
        }
      }
    }
  };

  try {
    const response = await makeRequest('PUT', url, requestData);
    console.log('Status:', response.statusCode);
    console.log('Response:', JSON.stringify(response.data, null, 2));
    return response.data;
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Example 6: Refund Operation
 */
async function example6_RefundOperation() {
  console.log('\n=== Example 6: Refund Operation ===');
  
  const orderId = `order-${Date.now()}`;
  const payTxnId = `pay-${Date.now()}`;
  const refundTxnId = `refund-${Date.now()}`;
  
  // Step 1: Make a payment
  console.log('Step 1: Payment');
  const payUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${payTxnId}`;
  
  const payRequest = {
    apiOperation: 'PAY',
    order: {
      amount: '200.00',
      currency: 'USD'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '4111111111111111',
          expiry: {
            month: '12',
            year: '2025'
          },
          securityCode: '123'
        }
      }
    }
  };

  try {
    const payResponse = await makeRequest('PUT', payUrl, payRequest);
    console.log('Payment Status:', payResponse.statusCode);
    console.log('Payment Response:', JSON.stringify(payResponse.data, null, 2));

    // Step 2: Refund
    console.log('\nStep 2: Refund');
    const refundUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${refundTxnId}`;
    
    const refundRequest = {
      apiOperation: 'REFUND',
      transaction: {
        amount: '100.00',
        currency: 'USD'
      }
    };

    const refundResponse = await makeRequest('PUT', refundUrl, refundRequest);
    console.log('Refund Status:', refundResponse.statusCode);
    console.log('Refund Response:', JSON.stringify(refundResponse.data, null, 2));
    
    return { payResponse: payResponse.data, refundResponse: refundResponse.data };
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Example 7: Get Order Details
 */
async function example7_GetOrderDetails() {
  console.log('\n=== Example 7: Get Order Details ===');
  
  const orderId = `order-${Date.now()}`;
  const transactionId = `txn-${Date.now()}`;
  
  // First create a payment
  const createUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${transactionId}`;
  
  const createRequest = {
    apiOperation: 'PAY',
    order: {
      amount: '75.00',
      currency: 'GBP'
    },
    sourceOfFunds: {
      type: 'CARD',
      provided: {
        card: {
          number: '4111111111111111',
          expiry: {
            month: '12',
            year: '2025'
          },
          securityCode: '123'
        }
      }
    }
  };

  try {
    // Create payment
    await makeRequest('PUT', createUrl, createRequest);
    
    // Get order details
    const getUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}`;
    const orderResponse = await makeRequest('GET', getUrl);
    
    console.log('Order Status:', orderResponse.statusCode);
    console.log('Order Details:', JSON.stringify(orderResponse.data, null, 2));
    
    // Get transaction details
    const getTxnUrl = `${BASE_URL}/merchant/${MERCHANT_ID}/order/${orderId}/transaction/${transactionId}`;
    const txnResponse = await makeRequest('GET', getTxnUrl);
    
    console.log('\nTransaction Status:', txnResponse.statusCode);
    console.log('Transaction Details:', JSON.stringify(txnResponse.data, null, 2));
    
    return { orderDetails: orderResponse.data, transactionDetails: txnResponse.data };
  } catch (error) {
    console.error('Error:', error);
  }
}

/**
 * Run all examples
 */
async function runAllExamples() {
  console.log('MPGS Mock Server - Usage Examples');
  console.log('==================================');
  
  // Check if server is running
  try {
    const healthResponse = await makeRequest('GET', 'http://localhost:3001/health');
    console.log('Server Status:', healthResponse.data.message);
  } catch (error) {
    console.error('Error: MPGS Mock Server is not running. Please start it with "npm start" or "npm run dev"');
    return;
  }

  await example1_SuccessfulPayment();
  await new Promise(resolve => setTimeout(resolve, 1000)); // Wait 1 second between examples
  
  await example2_AuthorizeAndCapture();
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  await example3_DeclinedPayment();
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  await example4_InsufficientFunds();
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  await example5_3DSAuthentication();
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  await example6_RefundOperation();
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  await example7_GetOrderDetails();
  
  console.log('\n=== All Examples Completed ===');
}

// Run examples if this file is executed directly
if (require.main === module) {
  runAllExamples().catch(console.error);
}

module.exports = {
  example1_SuccessfulPayment,
  example2_AuthorizeAndCapture,
  example3_DeclinedPayment,
  example4_InsufficientFunds,
  example5_3DSAuthentication,
  example6_RefundOperation,
  example7_GetOrderDetails,
  runAllExamples
};
