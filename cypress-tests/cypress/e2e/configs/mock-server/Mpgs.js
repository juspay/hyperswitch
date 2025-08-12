/* eslint-disable */
import express from "express";
import cors from "cors";

const app = express();
const PORT = process.env.PORT || 3012;

// Middleware
app.use(cors());
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Mock data storage
const mockData = {
  orders: {},
  transactions: {},
};

// Mock API credentials
const validCredentials = {
  "merchant.TEST123456": "test-password",
  "merchant.TESTMPGS": "mpgs-secret",
  "merchant.DEMO": "demo-password",
};

// Helper functions
function getCurrentTimestamp() {
  return new Date().toISOString();
}

// MPGS Test Cards Data
const testCards = {
  // Standard test cards - Success scenarios
  success: [
    "5123450000000008", // Mastercard
    "2223000000000007", // Mastercard
    "5111111111111118", // Mastercard
    "2223000000000023", // Mastercard
    "4508750015741019", // Visa
    "4012000033330026", // Visa
    "30123400000000", // Diners Club
    "36259600000012", // Diners Club
    "3528000000000007", // JCB
    "3528111100000001", // JCB
    "6011003179988686", // Discover
    "6011963280099774", // Discover
    "5000000000000000005", // Maestro
    "5666555544443333", // Maestro
    "135492354874528", // UATP
    "135420001569134", // UATP
  ],
  // Decline scenarios
  decline: ["4000000000000002"],
  // Expired card scenarios
  expired: ["4000000000000069"],
  // Insufficient funds
  insufficientFunds: ["4000000000000119"],
};

// Authentication middleware
function authenticateBasic(req, res, next) {
  const authHeader = req.headers.authorization;

  if (!authHeader || !authHeader.startsWith("Basic ")) {
    return res.status(401).json({
      error: {
        cause: "AUTHENTICATION_FAILED",
        explanation: "Missing or invalid Authorization header",
      },
      result: "ERROR",
    });
  }

  try {
    const base64Credentials = authHeader.split(" ")[1];
    const credentials = Buffer.from(base64Credentials, "base64").toString(
      "ascii"
    );
    const [merchantAuth, password] = credentials.split(":");
    if (
      !validCredentials[merchantAuth] ||
      validCredentials[merchantAuth] !== password
    ) {
      return res.status(401).json({
        error: {
          cause: "AUTHENTICATION_FAILED",
          explanation: "Invalid merchant credentials",
        },
        result: "ERROR",
      });
    }

    req.merchantAuth = merchantAuth;
    next();
  } catch (error) {
    return res.status(401).json({
      error: {
        cause: "AUTHENTICATION_FAILED",
        explanation: `Invalid authorization format: ${error}`,
      },
      result: "ERROR",
    });
  }
}

// Logging middleware
app.use((req, res, next) => {
  console.log(`${new Date().toISOString()} - ${req.method} ${req.path}`);
  console.log("Headers:", JSON.stringify(req.headers, null, 2));
  if (req.body && Object.keys(req.body).length > 0) {
    console.log("Body:", JSON.stringify(req.body, null, 2));
  }
  next();
});

// Health check endpoint
app.get("/health", (req, res) => {
  res.json({
    status: "OK",
    timestamp: getCurrentTimestamp(),
    service: "MPGS Mock Server",
  });
});

