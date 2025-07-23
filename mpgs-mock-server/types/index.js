// MPGS API Types based on reference documentation

// API Operation Types
const ApiOperation = {
  AUTHORIZE: 'AUTHORIZE',
  CAPTURE: 'CAPTURE',
  PAY: 'PAY',
  VOID_AUTHORIZATION: 'VOID_AUTHORIZATION',
  REFUND: 'REFUND'
};

// Gateway Response Codes
const GatewayCode = {
  APPROVED: 'APPROVED',
  APPROVED_AUTO: 'APPROVED_AUTO',
  APPROVED_PENDING_SETTLEMENT: 'APPROVED_PENDING_SETTLEMENT',
  DECLINED: 'DECLINED',
  DECLINED_INSUFFICIENT_FUNDS: 'INSUFFICIENT_FUNDS',
  DECLINED_INVALID_PIN: 'DECLINED_INVALID_PIN',
  DECLINED_AVS: 'DECLINED_AVS',
  DECLINED_CSC: 'DECLINED_CSC',
  EXPIRED_CARD: 'EXPIRED_CARD',
  AUTHENTICATION_FAILED: 'AUTHENTICATION_FAILED',
  AUTHENTICATION_IN_PROGRESS: 'AUTHENTICATION_IN_PROGRESS',
  BLOCKED: 'BLOCKED',
  CANCELLED: 'CANCELLED',
  PENDING: 'PENDING',
  SUBMITTED: 'SUBMITTED',
  SYSTEM_ERROR: 'SYSTEM_ERROR',
  TIMED_OUT: 'TIMED_OUT',
  UNKNOWN: 'UNKNOWN',
  UNSPECIFIED_FAILURE: 'UNSPECIFIED_FAILURE'
};

// Result Types
const Result = {
  SUCCESS: 'SUCCESS',
  PENDING: 'PENDING',
  FAILURE: 'FAILURE',
  UNKNOWN: 'UNKNOWN',
  ERROR: 'ERROR'
};

// Transaction Types
const TransactionType = {
  AUTHORIZATION: 'AUTHORIZATION',
  CAPTURE: 'CAPTURE',
  PAYMENT: 'PAYMENT',
  REFUND: 'REFUND',
  VOID_AUTHORIZATION: 'VOID_AUTHORIZATION',
  VOID_PAYMENT: 'VOID_PAYMENT',
  VERIFICATION: 'VERIFICATION'
};

// Error Causes
const ErrorCause = {
  INVALID_REQUEST: 'INVALID_REQUEST',
  REQUEST_REJECTED: 'REQUEST_REJECTED',
  SERVER_BUSY: 'SERVER_BUSY',
  SERVER_FAILED: 'SERVER_FAILED'
};

// Validation Types
const ValidationType = {
  INVALID: 'INVALID',
  MISSING: 'MISSING',
  UNSUPPORTED: 'UNSUPPORTED'
};

// Source of Funds Types
const SourceOfFundsType = {
  CARD: 'CARD'
};

// Request Types

/**
 * MPGS Payment Request
 */
class MpgsPaymentRequest {
  constructor() {
    this.apiOperation = '';
    this.order = new MpgsOrder();
    this.sourceOfFunds = new MpgsSourceOfFunds();
    this.transaction = new MpgsTransaction();
    this.customer = null;
    this.billing = null;
    this.shipping = null;
  }
}

/**
 * MPGS Order
 */
class MpgsOrder {
  constructor() {
    this.amount = '';
    this.currency = '';
    this.netAmount = null;
    this.reference = null;
    this.description = null;
  }
}

/**
 * MPGS Source of Funds
 */
class MpgsSourceOfFunds {
  constructor() {
    this.type = SourceOfFundsType.CARD;
    this.provided = new MpgsProvidedSourceOfFunds();
  }
}

/**
 * MPGS Provided Source of Funds
 */
class MpgsProvidedSourceOfFunds {
  constructor() {
    this.card = new MpgsCard();
  }
}

/**
 * MPGS Card
 */
