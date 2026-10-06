/* eslint-disable no-console */
import * as express from "express";
import type { NextFunction, Request, Response, Router } from "express";
import cors from "cors";
import { Buffer } from "buffer";

const router: Router = express.default.Router();
const PORT = process.env.PORT || 3011; // Using a different port for Affirm

// Middleware
router.use(cors());
router.use(express.default.json());
router.use(express.default.urlencoded({ extended: true }));

// Mock data storage
interface MockData {
  transactions: Record<string, any>;
  refunds: Record<string, any>;
  checkouts: Record<string, any>;
}

const mockData: MockData = {
  transactions: {},
  refunds: {},
  checkouts: {},
};

// Valid API keys (public_key:private_key)
const validApiKeys: Record<string, string> = {
  public_key_123: "private_key_456",
};

// Helper functions
function generateId(prefix: string): string {
  return `${prefix}_${Math.random().toString(36).substr(2, 16)}`;
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

  if (!authHeader || !authHeader.startsWith("Basic ")) {
    res.status(401).json({
      status_code: 401,
      code: "unauthorized",
      message: "Missing or invalid Authorization header",
      error_type: "authentication_error",
    });
    return;
  }

  const token = authHeader.split(" ")[1];
  const decodedToken = Buffer.from(token, "base64").toString("utf8");
  const [publicKey, privateKey] = decodedToken.split(":");

  if (!publicKey || !privateKey || validApiKeys[publicKey] !== privateKey) {
    res.status(401).json({
      status_code: 401,
      code: "unauthorized",
      message: "Invalid API key",
      error_type: "authentication_error",
    });
    return;
  }

  next();
}

// Logging middleware
router.use((req: Request, res: Response, next: NextFunction): void => {
  console.log(
    `[Affirm Mock] ${new Date().toISOString()} - ${req.method} ${req.path}`
  );
  if (Object.keys(req.headers).length > 0)
    console.log("Headers:", JSON.stringify(req.headers, null, 2));
  if (Object.keys(req.body).length > 0)
    console.log("Body:", JSON.stringify(req.body, null, 2));
  next();
});

// Health check endpoint
router.get("/health", (req: Request, res: Response): void => {
  res.json({
    status: "success",
    msg: "Affirm Mock Server is running",
    timestamp: getCurrentTimestamp(),
  });
});

// 1. POST /v2/checkout/direct - Create a checkout
router.post(
  "/v2/checkout/direct",
  authenticateApiKey,
  (req: Request, res: Response) => {
    const { merchant, total } = req.body;

    if (!merchant || !total) {
      return res.status(400).json({
        status_code: 400,
        code: "bad_request",
        message: "Missing required fields",
        error_type: "invalid_request_error",
      });
    }

    const checkoutId = generateId("checkout");
    const redirectUrl = `${merchant.user_confirmation_url}?checkout_token=${checkoutId}`;

    const checkout = {
      checkout_id: checkoutId,
      redirect_url: redirectUrl,
      ...req.body,
    };
    mockData.checkouts[checkoutId] = checkout;

    res.status(200).json({
      checkout_id: checkoutId,
      redirect_url: redirectUrl,
    });
  }
);

// 2. POST /v1/transactions - Complete authorize (read charge)
router.post(
  "/v1/transactions",
  authenticateApiKey,
  (req: Request, res: Response) => {
    const { transaction_id } = req.body;

    if (!transaction_id) {
      return res.status(400).json({
        status_code: 400,
        code: "bad_request",
        message: "Missing transaction_id",
        error_type: "invalid_request_error",
      });
    }

    const checkout = mockData.checkouts[transaction_id];
    if (!checkout) {
      return res.status(404).json({
        status_code: 404,
        code: "not_found",
        message: "Checkout not found",
        error_type: "invalid_request_error",
      });
    }

    const transactionId = generateId("txn");
    const now = getCurrentTimestamp();
    const transaction = {
      id: transactionId,
      status: "authorized",
      amount: checkout.total,
      currency: checkout.currency,
      created: now,
      order_id: checkout.order_id,
      checkout_id: transaction_id,
      events: [
        {
          id: generateId("evt"),
          type: "auth",
          created: now,
        },
      ],
    };
    mockData.transactions[transactionId] = transaction;

    res.status(200).json(transaction);
  }
);

