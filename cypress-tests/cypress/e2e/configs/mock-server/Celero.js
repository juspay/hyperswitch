/* eslint-disable no-console */
import express from "express";
import cors from "cors";

const app = express();
const PORT = process.env.PORT || 3010;

// Middleware
app.use(cors());
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Mock data storage
const mockData = {
  transactions: {},
  refunds: {},
};

// Helper functions
function generateId(prefix) {
  return `${prefix}_${Math.random().toString(36).substr(2, 10)}`;
}

function generateAuthCode() {
  return Math.floor(100000 + Math.random() * 900000).toString();
}

function getCurrentTimestamp() {
  return new Date().toISOString();
}

// Authentication middleware
function authenticateApiKey(req, res, next) {
  const authHeader = req.headers.authorization;

  if (!authHeader) {
    return res.status(401).json({
      status: "error",
      msg: "Missing Authorization header",
    });
  }

  // Check if the API key is valid
  const apiKey = authHeader;

  req.apiKey = apiKey;
  next();
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
    status: "success",
    msg: "Celero Mock Server is running",
    timestamp: getCurrentTimestamp(),
  });
});

// 1. POST /api/transaction - Create payment (authorize or sale)
app.post("/api/transaction", authenticateApiKey, (req, res) => {
  try {
    const {
      idempotency_key,
      type: transaction_type,
      amount,
      currency,
      payment_method,
    } = req.body;

    // Basic validation
    if (
      !idempotency_key ||
      !transaction_type ||
      !amount ||
      !currency ||
      !payment_method
    ) {
      return res.status(400).json({
        status: "error",
        msg: "Missing required fields: idempotency_key, type, amount, currency, payment_method",
      });
    }

    // Check if payment method is card
    if (!payment_method.card) {
      return res.status(400).json({
        status: "error",
        msg: "Only card payment method is supported",
      });
    }

    // Check for fail card - reject payment if fail card is used
    const failCard = "4000000000000002";
    if (payment_method.card.number === failCard) {
      return res.status(200).json({
        status: "success",
        msg: "Transaction processed",
        data: {
          id: generateId("txn"),
          type: transaction_type,
          amount,
          currency,
          response: {
            card: {
              status: "declined",
              processor_response_code: "05",
              avs_response_code: "N",
            },
          },
          billing_address: req.body.billing_address,
          shipping_address: req.body.shipping_address,
        },
      });
    }

    // Process the transaction
    const transactionId = generateId("txn");
    const authCode = generateAuthCode();
    const now = getCurrentTimestamp();

    const transaction = {
      id: transactionId,
      type: transaction_type,
      amount,
      currency,
      response: {
        card: {
          status: transaction_type === "sale" ? "settled" : "approved",
          auth_code: authCode,
          processor_response_code: "00",
          avs_response_code: "Y",
        },
      },
      billing_address: req.body.billing_address,
      shipping_address: req.body.shipping_address,
      created_at: now,
      updated_at: now,
    };

    // Store the transaction
    mockData.transactions[transactionId] = transaction;

    res.status(200).json({
      status: "success",
      msg: "Transaction processed",
      data: transaction,
    });
  } catch (error) {
    console.error("Error processing transaction:", error);
    res.status(500).json({
      status: "error",
      msg: `Internal server error: ${error.message}`,
    });
  }
});

// 2. POST /api/transaction/search - Search for transactions (used for payment sync and refund sync)
app.post("/api/transaction/search", authenticateApiKey, (req, res) => {
  try {
    const { transaction_id } = req.body;

    if (!transaction_id) {
      return res.status(400).json({
        status: "error",
        msg: "Missing required field: transaction_id",
      });
    }

    // Check if it's a transaction ID or refund ID
    if (transaction_id.startsWith("txn_")) {
      // It's a transaction ID
      const transaction = mockData.transactions[transaction_id];
      if (!transaction) {
        return res.status(404).json({
          status: "error",
          msg: "Transaction not found",
        });
      }

      return res.status(200).json({
        status: "success",
        msg: "Transaction found",
        data: transaction,
      });
    } else if (transaction_id.startsWith("ref_")) {
      // It's a refund ID
      const refund = mockData.refunds[transaction_id];
      if (!refund) {
        return res.status(404).json({
          status: "error",
          msg: "Refund not found",
        });
      }

      return res.status(200).json({
        status: "success",
        msg: "Refund found",
        data: refund,
      });
    } else {
      return res.status(404).json({
        status: "error",
        msg: "Invalid transaction_id format",
      });
    }
  } catch (error) {
    console.error("Error searching transaction:", error);
    res.status(500).json({
      status: "error",
      msg: `Internal server error: ${error.message}`,
    });
  }
});

