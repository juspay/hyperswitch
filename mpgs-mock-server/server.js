const express = require('express');
const cors = require('cors');
const bodyParser = require('body-parser');
const { v4: uuidv4 } = require('uuid');

// Import types and middleware
const {
  ApiOperation,
  GatewayCode,
  Result,
  TransactionType,
  ErrorCause,
  ValidationType,
  createSuccessResponse,
  createErrorResponse,
  validatePaymentRequest
} = require('./types');

const { authenticateRequest, getCardBehavior } = require('./middleware/auth');

const app = express();
const PORT = process.env.PORT || 3001;

// Middleware
app.use(cors());
app.use(bodyParser.json());
app.use(bodyParser.urlencoded({ extended: true }));

// In-memory storage for orders and transactions
const orders = new Map();
const transactions = new Map();

// Helper function to store order/transaction data
function storeOrderTransaction(merchantId, orderId, transactionId, data) {
  const orderKey = `${merchantId}:${orderId}`;
  const transactionKey = `${merchantId}:${orderId}:${transactionId}`;
  
  console.log(`Storing order/transaction: ${orderKey} / ${transactionKey}`);
  
  if (!orders.has(orderKey)) {
    orders.set(orderKey, {
      id: orderId,
      merchantId,
      transactions: new Map(),
      creationTime: new Date().toISOString(),
      lastUpdatedTime: new Date().toISOString(),
      totalAuthorizedAmount: 0,
      totalCapturedAmount: 0,
      totalRefundedAmount: 0,
      amount: 0,
      currency: 'USD'
    });
    console.log(`Created new order: ${orderKey}`);
  }
  
  const order = orders.get(orderKey);
  order.lastUpdatedTime = new Date().toISOString();
  
  // Update order amount and currency from response data first
  if (data.order) {
    order.amount = data.order.amount;
    order.currency = data.order.currency;
    order.totalAuthorizedAmount = data.order.totalAuthorizedAmount || order.totalAuthorizedAmount;
    order.totalCapturedAmount = data.order.totalCapturedAmount || order.totalCapturedAmount;
    order.totalRefundedAmount = data.order.totalRefundedAmount || order.totalRefundedAmount;
  }
  
  order.transactions.set(transactionId, data);
  transactions.set(transactionKey, data);
  console.log(`Order count: ${orders.size}, Transaction count: ${transactions.size}`);
}

// Helper function to get stored order
function getStoredOrder(merchantId, orderId) {
  const orderKey = `${merchantId}:${orderId}`;
  console.log(`Looking for order: ${orderKey}, available orders: ${Array.from(orders.keys()).join(', ')}`);
  return orders.get(orderKey);
}

// Helper function to get stored transaction
function getStoredTransaction(merchantId, orderId, transactionId) {
  const transactionKey = `${merchantId}:${orderId}:${transactionId}`;
  console.log(`Looking for transaction: ${transactionKey}, available transactions: ${Array.from(transactions.keys()).join(', ')}`);
  return transactions.get(transactionKey);
}

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({ status: 'OK', message: 'MPGS Mock Server is running' });
});

// API Documentation endpoint
app.get('/api/docs', (req, res) => {
  res.json({
    name: 'MPGS Mock Server',
    version: '1.0.0',
    description: 'Mock implementation of MPGS payment gateway API',
    endpoints: {
      payment: 'PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}',
      getOrder: 'GET /api/rest/version/100/merchant/{merchantId}/order/{orderId}',
      getTransaction: 'GET /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}'
    },
    testCards: {
      success: ['4111111111111111', '5555555555554444'],
      decline: ['4000000000000002', '4000000000000119'],
      error: ['4000000000000069'],
      '3ds': ['4000000000000044']
    }
  });
});

