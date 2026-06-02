/* eslint-disable no-console */
import * as express from "express";
import type { NextFunction, Request, Response, Router } from "express";
import cors from "cors";

const router: Router = express.default.Router();
const PORT = process.env.PORT || 3010;

// Middleware
router.use(cors());
router.use(express.default.json());
router.use(express.default.urlencoded({ extended: true }));

// Mock data storage
interface MockData {
  transactions: Record<string, any>;
  refunds: Record<string, any>;
}

const mockData: MockData = {
  transactions: {},
  refunds: {},
};

// Valid API keys
const validApiKeys: string[] = [
  "celero-test-api-key-123",
  "celero-api-key-456",
  "api-celero-test",
  "alpha-test-api-key-123",
];

// Helper functions
function generateId(prefix: string): string {
  return `${prefix}_${Math.random().toString(36).substr(2, 10)}`;
}

function generateAuthCode(): string {
  return Math.floor(100000 + Math.random() * 900000).toString();
}

function getCurrentTimestamp(): string {
  return new Date().toISOString();
}

// Authentication middleware
function authenticateApiKey(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const authHeader = req.headers.authorization;

  if (!authHeader) {
    res.status(401).json({
      status: "error",
      msg: "Missing Authorization header",
    });
    return;
  }

  // Check if the API key is valid
  const apiKey = authHeader;

  if (!validApiKeys.includes(apiKey)) {
    res.status(401).json({
      status: "error",
      msg: "Invalid API key",
    });
    return;
  }

  (req as any).apiKey = apiKey;
  next();
}

// Logging middleware
router.use((req: Request, res: Response, next: NextFunction): void => {
  console.log(`${new Date().toISOString()} - ${req.method} ${req.path}`);
  console.log("Headers:", JSON.stringify(req.headers, null, 2));
  if (req.body && Object.keys(req.body).length > 0) {
    console.log("Body:", JSON.stringify(req.body, null, 2));
  }
  next();
});

// Health check endpoint
router.get("/health", (req: Request, res: Response): void => {
  res.json({
    status: "success",
    msg: "Celero Mock Server is running",
    timestamp: getCurrentTimestamp(),
  });
});

