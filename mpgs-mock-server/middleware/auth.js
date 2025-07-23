const auth = require('basic-auth');

/**
 * Authentication middleware for MPGS API
 * Expected format: merchant.{merchantId}:{password}
 */
function authenticateRequest(req, res, next) {
  const credentials = auth(req);
  
  if (!credentials) {
    return res.status(401).json({
      error: {
        cause: 'REQUEST_REJECTED',
        explanation: 'Authentication credentials are required'
      },
      result: 'ERROR'
    });
  }
  
  // Parse the username which should be in format: merchant.{merchantId}
  const usernameParts = credentials.name.split('.');
  
  if (usernameParts.length !== 2 || usernameParts[0] !== 'merchant') {
    return res.status(401).json({
      error: {
        cause: 'REQUEST_REJECTED',
        explanation: 'Invalid authentication format. Expected format: merchant.{merchantId}'
      },
      result: 'ERROR'
    });
  }
  
  const merchantId = usernameParts[1];
  const password = credentials.pass;
  
  // Validate merchant ID matches the URL parameter
  if (req.params.merchantId && req.params.merchantId !== merchantId) {
    return res.status(401).json({
      error: {
        cause: 'REQUEST_REJECTED',
        explanation: 'Merchant ID in authentication does not match URL parameter'
      },
      result: 'ERROR'
    });
  }
  
  // For demo purposes, accept any password for test merchants
  // In production, this would validate against a secure credential store
  if (!password) {
    return res.status(401).json({
      error: {
        cause: 'REQUEST_REJECTED',
        explanation: 'Password is required'
      },
      result: 'ERROR'
    });
  }
  
  // Store merchant info in request for later use
  req.merchantId = merchantId;
  req.authenticated = true;
  
  next();
}

/**
 * Test card numbers and their expected behaviors based on MPGS specification
 */