// Helper function to determine card response based on test card data
function getCardResponse(cardNumber, expiryDate, amount) {
  // Check expiry date for specific responses
  // if (expiryDate) {
  //   const [month, year] = expiryDate.split("/");
  //   if (month === "05" && year === "39") {
  //     return { result: "FAILURE", gatewayCode: "DECLINED" };
  //   }
  //   if (month === "04" && year === "27") {
  //     return { result: "FAILURE", gatewayCode: "EXPIRED_CARD" };
  //   }
  //   if (month === "08" && year === "28") {
  //     return { result: "PENDING", gatewayCode: "TIMED_OUT" };
  //   }
  //   if (month === "01" && year === "37") {
  //     return { result: "FAILURE", gatewayCode: "ACQUIRER_SYSTEM_ERROR" };
  //   }
  //   if (month === "02" && year === "37") {
  //     return { result: "FAILURE", gatewayCode: "UNSPECIFIED_FAILURE" };
  //   }
  //   if (month === "05" && year === "37") {
  //     return { result: "FAILURE", gatewayCode: "UNKNOWN" };
  //   }
  // }

  // Check specific test amounts for simulation behavior
  if (amount) {
    const amountValue = parseFloat(amount);
    if (amountValue === 1.2) {
      return { result: "FAILURE", gatewayCode: "INSUFFICIENT_FUNDS" };
    }
    if (amountValue === 8.88) {
      return { result: "PENDING", gatewayCode: "TIMED_OUT" };
    }
    if (amountValue === 6.66) {
      return { result: "FAILURE", gatewayCode: "EXPIRED_CARD" };
    }
  }

  // // Check card number patterns
  // if (testCards.decline.includes(cardNumber)) {
  //   return { result: "FAILURE", gatewayCode: "DECLINED" };
  // }
  // if (testCards.expired.includes(cardNumber)) {
  //   return { result: "FAILURE", gatewayCode: "EXPIRED_CARD" };
  // }
  // if (testCards.insufficientFunds.includes(cardNumber)) {
  //   return { result: "FAILURE", gatewayCode: "INSUFFICIENT_FUNDS" };
  // }

  // Default success for valid test cards
  // if (testCards.success.includes(cardNumber)) {
    return { result: "SUCCESS", gatewayCode: "APPROVED" };
  // }

  // Unknown card - decline
  return { result: "FAILURE", gatewayCode: "DECLINED" };
}

// Helper function to handle PAY/AUTHORIZE operations
function handlePaymentOperation(
  apiOperation,
  order,
  sourceOfFunds,
  transactionId,
  orderId,
  merchantId
) {
  const cardNumber = sourceOfFunds.provided?.card?.number;
  const expiry = sourceOfFunds.provided?.card?.expiry;
  const expiryDate = expiry ? `${expiry.month}/${expiry.year}` : null;
  const cardResponse = getCardResponse(cardNumber, expiryDate, order.amount);
  const now = getCurrentTimestamp();

  const response = {
    result: cardResponse.result,
    transaction: {
      id: transactionId,
      type: apiOperation === "PAY" ? "PAYMENT" : "AUTHORIZATION",
    },
    response: {
      gatewayCode: cardResponse.gatewayCode,
    },
    order: {
      amount: parseFloat(order.amount),
      currency: order.currency,
    },
  };
  console.log(`Handling ${apiOperation} operation for order ${orderId}, transaction ${transactionId}`);
  console.log("Response:", JSON.stringify(response, null, 2));
  // Store the transaction
  mockData.transactions[transactionId] = {
    ...response,
    orderId,
    merchantId,
    created: now,
  };

  // Store order data
  if (!mockData.orders[orderId]) {
    mockData.orders[orderId] = {
      id: orderId,
      amount: parseFloat(order.amount),
      currency: order.currency,
      transactions: [],
      created: now,
    };
  }
  mockData.orders[orderId].transactions.push(transactionId);

  return response;
}

// Helper function to handle CAPTURE operation
function handleCaptureOperation(
  transaction,
  transactionId,
  orderId,
  merchantId
) {
  const authTransaction = Object.values(mockData.transactions).find(
    (t) => t.orderId === orderId && t.transaction.type === "AUTHORIZATION"
  );

  if (!authTransaction) {
    return {
      error: {
        cause: "INVALID_REQUEST",
        explanation: "No authorization found for this order",
      },
      result: "ERROR",
    };
  }

  const now = getCurrentTimestamp();
  const response = {
    result: "SUCCESS",
    transaction: {
      id: transactionId,
      type: "CAPTURE",
    },
    response: {
      gatewayCode: "APPROVED",
    },
    order: {
      amount: transaction?.amount
        ? parseFloat(transaction.amount)
        : authTransaction.order.amount,
      currency: transaction?.currency || authTransaction.order.currency,
    },
  };

  mockData.transactions[transactionId] = {
    ...response,
    orderId,
    merchantId,
    created: now,
  };

  return response;
}