// 1. POST /api/transaction - Create payment (authorize or sale)
router.post(
  "/api/transaction",
  authenticateApiKey,
  (req: Request, res: Response): void => {
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
        res.status(400).json({
          status: "error",
          msg: "Missing required fields: idempotency_key, type, amount, currency, payment_method",
        });
        return;
      }

      // Check if payment method is card
      if (!payment_method.card) {
        res.status(400).json({
          status: "error",
          msg: "Only card payment method is supported",
        });
        return;
      }

      // Check for fail card - reject payment if fail card is used
      const failCard = "4000000000000002";
      if (payment_method.card.number === failCard) {
        res.status(200).json({
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
        return;
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
    } catch (error: any) {
      console.error("Error processing transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// 2. POST /api/transaction/search - Search for transactions (used for payment sync and refund sync)
router.post(
  "/api/transaction/search",
  authenticateApiKey,
  (req: Request, res: Response): void => {
    try {
      const { transaction_id } = req.body;

      if (!transaction_id) {
        res.status(400).json({
          status: "error",
          msg: "Missing required field: transaction_id",
        });
        return;
      }

      // Check if it's a transaction ID or refund ID
      if (transaction_id.startsWith("txn_")) {
        // It's a transaction ID
        const transaction = mockData.transactions[transaction_id];
        if (!transaction) {
          res.status(404).json({
            status: "error",
            msg: "Transaction not found",
          });
          return;
        }

        res.status(200).json({
          status: "success",
          msg: "Transaction found",
          data: transaction,
        });
      } else if (transaction_id.startsWith("ref_")) {
        // It's a refund ID
        const refund = mockData.refunds[transaction_id];
        if (!refund) {
          res.status(404).json({
            status: "error",
            msg: "Refund not found",
          });
          return;
        }

        res.status(200).json({
          status: "success",
          msg: "Refund found",
          data: refund,
        });
      } else {
        res.status(404).json({
          status: "error",
          msg: "Invalid transaction_id format",
        });
      }
    } catch (error: any) {
      console.error("Error searching transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// 3. POST /api/transaction/:transactionId/capture - Capture an authorized payment
router.post(
  "/api/transaction/:transactionId/capture",
  authenticateApiKey,
  (req: Request, res: Response): void => {
    try {
      const { transactionId } = req.params;
      const { amount, order_id } = req.body;

      // Validate transaction exists
      const transaction = mockData.transactions[transactionId];
      if (!transaction) {
        res.status(404).json({
          status: "error",
          msg: "Transaction not found",
        });
        return;
      }

      // Validate transaction is authorized
      if (
        transaction.type !== "authorize" ||
        transaction.response.card.status !== "approved"
      ) {
        res.status(400).json({
          status: "error",
          msg: "Transaction cannot be captured (not an approved authorization)",
        });
        return;
      }

      // Validate amount
      if (!amount || amount <= 0) {
        res.status(400).json({
          status: "error",
          msg: "Invalid amount for capture",
        });
        return;
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
    } catch (error: any) {
      console.error("Error capturing transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// 4. POST /api/transaction/:transactionId/void - Void a transaction
router.post(
  "/api/transaction/:transactionId/void",
  authenticateApiKey,
  (req: Request, res: Response): void => {
    try {
      const { transactionId } = req.params;

      // Validate transaction exists
      const transaction = mockData.transactions[transactionId];
      if (!transaction) {
        res.status(404).json({
          status: "error",
          msg: "Transaction not found",
        });
        return;
      }

      // Validate transaction can be voided (not settled)
      if (transaction.response.card.status === "settled") {
        res.status(400).json({
          status: "error",
          msg: "Transaction cannot be voided (already settled)",
        });
        return;
      }

      // Update transaction status
      transaction.response.card.status = "voided";
      transaction.updated_at = getCurrentTimestamp();

      res.status(200).json({
        status: "success",
        msg: "Transaction voided successfully",
      });
    } catch (error: any) {
      console.error("Error voiding transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// 5. POST /api/transaction/:transactionId/refund - Refund a transaction
router.post(
  "/api/transaction/:transactionId/refund",
  authenticateApiKey,
  (req: Request, res: Response): void => {
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
        res.status(400).json({
          status: "error",
          msg: "Invalid amount for refund",
        });
        return;
      }

      // Validate amount doesn't exceed original transaction
      if (amount > transaction.amount) {
        res.status(400).json({
          status: "error",
          msg: "Refund amount exceeds original transaction amount",
        });
        return;
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
    } catch (error: any) {
      console.error("Error refunding transaction:", error);
      res.status(500).json({
        status: "error",
        msg: `Internal server error: ${error.message}`,
      });
    }
  }
);

// Error handling middleware

router.use(
  (err: Error, req: Request, res: Response, _next: NextFunction): void => {
    console.error("Error:", err);
    res.status(500).json({
      status: "error",
      msg: "An unexpected error occurred",
    });
  }
);

// 404 handler
router.use((req: Request, res: Response): void => {
  res.status(404).json({
    status: "error",
    msg: `Endpoint ${req.method} ${req.originalUrl} not found`,
  });
});

// Create Express app and use the router

console.log(`🚀 Celero Mock Server running on port ${PORT}`);
console.log(`📍 Server URL: http://localhost:${PORT}`);
console.log("\n📋 Available Endpoints:");
console.log("  GET  /health - Health check");
console.log(
  "  POST celero/api/transaction - Create payment (authorize or sale)"
);
console.log("  POST celero/api/transaction/search - Search for transactions");
console.log(
  "  POST /api/transaction/{id}/capture - Capture an authorized payment"
);
console.log("  POST celero/api/transaction/{id}/void - Void a transaction");
console.log("  POST celero/api/transaction/{id}/refund - Refund a transaction");
console.log("\n🔐 Authentication:");
console.log("  API Key in Authorization header");
console.log("  Valid API Keys:");
console.log("    - celero-test-api-key-123");
console.log("    - celero-api-key-456");
console.log("    - api-celero-test");
console.log("    - alpha-test-api-key-123");
console.log(
  "\n💡 Note: Celero mock server is designed to work with the Hyperswitch Celero connector"
);

export default router;
