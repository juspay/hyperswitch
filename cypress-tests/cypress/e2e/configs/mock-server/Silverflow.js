/* eslint-disable no-console */
const express = require("express");
const cors = require("cors");

const app = express();
const PORT = process.env.PORT || 3010;

// Middleware
app.use(cors());
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Mock data storage
const mockData = {
  charges: {},
  disputes: {},
  processorTokens: {},
  eventSubscriptions: {},
  idempotencyKeys: new Set(),
};

// Mock API credentials
const validCredentials = {
  "apk-1wtRxni5IsPsSpBLWpwr": "FWtnOOHAjbD6rNxWWEeVOCj7JXSEPGJQ",
  "apk-testkey123": "testsecret456",
};

// Helper functions
function generateId(prefix) {
  return `${prefix}-${Math.random().toString(36).substr(2, 16)}`;
}

function generateAuthCode() {
  return Math.floor(100000 + Math.random() * 900000).toString();
}

function getCurrentTimestamp() {
  return new Date().toISOString();
}

// Authentication middleware
function authenticateBasic(req, res, next) {
  const authHeader = req.headers.authorization;

  if (!authHeader || !authHeader.startsWith("Basic ")) {
    return res.status(401).json({
      error: "Authentication required",
      message: "Missing or invalid Authorization header",
    });
  }

  try {
    const base64Credentials = authHeader.split(" ")[1];
    const credentials = Buffer.from(base64Credentials, "base64").toString(
      "ascii"
    );
    const [apiKey, apiSecret] = credentials.split(":");

    if (!validCredentials[apiKey] || validCredentials[apiKey] !== apiSecret) {
      return res.status(401).json({
        error: "Invalid credentials",
        message: "Invalid API key or secret",
      });
    }

    req.apiKey = apiKey;
    next();
  } catch (error) {
    return res.status(401).json({
      error: "Authentication error",
      message: `Invalid authorization format: ${error}`,
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
  res.json({ status: "OK", timestamp: getCurrentTimestamp() });
});

// 1. POST /charges - Create payment authorization
app.post("/charges", authenticateBasic, (req, res) => {
  try {
    const { merchantAcceptorResolver, card, amount, type, clearingMode } =
      req.body;

    // Basic validation
    if (!merchantAcceptorResolver || !card || !amount || !type) {
      return res.status(400).json({
        error: "Bad Request",
        message:
          "Missing required fields: merchantAcceptorResolver, card, amount, type",
      });
    }

    // Check idempotency key
    const idempotencyKey = req.headers["idempotency-key"];
    if (idempotencyKey && mockData.idempotencyKeys.has(idempotencyKey)) {
      return res.status(409).json({
        error: "Conflict",
        message: "Idempotency key already used",
      });
    }

    if (idempotencyKey) {
      mockData.idempotencyKeys.add(idempotencyKey);
    }

    const chargeKey = generateId("chg");
    const authCode = generateAuthCode();

    const now = getCurrentTimestamp();
    const charge = {
      key: chargeKey,
      merchantAcceptorRef: {
        key: generateId("mac"),
        version: 1,
      },
      card: {
        maskedNumber: `${card.number.substring(0, 6)}****${card.number.substring(card.number.length - 4)}`,
      },
      amount,
      type,
      clearingMode: clearingMode || "auto",
      status: {
        authentication: "none",
        authorization: "approved",
        clearing: "approved",
      },
      authentication: {
        sca: {
          compliance: "out-of-scope",
          complianceReason: "moto",
          method: "none",
        },
        cvc: "match",
        avs: "none",
      },
      localTransactionDateTime: now,
      fraudLiability: "acquirer",
      authorizationIsoFields: {
        responseCode: "00",
        responseCodeDescription: "Approved",
        authorizationCode: authCode,
        networkCode: "0000",
        systemTraceAuditNumber: Math.floor(
          100000 + Math.random() * 900000
        ).toString(),
        retrievalReferenceNumber: Math.floor(
          100000000000 + Math.random() * 900000000000
        ).toString(),
        eci: "01",
        networkSpecificFields: {
          transactionIdentifier: Math.floor(
            100000000000000 + Math.random() * 900000000000000
          ).toString(),
          cvv2ResultCode: "M",
        },
      },
      created: now,
      version: 1,
    };

    mockData.charges[chargeKey] = charge;

    res.status(201).json(charge);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 2. POST /charges/{chargeKey}/clear - Manually clear charge
app.post("/charges/:chargeKey/clear", authenticateBasic, (req, res) => {
  try {
    const { chargeKey } = req.params;
    const { amount, closeCharge, clearAfter, reference } = req.body;

    const charge = mockData.charges[chargeKey];
    if (!charge) {
      return res.status(404).json({
        error: "Not Found",
        message: "Charge not found",
      });
    }

    // Validate amount if provided
    if (
      amount &&
      (typeof amount !== "number" || amount < 1 || amount > 999999999999)
    ) {
      return res.status(400).json({
        error: "Bad Request",
        message: "Invalid amount: must be integer between 1 and 999999999999",
      });
    }

    // Check idempotency key
    const idempotencyKey = req.headers["idempotency-key"];
    if (idempotencyKey && mockData.idempotencyKeys.has(idempotencyKey)) {
      return res.status(409).json({
        error: "Conflict",
        message: "Idempotency key already used",
      });
    }

    if (idempotencyKey) {
      mockData.idempotencyKeys.add(idempotencyKey);
    }

    // Determine clearing amount - if not provided, use full charge amount
    const clearingAmount = amount || charge.amount.value;

    // Check if charge is too old (6 months) - mock validation
    const chargeDate = new Date(charge.created);
    const sixMonthsAgo = new Date();
    sixMonthsAgo.setMonth(sixMonthsAgo.getMonth() - 6);

    if (chargeDate < sixMonthsAgo) {
      return res.status(409).json({
        error: "Conflict",
        type: "/silverflow/problems/charge/too-old",
        message: "Charge is too old to be manually cleared",
      });
    }

    const actionKey = generateId("act");
    const clearAction = {
      type: "clearing",
      key: actionKey,
      chargeKey,
      status: "pending",
      reference,
      amount: {
        value: clearingAmount,
        currency: charge.amount.currency,
      },
      closeCharge: closeCharge || false,
      clearAfter: clearAfter || null,
      created: getCurrentTimestamp(),
      lastModified: getCurrentTimestamp(),
      version: 1,
    };

    // Update charge status if fully cleared or closeCharge is true
    if (closeCharge || clearingAmount >= charge.amount.value) {
      charge.status.clearing = "cleared";
    } else {
      charge.status.clearing = "partially_cleared";
    }

    charge.actions.push(clearAction);
    charge.version += 1;

    res.status(201).json(clearAction);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 3. POST /charges/{chargeKey}/refund - Process refunds
app.post("/charges/:chargeKey/refund", authenticateBasic, (req, res) => {
  try {
    const { chargeKey } = req.params;
    const { refundAmount, reference, clearAfter, dynamicDescriptor } = req.body;

    const charge = mockData.charges[chargeKey];
    if (!charge) {
      return res.status(404).json({
        error: "Not Found",
        message: "Charge not found",
      });
    }

    // If refundAmount not provided, refund the full charge amount
    const refundAmountValue = refundAmount || charge.amount.value;

    // Validate refundAmount if provided
    if (
      refundAmount &&
      (typeof refundAmount !== "number" ||
        refundAmount < 1 ||
        refundAmount > 999999999999)
    ) {
      return res.status(400).json({
        error: "Bad Request",
        message:
          "Invalid refundAmount: must be integer between 1 and 999999999999",
      });
    }

    // Check idempotency key
    const idempotencyKey = req.headers["idempotency-key"];
    if (idempotencyKey && mockData.idempotencyKeys.has(idempotencyKey)) {
      return res.status(409).json({
        error: "Conflict",
        message: "Idempotency key already used",
      });
    }

    if (idempotencyKey) {
      mockData.idempotencyKeys.add(idempotencyKey);
    }

    const actionKey = generateId("act");

    // Mock network response
    const networks = ["visa", "mastercard", "american-express", "discover"];
    const network = networks[Math.floor(Math.random() * networks.length)];

    const refundAction = {
      type: "refund",
      key: actionKey,
      chargeKey,
      reference,
      amount: {
        value: refundAmountValue,
        currency: charge.amount.currency,
      },
      status: {
        authorization: "approved",
      },
      clearAfter: clearAfter || null,
      authorizationResponse: {
        network,
        responseCode: "00",
        responseCodeDescription: "Approved",
      },
      dynamicDescriptor: dynamicDescriptor || null,
      created: getCurrentTimestamp(),
      lastModified: getCurrentTimestamp(),
      version: 1,
    };

    charge.actions.push(refundAction);
    charge.version += 1;

    res.status(201).json(refundAction);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 4. POST /charges/{chargeKey}/reverse - Reverse charge (void)
app.post("/charges/:chargeKey/reverse", authenticateBasic, (req, res) => {
  try {
    const { chargeKey } = req.params;
    const { replacementAmount = 0, reference } = req.body;

    const charge = mockData.charges[chargeKey];
    if (!charge) {
      return res.status(404).json({
        error: "Not Found",
        message: "Charge not found",
      });
    }

    // Validate replacementAmount if provided
    if (
      replacementAmount &&
      (typeof replacementAmount !== "number" ||
        replacementAmount < 0 ||
        replacementAmount > 999999999999)
    ) {
      return res.status(400).json({
        error: "Bad Request",
        message:
          "Invalid replacementAmount: must be integer between 0 and 999999999999",
      });
    }

    // Check idempotency key
    const idempotencyKey = req.headers["idempotency-key"];
    if (idempotencyKey && mockData.idempotencyKeys.has(idempotencyKey)) {
      return res.status(409).json({
        error: "Conflict",
        message: "Idempotency key already used",
      });
    }

    if (idempotencyKey) {
      mockData.idempotencyKeys.add(idempotencyKey);
    }

    const actionKey = generateId("act");

    // Mock network response
    const networks = ["visa", "mastercard", "american-express", "discover"];
    const network = networks[Math.floor(Math.random() * networks.length)];

    const reversalAction = {
      type: "reversal",
      key: actionKey,
      chargeKey,
      reference,
      replacementAmount: {
        value: replacementAmount,
        currency: charge.amount.currency,
      },
      status: {
        authorization: "approved",
      },
      authorizationResponse: {
        network,
        responseCode: "00",
        responseCodeDescription: "Approved",
      },
      created: getCurrentTimestamp(),
      lastModified: getCurrentTimestamp(),
      version: 1,
    };

    // Update charge status based on replacement amount
    if (replacementAmount === 0) {
      charge.status.authorization = "reversed";
    } else {
      charge.status.authorization = "partially_reversed";
      charge.amount.value = replacementAmount; // Update to replacement amount
    }

    charge.actions.push(reversalAction);
    charge.version += 1;

    res.status(201).json(reversalAction);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 5. GET /charges/{chargeKey} - Get charge status (sync)
app.get("/charges/:chargeKey", authenticateBasic, (req, res) => {
  try {
    const { chargeKey } = req.params;

    const charge = mockData.charges[chargeKey];
    if (!charge) {
      return res.status(404).json({
        error: "Not Found",
        message: "Charge not found",
      });
    }

    res.json(charge);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 6. GET /disputes/{disputeKey} - Get dispute information
app.get("/disputes/:disputeKey", authenticateBasic, (req, res) => {
  try {
    const { disputeKey } = req.params;

    let dispute = mockData.disputes[disputeKey];
    if (!dispute) {
      // Create a mock dispute if it doesn't exist
      dispute = {
        disputeKey,
        disputeStage: "chargeback",
        disputeStatus: "received",
        chargeKey: generateId("chg"),
        amount: {
          value: 1000,
          currency: "EUR",
        },
        reason: {
          code: "4855",
          description: "Goods or Services Not Provided",
          category: "consumer-dispute",
        },
        network: "mastercard",
        created: getCurrentTimestamp(),
      };
      mockData.disputes[disputeKey] = dispute;
    }

    res.json(dispute);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 7. POST /disputes/{disputeKey}/defend - Defend disputes
app.post("/disputes/:disputeKey/defend", authenticateBasic, (req, res) => {
  try {
    const { disputeKey } = req.params;
    const { disputeResponseReasonId, disputeSubResponseReasonId } = req.body;

    const dispute = mockData.disputes[disputeKey];
    if (!dispute) {
      return res.status(404).json({
        error: "Not Found",
        message: "Dispute not found",
      });
    }

    if (!disputeResponseReasonId) {
      return res.status(400).json({
        error: "Bad Request",
        message: "Missing disputeResponseReasonId",
      });
    }

    dispute.disputeStatus = "defended";
    dispute.defenseDetails = {
      disputeResponseReasonId,
      disputeSubResponseReasonId,
      defendedAt: getCurrentTimestamp(),
    };

    res.json({
      disputeKey,
      status: "defense_submitted",
      message: "Dispute defense submitted successfully",
      defendedAt: getCurrentTimestamp(),
    });
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 8. POST /processorTokens - Create processor tokens for vaulting
app.post("/processorTokens", authenticateBasic, (req, res) => {
  try {
    const { reference, cardData } = req.body;

    if (!cardData || !cardData.cardNumber) {
      return res.status(400).json({
        error: "Bad Request",
        message: "Missing cardData or cardNumber",
      });
    }

    const processorTokenKey = generateId("ptk");
    const cardNumber = cardData.cardNumber.toString();

    // Determine card network based on first digits
    let network = "unknown";
    if (cardNumber.startsWith("4")) network = "visa";
    else if (cardNumber.startsWith("5") || cardNumber.startsWith("2"))
      network = "mastercard";
    else if (cardNumber.startsWith("3")) network = "amex";
    else if (cardNumber.startsWith("6")) network = "discover";

    const processorToken = {
      processorTokenKey,
      cardInfo: {
        first6: cardNumber.substring(0, 6),
        last4: cardNumber.substring(cardNumber.length - 4),
        network,
        type: "credit",
      },
      cvcPresent: !!cardData.cvc,
      reference,
      created: getCurrentTimestamp(),
      version: 1,
    };

    mockData.processorTokens[processorTokenKey] = processorToken;

    res.status(201).json(processorToken);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 9. POST /eventSubscriptions - Create webhook subscriptions
app.post("/eventSubscriptions", authenticateBasic, (req, res) => {
  try {
    const { eventSource, notificationUrl, status } = req.body;

    if (!eventSource || !notificationUrl) {
      return res.status(400).json({
        error: "Bad Request",
        message: "Missing eventSource or notificationUrl",
      });
    }

    const subscriptionKey = generateId("sub");
    const subscription = {
      subscriptionKey,
      eventSource,
      notificationUrl,
      status: status || "active",
      created: getCurrentTimestamp(),
      version: 1,
    };

    mockData.eventSubscriptions[subscriptionKey] = subscription;

    res.status(201).json(subscription);
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// 10. GET /eventSubscriptions - List event subscriptions
app.get("/eventSubscriptions", authenticateBasic, (req, res) => {
  try {
    const subscriptions = Object.values(mockData.eventSubscriptions);
    res.json({
      subscriptions,
      total: subscriptions.length,
    });
  } catch (error) {
    res.status(500).json({
      error: "Internal Server Error",
      message: error.message,
    });
  }
});

// Error handling middleware
app.use((err, req, res) => {
  console.error("Error:", err);
  res.status(500).json({
    error: "Internal Server Error",
    message: "An unexpected error occurred",
  });
});

// 404 handler
app.use("*", (req, res) => {
  res.status(404).json({
    error: "Not Found",
    message: `Endpoint ${req.method} ${req.originalUrl} not found`,
  });
});

// Start server
app.listen(PORT, () => {
  console.log(`🚀 Silverflow Mock Server running on port ${PORT}`);
  console.log(`📍 Server URL: http://localhost:${PORT}`);
  console.log("\n📋 Available Endpoints:");
  console.log("  GET  /health - Health check");
  console.log("  POST /charges - Create payment authorization");
  console.log("  POST /charges/{chargeKey}/clear - Manually clear charges");
  console.log("  POST /charges/{chargeKey}/refund - Process refunds");
  console.log("  POST /charges/{chargeKey}/reverse - Reverse charge (void)");
  console.log("  GET  /charges/{chargeKey} - Get charge status");
  console.log("  GET  /disputes/{disputeKey} - Get dispute information");
  console.log("  POST /disputes/{disputeKey}/defend - Defend disputes");
  console.log("  POST /processorTokens - Create processor tokens");
  console.log("  POST /eventSubscriptions - Create webhook subscriptions");
  console.log("  GET  /eventSubscriptions - List event subscriptions");
  console.log("\n🔐 Authentication:");
  console.log("  API Key: apk-1wtRxni5IsPsSpBLWpwr");
  console.log("  Secret: FWtnOOHAjbD6rNxWWEeVOCj7JXSEPGJQ");
  console.log("  Alternative - API Key: apk-testkey123, Secret: testsecret456");
  console.log("\n📖 Use Basic Auth with base64 encoded key:secret");
  console.log("  Example: Authorization: Basic <base64(apikey:secret)>");
});

module.exports = app;