const TEST_CARDS = {
  // Standard test cards - Success scenarios
  '5123450000000008': { behavior: 'success', gatewayCode: 'APPROVED' }, // Mastercard
  '2223000000000007': { behavior: 'success', gatewayCode: 'APPROVED' }, // Mastercard
  '5111111111111118': { behavior: 'success', gatewayCode: 'APPROVED' }, // Mastercard
  '2223000000000023': { behavior: 'success', gatewayCode: 'APPROVED' }, // Mastercard
  '4508750015741019': { behavior: 'success', gatewayCode: 'APPROVED' }, // Visa
  '4012000033330026': { behavior: 'success', gatewayCode: 'APPROVED' }, // Visa
  '30123400000000': { behavior: 'success', gatewayCode: 'APPROVED' },   // Diners Club
  '36259600000012': { behavior: 'success', gatewayCode: 'APPROVED' },   // Diners Club
  '3528000000000007': { behavior: 'success', gatewayCode: 'APPROVED' }, // JCB
  '3528111100000001': { behavior: 'success', gatewayCode: 'APPROVED' }, // JCB
  '6011003179988686': { behavior: 'success', gatewayCode: 'APPROVED' }, // Discover
  '6011963280099774': { behavior: 'success', gatewayCode: 'APPROVED' }, // Discover
  '5000000000000000005': { behavior: 'success', gatewayCode: 'APPROVED' }, // Maestro
  '5666555544443333': { behavior: 'success', gatewayCode: 'APPROVED' }, // Maestro
  '135492354874528': { behavior: 'success', gatewayCode: 'APPROVED' },  // UATP
  '135420001569134': { behavior: 'success', gatewayCode: 'APPROVED' },  // UATP
  
  // Test cards with specific behaviors based on expiry date
  // 01/39 -> APPROVED (handled by default success case above)
  // 05/39 -> DECLINED
  '4000000000000002': { behavior: 'decline', gatewayCode: 'DECLINED' },
  '4000000000000119': { behavior: 'decline', gatewayCode: 'INSUFFICIENT_FUNDS' },
  '4000000000000127': { behavior: 'decline', gatewayCode: 'DECLINED_CSC' },
  '4000000000000010': { behavior: 'decline', gatewayCode: 'DECLINED_AVS' },
  
  // 04/27 -> EXPIRED_CARD
  '4000000000000069': { behavior: 'error', gatewayCode: 'EXPIRED_CARD' },
  
  // 08/28 -> TIMED_OUT
  '4000000000000036': { behavior: 'pending', gatewayCode: 'TIMED_OUT' },
  
  // 01/37 -> ACQUIRER_SYSTEM_ERROR
  '4000000000000101': { behavior: 'error', gatewayCode: 'ACQUIRER_SYSTEM_ERROR' },
  
  // 02/37 -> UNSPECIFIED_FAILURE
  '4000000000000200': { behavior: 'error', gatewayCode: 'UNSPECIFIED_FAILURE' },
  
  // 05/37 -> UNKNOWN
  '4000000000000201': { behavior: 'pending', gatewayCode: 'UNKNOWN' },
  
  // 3DS scenarios
  '4000000000000044': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  
  // UnionPay 3DS enrolled
  '6201089999995464': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6201089999991455': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6201089999994020': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6201089999999300': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6201089999994749': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  
  // UnionPay Non-3DS enrolled
  '6214239999999611': { behavior: 'success', gatewayCode: 'APPROVED' },
  '6214239999999546': { behavior: 'success', gatewayCode: 'APPROVED' },
  
  // PayPak
  '2205459999997832': { behavior: 'success', gatewayCode: 'APPROVED' },
  '2205439999999541': { behavior: 'success', gatewayCode: 'APPROVED' },
  
  // Jaywan 3DS enrolled
  '6690109900000010': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6690109000011008': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6690109000011016': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6690109000011024': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  '6690109000011032': { behavior: '3ds', gatewayCode: 'AUTHENTICATION_IN_PROGRESS' },
  
  // Jaywan 3DS not enrolled
  '6690109000011057': { behavior: 'success', gatewayCode: 'APPROVED' },
  '6690109000011065': { behavior: 'success', gatewayCode: 'APPROVED' },
  
  // EFTPOS Australia test cards
  '5555229999999975': { behavior: 'success', gatewayCode: 'APPROVED' },
  '5555229999997722': { behavior: 'success', gatewayCode: 'APPROVED' },
  '4043409999991437': { behavior: 'success', gatewayCode: 'APPROVED' },
  '4029939999997636': { behavior: 'success', gatewayCode: 'APPROVED' },
  
  // Verve test cards - Nigeria
  '5060990580000217499': { behavior: 'success', gatewayCode: 'APPROVED' },
  '5079539999990592': { behavior: 'success', gatewayCode: 'APPROVED' },
  
  // Additional decline scenarios for testing
  '4000000000000259': { behavior: 'decline', gatewayCode: 'DECLINED_DO_NOT_CONTACT' },
  '4000000000000267': { behavior: 'decline', gatewayCode: 'DECLINED_INVALID_PIN' },
  '4000000000000275': { behavior: 'decline', gatewayCode: 'DECLINED_PIN_REQUIRED' },
  '4000000000000341': { behavior: 'decline', gatewayCode: 'REFERRED' },
  
  // Error scenarios
  '4000000000000077': { behavior: 'error', gatewayCode: 'INVALID_CSC' },
  '4000000000000085': { behavior: 'error', gatewayCode: 'SYSTEM_ERROR' },
  '4000000000000093': { behavior: 'error', gatewayCode: 'NOT_SUPPORTED' },
  
  // Pending scenarios
  '4000000000000051': { behavior: 'pending', gatewayCode: 'PENDING' },
  '4000000000000184': { behavior: 'pending', gatewayCode: 'SUBMITTED' }
};

/**
 * Get card behavior based on card number
 */
function getCardBehavior(cardNumber) {
  // Remove spaces and dashes
  const cleanCardNumber = cardNumber.replace(/[\s-]/g, '');
  
  return TEST_CARDS[cleanCardNumber] || { behavior: 'success', gatewayCode: 'APPROVED' };
}

module.exports = {
  authenticateRequest,
  getCardBehavior,
  TEST_CARDS
};