// Helper function to handle VOID operation
function handleVoidOperation(order, transactionId, orderId, merchantId) {
  const now = getCurrentTimestamp();
  const response = {
    result: "SUCCESS",
    transaction: {
      id: transactionId,
      type: "VOID",
    },
    response: {
      gatewayCode: "APPROVED",
    },
    order: {
      amount: 0,
      currency: order?.currency || "USD",
    },
  };

  mockData.transactions[transactionId] = {
    ...response,
    orderId,
    merchantId,
    created: now,
  };

  return response;
}

// Helper function to handle REFUND operation
function handleRefundOperation(
  transaction,
  transactionId,
  orderId,
  merchantId
) {
  const now = getCurrentTimestamp();
  const response = {
    result: "SUCCESS",
    transaction: {
      id: transactionId,
      type: "REFUND",
    },
    response: {
      gatewayCode: "APPROVED",
    },
    order: {
      amount: transaction?.amount ? parseFloat(transaction.amount) : 0,
      currency: transaction?.currency || "USD",
    },
  };

  mockData.transactions[transactionId] = {
    ...response,
    orderId,
    merchantId,
    created: now,
  };

  return response;
}

// 1. PUT /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
// This endpoint handles PAY, AUTHORIZE, CAPTURE, VOID, and REFUND operations
app.put(
  "/api/rest/version/100/merchant/:merchantId/order/:orderId/transaction/:transactionId",
  authenticateBasic,
  (req, res) => {
    try {
      const { merchantId, orderId, transactionId } = req.params;
      const { apiOperation, order, sourceOfFunds, transaction } = req.body;

      console.log(
        `Processing ${apiOperation} operation for order ${orderId}, transaction ${transactionId}`
      );
      // Basic validation
      if (!apiOperation) {
        return res.status(400).json({
          error: {
            cause: "INVALID_REQUEST",
            explanation: "Missing required field: apiOperation",
          },
          result: "ERROR",
        });
      }

      let response;

      if (apiOperation === "PAY" || apiOperation === "AUTHORIZE") {
        if (!order || !sourceOfFunds) {
          return res.status(400).json({
            error: {
              cause: "INVALID_REQUEST",
              explanation: "Missing required fields: order, sourceOfFunds",
            },
            result: "ERROR",
          });
        }
        response = handlePaymentOperation(
          apiOperation,
          order,
          sourceOfFunds,
          transactionId,
          orderId,
          merchantId
        );
      } else if (apiOperation === "CAPTURE") {
        response = handleCaptureOperation(
          transaction,
          transactionId,
          orderId,
          merchantId
        );
        if (response.error) {
          return res.status(404).json(response);
        }
      } else if (apiOperation === "VOID") {
        response = handleVoidOperation(
          order,
          transactionId,
          orderId,
          merchantId
        );
      } else if (apiOperation === "REFUND") {
        response = handleRefundOperation(
          transaction,
          transactionId,
          orderId,
          merchantId
        );
      } else {
        return res.status(400).json({
          error: {
            cause: "INVALID_REQUEST",
            explanation: `Unsupported apiOperation: ${apiOperation}`,
          },
          result: "ERROR",
        });
      }

      res.status(200).json(response);
    } catch (error) {
      console.error("Error processing transaction:", error);
      res.status(500).json({
        error: {
          cause: "SYSTEM_ERROR",
          explanation: error.message,
        },
        result: "ERROR",
      });
    }
  }
);

// 2. GET /api/rest/version/100/merchant/{merchantId}/order/{orderId}
// Get order status (payment sync)
app.get(
  "/api/rest/version/100/merchant/:merchantId/order/:orderId",
  authenticateBasic,
  (req, res) => {
    try {
      const { orderId } = req.params;

      const order = mockData.orders[orderId];
      if (!order) {
        return res.status(404).json({
          error: {
            cause: "INVALID_REQUEST",
            explanation: "Order not found",
          },
          result: "ERROR",
        });
      }

      // Get the latest transaction for this order
      const latestTransactionId =
        order.transactions[order.transactions.length - 1];
      const latestTransaction = mockData.transactions[latestTransactionId];

      if (!latestTransaction) {
        return res.status(404).json({
          error: {
            cause: "INVALID_REQUEST",
            explanation: "No transactions found for this order",
          },
          result: "ERROR",
        });
      }

      res.json(latestTransaction);
    } catch (error) {
      console.error("Error retrieving order:", error);
      res.status(500).json({
        error: {
          cause: "SYSTEM_ERROR",
          explanation: error.message,
        },
        result: "ERROR",
      });
    }
  }
);