// Payment Operations Endpoint
// PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
app.put('/api/rest/version/100/merchant/:merchantId/order/:orderId/transaction/:transactionId', 
  authenticateRequest, 
  (req, res) => {
    try {
      const { merchantId, orderId, transactionId } = req.params;
      const requestBody = req.body;
      
      console.log(`Processing ${requestBody.apiOperation} for merchant: ${merchantId}, order: ${orderId}, transaction: ${transactionId}`);
      
      // Validate request
      const validationErrors = validatePaymentRequest(requestBody);
      if (validationErrors.length > 0) {
        const firstError = validationErrors[0];
        return res.status(400).json(createErrorResponse(
          ErrorCause.INVALID_REQUEST,
          firstError.message,
          firstError.field,
          ValidationType.MISSING
        ));
      }
      
      // Process based on API operation
      switch (requestBody.apiOperation) {
        case ApiOperation.AUTHORIZE:
          return handleAuthorize(req, res, merchantId, orderId, transactionId, requestBody);
        
        case ApiOperation.PAY:
          return handlePay(req, res, merchantId, orderId, transactionId, requestBody);
        
        case ApiOperation.CAPTURE:
          return handleCapture(req, res, merchantId, orderId, transactionId, requestBody);
        
        case ApiOperation.VOID:
        case ApiOperation.VOID_AUTHORIZATION:
        case ApiOperation.VOID_PAYMENT:
        case ApiOperation.VOID_CAPTURE:
        case ApiOperation.VOID_REFUND:
          return handleVoid(req, res, merchantId, orderId, transactionId, requestBody);
        
        case ApiOperation.REFUND:
          return handleRefund(req, res, merchantId, orderId, transactionId, requestBody);
        
        case ApiOperation.VERIFY:
          return handleVerify(req, res, merchantId, orderId, transactionId, requestBody);
        
        case ApiOperation.DISBURSEMENT:
          return handleDisbursement(req, res, merchantId, orderId, transactionId, requestBody);
        
        default:
          return res.status(400).json(createErrorResponse(
            ErrorCause.INVALID_REQUEST,
            `Unsupported API operation: ${requestBody.apiOperation}`,
            'apiOperation',
            ValidationType.UNSUPPORTED
          ));
      }
      
    } catch (error) {
      console.error('Error processing payment request:', error);
      return res.status(500).json(createErrorResponse(
        ErrorCause.SERVER_FAILED,
        'Internal server error occurred'
      ));
    }
  }
);