// 3. GET /v1/transactions/:transactionId - PSync
router.get(
  "/v1/transactions/:transactionId",
  authenticateApiKey,
  (req: Request, res: Response) => {
    const { transactionId } = req.params;
    const transaction = mockData.transactions[transactionId];

    if (!transaction) {
      return res.status(404).json({
        status_code: 404,
        code: "not_found",
        message: "Transaction not found",
        error_type: "invalid_request_error",
      });
    }

    res.status(200).json(transaction);
  }
);

// 4. POST /v1/transactions/:transactionId/capture - Capture
router.post(
  "/v1/transactions/:transactionId/capture",
  authenticateApiKey,
  (req: Request, res: Response) => {
    const { transactionId } = req.params;
    const transaction = mockData.transactions[transactionId];

    if (!transaction) {
      return res.status(404).json({
        status_code: 404,
        code: "not_found",
        message: "Transaction not found",
        error_type: "invalid_request_error",
      });
    }

    if (transaction.status !== "authorized") {
      return res.status(400).json({
        status_code: 400,
        code: "bad_request",
        message: "Transaction is not authorized",
        error_type: "invalid_request_error",
      });
    }

    transaction.status = "captured";
    const event = {
      id: generateId("evt"),
      type: "capture",
      created: getCurrentTimestamp(),
    };
    transaction.events.push(event);

    res.status(200).json(event);
  }
);

// 5. POST /v1/transactions/:transactionId/void - Void
router.post(
  "/v1/transactions/:transactionId/void",
  authenticateApiKey,
  (req: Request, res: Response) => {
    const { transactionId } = req.params;
    const transaction = mockData.transactions[transactionId];

    if (!transaction) {
      return res.status(404).json({
        status_code: 404,
        code: "not_found",
        message: "Transaction not found",
        error_type: "invalid_request_error",
      });
    }

    if (transaction.status === "captured") {
      return res.status(400).json({
        status_code: 400,
        code: "bad_request",
        message: "Cannot void a captured transaction",
        error_type: "invalid_request_error",
      });
    }

    transaction.status = "voided";
    const event = {
      id: generateId("evt"),
      type: "void",
      created: getCurrentTimestamp(),
    };
    transaction.events.push(event);

    res.status(200).json(event);
  }
);

// 6. POST /v1/transactions/:transactionId/refund - Refund
router.post(
  "/v1/transactions/:transactionId/refund",
  authenticateApiKey,
  (req: Request, res: Response) => {
    const { transactionId } = req.params;
    const { amount } = req.body;
    const transaction = mockData.transactions[transactionId];

    if (!transaction) {
      return res.status(404).json({
        status_code: 404,
        code: "not_found",
        message: "Transaction not found",
        error_type: "invalid_request_error",
      });
    }

    if (transaction.status !== "captured") {
      return res.status(400).json({
        status_code: 400,
        code: "bad_request",
        message: "Cannot refund a non-captured transaction",
        error_type: "invalid_request_error",
      });
    }

    if (amount > transaction.amount) {
      return res.status(400).json({
        status_code: 400,
        code: "bad_request",
        message: "Refund amount exceeds transaction amount",
        error_type: "invalid_request_error",
      });
    }

    const refundId = generateId("ref");
    const now = getCurrentTimestamp();
    const refund = {
      id: refundId,
      amount,
      created: now,
      currency: transaction.currency,
      transaction_id: transactionId,
      type: "refund",
    };
    mockData.refunds[refundId] = refund;

    transaction.status = "refunded";
    transaction.events.push(refund);

    res.status(200).json(refund);
  }
);

// Error handling middleware
router.use(
  (err: Error, req: Request, res: Response, _next: NextFunction): void => {
    console.error("[Affirm Mock] Error:", err);
    res.status(500).json({
      status_code: 500,
      code: "internal_server_error",
      message: "An unexpected error occurred",
      error_type: "api_error",
    });
  }
);

// 404 handler
router.use((req: Request, res: Response): void => {
  res.status(404).json({
    status_code: 404,
    code: "not_found",
    message: `Endpoint ${req.method} ${req.originalUrl} not found`,
    error_type: "invalid_request_error",
  });
});

console.log(`🚀 Affirm Mock Server running on port ${PORT}`);
console.log(`📍 Server URL: http://localhost:${PORT}`);

export default router;