// 3. GET /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId}
// Get transaction status (refund sync)
app.get(
  "/api/rest/version/100/merchant/:merchantId/order/:orderId/transaction/:transactionId",
  authenticateBasic,
  (req, res) => {
    try {
      const { transactionId } = req.params;

      const transaction = mockData.transactions[transactionId];
      if (!transaction) {
        return res.status(404).json({
          error: {
            cause: "INVALID_REQUEST",
            explanation: "Transaction not found",
          },
          result: "ERROR",
        });
      }

      res.json(transaction);
    } catch (error) {
      console.error("Error retrieving transaction:", error);
      res.status(500).json({
        error: {
          cause: "SYSTEM_ERROR",
          explanation: error.message,
        },
        result: "ERROR",
      });
    }
  }
);

// Error handling middleware
// eslint-disable-next-line no-unused-vars
app.use((err, req, res, next) => {
  console.error("Error:", err);
  res.status(500).json({
    error: {
      cause: "SYSTEM_ERROR",
      explanation: "An unexpected error occurred",
    },
    result: "ERROR",
  });
});

// 404 handler
app.use((req, res) => {
  res.status(404).json({
    error: {
      cause: "INVALID_REQUEST",
      explanation: `Endpoint ${req.method} ${req.originalUrl} not found`,
    },
    result: "ERROR",
  });
});

// Start server
app.listen(PORT, () => {
  console.log(`🚀 MPGS Mock Server running on port ${PORT}`);
  console.log(`📍 Server URL: http://localhost:${PORT}`);
  console.log("\n📋 Available Endpoints:");
  console.log("  GET  /health - Health check");
  console.log(
    "  PUT  /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId} - Payment operations"
  );
  console.log(
    "  GET  /api/rest/version/100/merchant/{merchantId}/order/{orderId} - Get order status"
  );
  console.log(
    "  GET  /api/rest/version/100/merchant/{merchantId}/order/{orderId}/transaction/{transactionId} - Get transaction status"
  );
  console.log("\n🔐 Authentication (Basic Auth format):");
  console.log("  Format: merchant.{merchantId}:{password}");
  console.log("  Test Credentials:");
  console.log("    - merchant.TEST123456:test-password");
  console.log("    - merchant.TESTMPGS:mpgs-secret");
  console.log("    - merchant.DEMO:demo-password");
  console.log("\n💳 Test Cards:");
  console.log("  Success Cards:");
  console.log("    - 5123450000000008 (Mastercard)");
  console.log("    - 4508750015741019 (Visa)");
  console.log("    - 30123400000000 (Diners Club)");
  console.log("    - 3528000000000007 (JCB)");
  console.log("    - 6011003179988686 (Discover)");
  console.log("  Decline Cards:");
  console.log("    - 4000000000000002 (Generic decline)");
  console.log("\n📖 Special Expiry Dates for Testing:");
  console.log("  - 01/39: APPROVED");
  console.log("  - 05/39: DECLINED");
  console.log("  - 04/27: EXPIRED_CARD");
  console.log("  - 08/28: TIMED_OUT");
  console.log("  - 01/37: ACQUIRER_SYSTEM_ERROR");
  console.log("\n💰 Special Amounts for Testing:");
  console.log("  - 1.20: INSUFFICIENT_FUNDS");
  console.log("  - 8.88: TIMED_OUT");
  console.log("  - 6.66: EXPIRED_CARD");
  console.log("\n🔄 Supported Operations:");
  console.log("  - PAY (auto-capture payment)");
  console.log("  - AUTHORIZE (authorization only)");
  console.log("  - CAPTURE (capture authorized payment)");
  console.log("  - VOID (cancel authorization)");
  console.log("  - REFUND (refund payment)");
});

export default app;