// Handle Authorize operation
function handleAuthorize(req, res, merchantId, orderId, transactionId, requestBody) {
  const { order, sourceOfFunds } = requestBody;
  const amount = order.amount || order.netAmount;
  const currency = order.currency;
  
  // Check card behavior for test scenarios
  if (sourceOfFunds?.provided?.card?.number) {
    const cardBehavior = getCardBehavior(sourceOfFunds.provided.card.number);
    
    if (cardBehavior.behavior === 'decline') {
      const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.AUTHORIZE, amount, currency);
      response.result = Result.FAILURE;
      response.response.gatewayCode = cardBehavior.gatewayCode;
      
      storeOrderTransaction(merchantId, orderId, transactionId, response);
      return res.status(200).json(response);
    }
    
    if (cardBehavior.behavior === 'error') {
      return res.status(400).json(createErrorResponse(
        ErrorCause.INVALID_REQUEST,
        `Card validation failed: ${cardBehavior.gatewayCode}`,
        'sourceOfFunds.provided.card.number'
      ));
    }
    
    if (cardBehavior.behavior === 'pending') {
      const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.AUTHORIZE, amount, currency);
      response.result = Result.PENDING;
      response.response.gatewayCode = GatewayCode.PENDING;
      
      storeOrderTransaction(merchantId, orderId, transactionId, response);
      return res.status(200).json(response);
    }
    
    if (cardBehavior.behavior === '3ds') {
      const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.AUTHORIZE, amount, currency);
      response.result = Result.PENDING;
      response.response.gatewayCode = GatewayCode.AUTHENTICATION_IN_PROGRESS;
      
      storeOrderTransaction(merchantId, orderId, transactionId, response);
      return res.status(200).json(response);
    }
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.AUTHORIZE, amount, currency);
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Handle Pay operation
function handlePay(req, res, merchantId, orderId, transactionId, requestBody) {
  const { order, sourceOfFunds } = requestBody;
  const amount = order.amount || order.netAmount;
  const currency = order.currency;
  
  // Check card behavior for test scenarios
  if (sourceOfFunds?.provided?.card?.number) {
    const cardBehavior = getCardBehavior(sourceOfFunds.provided.card.number);
    
    if (cardBehavior.behavior === 'decline') {
      const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.PAY, amount, currency);
      response.result = Result.FAILURE;
      response.response.gatewayCode = cardBehavior.gatewayCode;
      
      storeOrderTransaction(merchantId, orderId, transactionId, response);
      return res.status(200).json(response);
    }
    
    if (cardBehavior.behavior === 'error') {
      return res.status(400).json(createErrorResponse(
        ErrorCause.INVALID_REQUEST,
        `Card validation failed: ${cardBehavior.gatewayCode}`,
        'sourceOfFunds.provided.card.number'
      ));
    }
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.PAY, amount, currency);
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Handle Capture operation
function handleCapture(req, res, merchantId, orderId, transactionId, requestBody) {
  const { transaction } = requestBody;
  const amount = transaction.amount;
  const currency = transaction.currency;
  
  // Check if there's an existing order
  const existingOrder = getStoredOrder(merchantId, orderId);
  if (!existingOrder) {
    return res.status(400).json(createErrorResponse(
      ErrorCause.INVALID_REQUEST,
      'Order not found for capture operation',
      'orderId'
    ));
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.CAPTURE, amount, currency || existingOrder.currency);
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Handle Void operation
function handleVoid(req, res, merchantId, orderId, transactionId, requestBody) {
  // Check if there's an existing order
  const existingOrder = getStoredOrder(merchantId, orderId);
  if (!existingOrder) {
    return res.status(400).json(createErrorResponse(
      ErrorCause.INVALID_REQUEST,
      'Order not found for void operation',
      'orderId'
    ));
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.VOID_AUTHORIZATION, 0, existingOrder.currency);
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Handle Refund operation
function handleRefund(req, res, merchantId, orderId, transactionId, requestBody) {
  const { transaction } = requestBody;
  const amount = transaction.amount;
  const currency = transaction.currency;
  
  // Check if there's an existing order
  const existingOrder = getStoredOrder(merchantId, orderId);
  if (!existingOrder) {
    return res.status(400).json(createErrorResponse(
      ErrorCause.INVALID_REQUEST,
      'Order not found for refund operation',
      'orderId'
    ));
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.REFUND, amount, currency || existingOrder.currency);
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Handle Verify operation
function handleVerify(req, res, merchantId, orderId, transactionId, requestBody) {
  const { order, sourceOfFunds } = requestBody;
  const currency = order.currency;
  
  // Check card behavior for test scenarios
  if (sourceOfFunds?.provided?.card?.number) {
    const cardBehavior = getCardBehavior(sourceOfFunds.provided.card.number);
    
    if (cardBehavior.behavior === 'decline') {
      const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.VERIFY, 0, currency);
      response.result = Result.FAILURE;
      response.response.gatewayCode = cardBehavior.gatewayCode;
      response.transaction.type = TransactionType.VERIFICATION;
      
      storeOrderTransaction(merchantId, orderId, transactionId, response);
      return res.status(200).json(response);
    }
    
    if (cardBehavior.behavior === 'error') {
      return res.status(400).json(createErrorResponse(
        ErrorCause.INVALID_REQUEST,
        `Card validation failed: ${cardBehavior.gatewayCode}`,
        'sourceOfFunds.provided.card.number'
      ));
    }
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.VERIFY, 0, currency);
  response.transaction.type = TransactionType.VERIFICATION;
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Handle Disbursement operation
function handleDisbursement(req, res, merchantId, orderId, transactionId, requestBody) {
  const { order, sourceOfFunds, disbursementType } = requestBody;
  const amount = order.amount;
  const currency = order.currency;
  
  // Validate disbursement type
  const validDisbursementTypes = ['GAMING_WINNINGS', 'CREDIT_CARD_BILL_PAYMENT'];
  if (!validDisbursementTypes.includes(disbursementType)) {
    return res.status(400).json(createErrorResponse(
      ErrorCause.INVALID_REQUEST,
      `Invalid disbursement type: ${disbursementType}`,
      'disbursementType',
      ValidationType.INVALID
    ));
  }
  
  // Check card behavior for test scenarios
  if (sourceOfFunds?.provided?.card?.number) {
    const cardBehavior = getCardBehavior(sourceOfFunds.provided.card.number);
    
    if (cardBehavior.behavior === 'decline') {
      const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.DISBURSEMENT, amount, currency);
      response.result = Result.FAILURE;
      response.response.gatewayCode = cardBehavior.gatewayCode;
      response.transaction.type = TransactionType.DISBURSEMENT;
      
      storeOrderTransaction(merchantId, orderId, transactionId, response);
      return res.status(200).json(response);
    }
    
    if (cardBehavior.behavior === 'error') {
      return res.status(400).json(createErrorResponse(
        ErrorCause.INVALID_REQUEST,
        `Card validation failed: ${cardBehavior.gatewayCode}`,
        'sourceOfFunds.provided.card.number'
      ));
    }
  }
  
  // Success case
  const response = createSuccessResponse(merchantId, orderId, transactionId, ApiOperation.DISBURSEMENT, amount, currency);
  response.transaction.type = TransactionType.DISBURSEMENT;
  response.order.totalDisbursedAmount = parseFloat(amount);
  storeOrderTransaction(merchantId, orderId, transactionId, response);
  
  return res.status(200).json(response);
}

// Get Order Details
// GET /api/rest/version/100/merchant/{merchantId}/order/{orderId}
app.get('/api/rest/version/100/merchant/:merchantId/order/:orderId', 
  authenticateRequest, 
  (req, res) => {
    try {
      const { merchantId, orderId } = req.params;
      
      const order = getStoredOrder(merchantId, orderId);
      if (!order) {
        return res.status(404).json(createErrorResponse(
          ErrorCause.INVALID_REQUEST,
          'Order not found',
          'orderId'
        ));
      }
      
      // Build order response with transactions
      const orderResponse = {
        merchant: merchantId,
        order: {
          id: order.id,
          amount: order.amount,
          currency: order.currency,
          creationTime: order.creationTime,
          lastUpdatedTime: order.lastUpdatedTime,
          merchantAmount: order.amount,
          merchantCurrency: order.currency,
          totalAuthorizedAmount: order.totalAuthorizedAmount,
          totalCapturedAmount: order.totalCapturedAmount,
          totalDisbursedAmount: 0,
          totalRefundedAmount: order.totalRefundedAmount
        },
        result: Result.SUCCESS,
        timeOfRecord: new Date().toISOString(),
        version: '100'
      };
      
      return res.status(200).json(orderResponse);
      
    } catch (error) {
      console.error('Error retrieving order:', error);
      return res.status(500).json(createErrorResponse(
        ErrorCause.SERVER_FAILED,
        'Internal server error occurred'
      ));
    }
  }
);

// Get Transaction Details
// GET /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
app.get('/api/rest/version/100/merchant/:merchantId/order/:orderId/transaction/:transactionId', 
  authenticateRequest, 
  (req, res) => {
    try {
      const { merchantId, orderId, transactionId } = req.params;
      
      const transaction = getStoredTransaction(merchantId, orderId, transactionId);
      if (!transaction) {
        return res.status(404).json(createErrorResponse(
          ErrorCause.INVALID_REQUEST,
          'Transaction not found',
          'transactionId'
        ));
      }
      
      return res.status(200).json(transaction);
      
    } catch (error) {
      console.error('Error retrieving transaction:', error);
      return res.status(500).json(createErrorResponse(
        ErrorCause.SERVER_FAILED,
        'Internal server error occurred'
      ));
    }
  }
);

// Error handling middleware
app.use((err, req, res, next) => {
  console.error('Unhandled error:', err);
  res.status(500).json(createErrorResponse(
    ErrorCause.SERVER_FAILED,
    'An unexpected error occurred'
  ));
});

// 404 handler
app.use((req, res) => {
  res.status(404).json(createErrorResponse(
    ErrorCause.INVALID_REQUEST,
    `Endpoint not found: ${req.method} ${req.path}`,
    'path'
  ));
});

// Start server only if this file is run directly (not imported)
if (require.main === module) {
  app.listen(PORT, () => {
    console.log(`MPGS Mock Server is running on port ${PORT}`);
    console.log(`Health check: http://localhost:${PORT}/health`);
    console.log(`API Documentation: http://localhost:${PORT}/api/docs`);
    console.log(`Base URL: http://localhost:${PORT}/api/rest/version/100`);
  });
}

module.exports = app;
