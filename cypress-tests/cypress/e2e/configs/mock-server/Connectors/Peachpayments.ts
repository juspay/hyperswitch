/* eslint-disable no-console */
import * as express from "express";
import type { NextFunction, Request, Response, Router } from "express";

const router: Router = express.default.Router();

// Mock data storage
interface MockData {
  payments: Record<string, any>;
}

const mockData: MockData = {
  payments: {},
};

// Mock API credentials
const validCredentials: Record<string, { username: string; password: string }> =
  {
    "peach-test-entity-id": {
      username: "peach-test-username",
      password: "peach-test-password",
    },
  };

// Helper functions
function generateId(prefix: string): string {
  return `${prefix}${Math.random().toString(36).substr(2, 16)}`;
}

function getCurrentTimestamp(): string {
  return new Date().toISOString();
}

// Parse form-encoded body
router.use(express.default.urlencoded({ extended: true }));

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
  res.json({ status: "OK", timestamp: getCurrentTimestamp() });
});

// APM Payments endpoint - POST /v1/payments
router.post("/v1/payments", (req: Request, res: Response): void => {
  try {
    const {
      "authentication.entityId": entityId,
      "authentication.userId": username,
      "authentication.password": password,
      amount,
      currency,
      paymentType,
      paymentBrand,
      merchantTransactionId,
      shopperResultUrl,
    } = req.body;

    // Validate authentication
    if (!entityId || !username || !password) {
      res.status(401).json({
        result: {
          code: "100.150.100",
          description: "Missing authentication credentials",
        },
      });
      return;
    }

    const validCreds = validCredentials[entityId];
    if (
      !validCreds ||
      validCreds.username !== username ||
      validCreds.password !== password
    ) {
      res.status(401).json({
        result: {
          code: "100.150.101",
          description: "Invalid authentication credentials",
        },
      });
      return;
    }

    // Validate required fields
    if (!amount || !currency || !paymentType || !paymentBrand) {
      res.status(400).json({
        result: {
          code: "100.100.100",
          description: "Missing required fields",
        },
      });
      return;
    }

    // Generate payment ID
    const paymentId = generateId("8ac7a4a1");
    const now = getCurrentTimestamp();

    // APM payments always return a redirect for customer action
    const payment = {
      id: paymentId,
      paymentType,
      paymentBrand,
      amount,
      currency,
      merchantTransactionId,
      result: {
        code: "000.200.000",
        description:
          "Transaction pending - redirect shopper to complete payment",
      },
      redirect: {
        url: `https://test.peachpayments.com/checkout/${paymentId}`,
        method: "GET",
        parameters: [],
      },
      timestamp: now,
      ndc: generateId("8a829418"),
    };

    mockData.payments[paymentId] = {
      ...payment,
      shopperResultUrl,
      status: "pending",
    };

    res.status(200).json(payment);
  } catch (error: any) {
    res.status(500).json({
      result: {
        code: "900.100.100",
        description: `Internal server error: ${error.message}`,
      },
    });
  }
});

// Payment status query - GET /v1/query/:paymentId
router.get("/v1/query/:paymentId", (req: Request, res: Response): void => {
  try {
    const { paymentId } = req.params;
    const entityId = req.query.entityId as string;

    if (!entityId) {
      res.status(400).json({
        result: {
          code: "100.100.100",
          description: "Missing entityId query parameter",
        },
      });
      return;
    }

    const payment = mockData.payments[paymentId];
    if (!payment) {
      res.status(404).json({
        result: {
          code: "700.400.200",
          description: "Transaction not found",
        },
      });
      return;
    }

    // Simulate payment completion after query (for testing)
    // In real scenario, webhook would notify completion
    const completedPayment = {
      id: payment.id,
      paymentType: payment.paymentType,
      paymentBrand: payment.paymentBrand,
      amount: payment.amount,
      currency: payment.currency,
      merchantTransactionId: payment.merchantTransactionId,
      result: {
        code: "000.000.000",
        description:
          "Transaction succeeded - Request successfully processed in 'Merchant in Integrator Test Mode'",
      },
      timestamp: getCurrentTimestamp(),
      ndc: payment.ndc,
    };

    // Update stored payment status
    mockData.payments[paymentId].status = "completed";

    res.status(200).json(completedPayment);
  } catch (error: any) {
    res.status(500).json({
      result: {
        code: "900.100.100",
        description: `Internal server error: ${error.message}`,
      },
    });
  }
});

// Refund endpoint - POST /v1/payments/:paymentId
router.post("/v1/payments/:paymentId", (req: Request, res: Response): void => {
  try {
    const { paymentId } = req.params;
    const {
      "authentication.entityId": entityId,
      "authentication.userId": username,
      "authentication.password": password,
      amount,
      currency,
      paymentType,
    } = req.body;

    // Validate authentication
    if (!entityId || !username || !password) {
      res.status(401).json({
        result: {
          code: "100.150.100",
          description: "Missing authentication credentials",
        },
      });
      return;
    }

    const validCreds = validCredentials[entityId];
    if (
      !validCreds ||
      validCreds.username !== username ||
      validCreds.password !== password
    ) {
      res.status(401).json({
        result: {
          code: "100.150.101",
          description: "Invalid authentication credentials",
        },
      });
      return;
    }

    // Check if original payment exists
    const originalPayment = mockData.payments[paymentId];
    if (!originalPayment) {
      res.status(404).json({
        result: {
          code: "700.400.200",
          description: "Transaction not found",
        },
      });
      return;
    }

    // Validate this is a refund request
    if (paymentType !== "RF") {
      res.status(400).json({
        result: {
          code: "100.100.101",
          description: "Invalid paymentType for refund",
        },
      });
      return;
    }

    const refundId = generateId("8ac7a4a1");
    const now = getCurrentTimestamp();

    const refund = {
      id: refundId,
      referencedId: paymentId,
      paymentType: "RF",
      amount,
      currency,
      result: {
        code: "000.000.000",
        description: "Transaction succeeded",
      },
      timestamp: now,
      ndc: generateId("8a829418"),
    };

    res.status(200).json(refund);
  } catch (error: any) {
    res.status(500).json({
      result: {
        code: "900.100.100",
        description: `Internal server error: ${error.message}`,
      },
    });
  }
});

// Error handling middleware
router.use(
  (err: Error, req: Request, res: Response, _next: NextFunction): void => {
    console.error("Error:", err);
    res.status(500).json({
      result: {
        code: "900.100.100",
        description: "An unexpected error occurred",
      },
    });
  }
);

// 404 handler
router.use((req: Request, res: Response): void => {
  res.status(404).json({
    result: {
      code: "700.400.000",
      description: `Endpoint ${req.method} ${req.originalUrl} not found`,
    },
  });
});

// Log available endpoints for debugging purposes
console.log("\n📋 Peachpayments APM Mock API Endpoints:");
console.log("  GET  /health - Health check");
console.log("  POST /v1/payments - Create APM payment");
console.log("  GET  /v1/query/{paymentId} - Query payment status");
console.log("  POST /v1/payments/{paymentId} - Refund payment");
console.log("\n🔐 Authentication (form-encoded body):");
console.log("  authentication.entityId: peach-test-entity-id");
console.log("  authentication.userId: peach-test-username");
console.log("  authentication.password: peach-test-password");

export default router;