class MpgsCard {
  constructor() {
    this.number = '';
    this.expiry = new MpgsExpiry();
    this.securityCode = '';
  }
}

/**
 * MPGS Card Expiry
 */
class MpgsExpiry {
  constructor() {
    this.month = '';
    this.year = '';
  }
}

/**
 * MPGS Transaction
 */
class MpgsTransaction {
  constructor() {
    this.reference = '';
    this.amount = null;
    this.currency = null;
  }
}

// Response Types

/**
 * MPGS Payment Response
 */
class MpgsPaymentResponse {
  constructor() {
    this.merchant = '';
    this.order = new MpgsOrderResponse();
    this.response = new MpgsResponseDetails();
    this.result = '';
    this.transaction = new MpgsTransactionResponse();
    this.timeOfRecord = '';
    this.version = '100';
  }
}

/**
 * MPGS Order Response
 */
class MpgsOrderResponse {
  constructor() {
    this.amount = 0;
    this.creationTime = '';
    this.currency = '';
    this.id = '';
    this.lastUpdatedTime = '';
    this.merchantAmount = 0;
    this.merchantCurrency = '';
    this.totalAuthorizedAmount = 0;
    this.totalCapturedAmount = 0;
    this.totalDisbursedAmount = 0;
    this.totalRefundedAmount = 0;
  }
}

/**
 * MPGS Response Details
 */
class MpgsResponseDetails {
  constructor() {
    this.gatewayCode = '';
    this.acquirerCode = null;
    this.acquirerMessage = null;
  }
}

/**
 * MPGS Transaction Response
 */
class MpgsTransactionResponse {
  constructor() {
    this.id = '';
    this.type = '';
    this.amount = 0;
    this.currency = '';
    this.reference = '';
    this.acquirer = new MpgsAcquirer();
  }
}

/**
 * MPGS Acquirer
 */
class MpgsAcquirer {
  constructor() {
    this.id = 'TEST_ACQUIRER';
    this.merchantId = '';
    this.transactionId = '';
  }
}

/**
 * MPGS Error Response
 */
class MpgsErrorResponse {
  constructor() {
    this.error = new MpgsError();
    this.result = Result.ERROR;
  }
}

/**
 * MPGS Error
 */
class MpgsError {
  constructor() {
    this.cause = '';
    this.explanation = '';
    this.field = null;
    this.supportCode = null;
    this.validationType = null;
  }
}

/**
 * MPGS Capture Request
 */
class MpgsCaptureRequest {
  constructor() {
    this.apiOperation = ApiOperation.CAPTURE;
    this.transaction = new MpgsTransaction();
  }
}

/**
 * MPGS Void Request
 */
class MpgsVoidRequest {
  constructor() {
    this.apiOperation = ApiOperation.VOID_AUTHORIZATION;
    this.transaction = new MpgsTransactionReference();
  }
}

/**
 * MPGS Transaction Reference
 */
class MpgsTransactionReference {
  constructor() {
    this.reference = '';
  }
}

/**
 * MPGS Refund Request
 */
class MpgsRefundRequest {
  constructor() {
    this.apiOperation = ApiOperation.REFUND;
    this.transaction = new MpgsTransaction();
  }
}

// Helper Functions

/**
 * Create a standard MPGS success response
 */