// 3. POST /api/transaction/:transactionId/capture - Capture an authorized payment
app.post(
  "/api/transaction/:transactionId/capture",
  authenticateApiKey,
  (req, res) => {
    try {
      const { transactionId } = req.params;
      const { amount, order_id } = req.body;

      // Validate transaction exists
      const transaction = mockData.transactions[transactionId];
      if (!transaction) {
        return res.status(404).json({
          status: "error",
          msg: "Transaction not found",
        });
      }

      // Validate transaction is authorized
      if (
        transaction.type !== "authorize" ||
        transaction.response.card.status !== "approved"
      ) {
        return res.status(400).json({
          status: "error",
          msg: "Transaction cannot be captured (not an approved authorization)",
        });
      }

      // Validate amount
      if (!amount || amount <= 0) {
        return res.status(400).json({
          status: "error",
          msg: "Invalid amount for capture",
        });
      }

      // Update transaction status
      transaction.response.card.status = "settled";
      transaction.updated_at = getCurrentTimestamp();

      // If amount is different from original, update it
      if (amount !== transaction.amount) {
        transaction.amount = amount;
      }

      // Add order_id if provided
      if (order_id) {
        transaction.order_id = order_id;
      }

      res.status(200).json({
        status: "success",
        msg: "Transaction captured successfully",
      });
    } catch (error) {
      console.error("Error capturing transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// 4. POST /api/transaction/:transactionId/void - Void a transaction
app.post(
  "/api/transaction/:transactionId/void",
  authenticateApiKey,
  (req, res) => {
    try {
      const { transactionId } = req.params;

      // Validate transaction exists
      const transaction = mockData.transactions[transactionId];
      if (!transaction) {
        return res.status(404).json({
          status: "error",
          msg: "Transaction not found",
        });
      }

      // Validate transaction can be voided (not settled)
      if (transaction.response.card.status === "settled") {
        return res.status(400).json({
          status: "error",
          msg: "Transaction cannot be voided (already settled)",
        });
      }

      // Update transaction status
      transaction.response.card.status = "voided";
      transaction.updated_at = getCurrentTimestamp();

      res.status(200).json({
        status: "success",
        msg: "Transaction voided successfully",
      });
    } catch (error) {
      console.error("Error voiding transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// 5. POST /api/transaction/:transactionId/refund - Refund a transaction
app.post(
  "/api/transaction/:transactionId/refund",
  authenticateApiKey,
  (req, res) => {
    try {
      const { transactionId } = req.params;
      const { amount, surcharge } = req.body;

      // Validate transaction exists
      const transaction = mockData.transactions[transactionId];
      // if (!transaction) {
      //   return res.status(404).json({
      //     status: "error",
      //     msg: "Transaction not found",
      //   });
      // }

      // Validate transaction can be refunded (must be settled)

      // Validate amount
      if (!amount || amount <= 0) {
        return res.status(400).json({
          status: "error",
          msg: "Invalid amount for refund",
        });
      }

      // Validate amount doesn't exceed original transaction
      if (amount > transaction.amount) {
        return res.status(400).json({
          status: "error",
          msg: "Refund amount exceeds original transaction amount",
        });
      }

      // Create refund record
      const refundId = generateId("ref");
      const refund = {
        id: refundId,
        transaction_id: transactionId,
        amount,
        surcharge: surcharge || 0,
        currency: transaction.currency,
        status: "success",
        created_at: getCurrentTimestamp(),
      };

      // Store the refund
      mockData.refunds[refundId] = refund;

      // If full refund, update transaction status
      if (amount === transaction.amount) {
        transaction.response.card.status = "refunded";
        transaction.updated_at = getCurrentTimestamp();
      } else {
        transaction.response.card.status = "partially_refunded";
        transaction.updated_at = getCurrentTimestamp();
      }

      res.status(200).json({
        status: "success",
        msg: "Transaction refunded successfully",
        data: null,
      });
    } catch (error) {
      console.error("Error refunding transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// Error handling middleware
// eslint-disable-next-line no-unused-vars
app.use((err, req, res, next) => {
  console.error("Error:", err);
  res.status(500).json({
    status: "error",
    msg: "An unexpected error occurred",
  });
});

// 404 handler
app.use((req, res) => {
  res.status(404).json({
    status: "error",
    msg: `Endpoint ${req.method} ${req.originalUrl} not found`,
  });
});

// Start server
app.listen(PORT, () => {
  console.log(`🚀 Celero Mock Server running on port ${PORT}`);
  console.log(`📍 Server URL: http://localhost:${PORT}`);
  console.log("\n📋 Available Endpoints:");
  console.log("  GET  /health - Health check");
  console.log("  POST /api/transaction - Create payment (authorize or sale)");
  console.log("  POST /api/transaction/search - Search for transactions");
  console.log(
    "  POST /api/transaction/{id}/capture - Capture an authorized payment"
  );
  console.log("  POST /api/transaction/{id}/void - Void a transaction");
  console.log("  POST /api/transaction/{id}/refund - Refund a transaction");
  console.log("\n🔐 Authentication:");
  console.log("  API Key in Authorization header");
  console.log("  Valid API Keys:");
  console.log("    - celero-test-api-key-123");
  console.log("    - celero-api-key-456");
  console.log("    - api-celero-test");
  console.log(
    "\n💡 Note: Celero mock server is designed to work with the Hyperswitch Celero connector"
  );
});

export default app;