function createSuccessResponse(merchantId, orderId, transactionId, operation, amount, currency) {
  const response = new MpgsPaymentResponse();
  const now = new Date().toISOString();
  
  response.merchant = merchantId;
  response.result = Result.SUCCESS;
  response.timeOfRecord = now;
  
  // Order details
  response.order.id = orderId;
  response.order.amount = parseFloat(amount);
  response.order.currency = currency;
  response.order.merchantAmount = parseFloat(amount);
  response.order.merchantCurrency = currency;
  response.order.creationTime = now;
  response.order.lastUpdatedTime = now;
  
  // Transaction details
  response.transaction.id = transactionId;
  response.transaction.amount = parseFloat(amount);
  response.transaction.currency = currency;
  response.transaction.acquirer.merchantId = merchantId;
  response.transaction.acquirer.transactionId = transactionId;
  
  // Response details
  response.response.gatewayCode = GatewayCode.APPROVED;
  
  // Set transaction type and order amounts based on operation
  switch (operation) {
    case ApiOperation.AUTHORIZE:
      response.transaction.type = TransactionType.AUTHORIZATION;
      response.order.totalAuthorizedAmount = parseFloat(amount);
      break;
    case ApiOperation.PAY:
      response.transaction.type = TransactionType.PAYMENT;
      response.order.totalAuthorizedAmount = parseFloat(amount);
      response.order.totalCapturedAmount = parseFloat(amount);
      break;
    case ApiOperation.CAPTURE:
      response.transaction.type = TransactionType.CAPTURE;
      response.order.totalCapturedAmount = parseFloat(amount);
      break;
    case ApiOperation.VOID_AUTHORIZATION:
      response.transaction.type = TransactionType.VOID_AUTHORIZATION;
      response.order.totalAuthorizedAmount = 0;
      break;
    case ApiOperation.REFUND:
      response.transaction.type = TransactionType.REFUND;
      response.order.totalRefundedAmount = parseFloat(amount);
      break;
  }
  
  return response;
}

/**
 * Create a standard MPGS error response
 */
function createErrorResponse(cause, explanation, field = null, validationType = null) {
  const errorResponse = new MpgsErrorResponse();
  
  errorResponse.error.cause = cause;
  errorResponse.error.explanation = explanation;
  
  if (field) {
    errorResponse.error.field = field;
  }
  
  if (validationType) {
    errorResponse.error.validationType = validationType;
  }
  
  return errorResponse;
}

/**
 * Validate payment request
 */
function validatePaymentRequest(request) {
  const errors = [];
  
  if (!request.apiOperation) {
    errors.push({ field: 'apiOperation', message: 'API operation is required' });
  }
  
  if (!request.order) {
    errors.push({ field: 'order', message: 'Order is required' });
  } else {
    if (!request.order.amount && !request.order.netAmount) {
      errors.push({ field: 'order.amount', message: 'Either amount or netAmount must be provided' });
    }
    
    if (!request.order.currency) {
      errors.push({ field: 'order.currency', message: 'Currency is required' });
    }
  }
  
  // Check for valid API operations first
  const validOperations = [ApiOperation.AUTHORIZE, ApiOperation.PAY, ApiOperation.CAPTURE, ApiOperation.VOID_AUTHORIZATION, ApiOperation.REFUND];
  if (request.apiOperation && !validOperations.includes(request.apiOperation)) {
    errors.push({ field: 'apiOperation', message: `Unsupported API operation: ${request.apiOperation}` });
  }
  
  if (request.apiOperation !== ApiOperation.CAPTURE && request.apiOperation !== ApiOperation.VOID_AUTHORIZATION && request.apiOperation !== ApiOperation.REFUND) {
    if (!request.sourceOfFunds) {
      errors.push({ field: 'sourceOfFunds', message: 'Source of funds is required' });
    } else if (!request.sourceOfFunds.provided?.card) {
      errors.push({ field: 'sourceOfFunds.provided.card', message: 'Card details are required' });
    }
  }
  
  return errors;
}

module.exports = {
  // Enums
  ApiOperation,
  GatewayCode,
  Result,
  TransactionType,
  ErrorCause,
  ValidationType,
  SourceOfFundsType,
  
  // Request Classes
  MpgsPaymentRequest,
  MpgsOrder,
  MpgsSourceOfFunds,
  MpgsProvidedSourceOfFunds,
  MpgsCard,
  MpgsExpiry,
  MpgsTransaction,
  MpgsCaptureRequest,
  MpgsVoidRequest,
  MpgsRefundRequest,
  MpgsTransactionReference,
  
  // Response Classes
  MpgsPaymentResponse,
  MpgsOrderResponse,
  MpgsResponseDetails,
  MpgsTransactionResponse,
  MpgsAcquirer,
  MpgsErrorResponse,
  MpgsError,
  
  // Helper Functions
  createSuccessResponse,
  createErrorResponse,
  validatePaymentRequest
};
