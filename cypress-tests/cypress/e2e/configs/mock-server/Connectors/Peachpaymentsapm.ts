/* eslint-disable no-console */
import * as express from "express";
import type { Request, Response, Router } from "express";

// Create router for Peachpaymentsapm mock
const router: Router = express.default.Router();

// Log all requests
router.use((req: Request, _res: Response, next) => {
  console.log(`[Peachpaymentsapm] ${req.method} ${req.path}`);
  next();
});

// Health check
router.get("/health", (_req: Request, res: Response) => {
  res.json({
    status: "OK",
    connector: "peachpaymentsapm",
    timestamp: new Date().toISOString(),
  });
});

// Payment endpoint - simulates PeachPayments Payments API
router.post("/v1/payments", (req: Request, res: Response) => {
  const transactionId = `8ac7a4c8${Date.now().toString(16)}`;

  // Return pending response with redirect (typical for EFT payments)
  res.status(200).json({
    id: transactionId,
    paymentType: "DB",
    paymentBrand: "PAYSHAP",
    amount: "100.00",
    currency: "ZAR",
    descriptor: "Test payment",
    merchantTransactionId: req.body?.merchantTransactionId || "test_ref",
    result: {
      code: "000.200.000",
      description: "Transaction pending",
    },
    redirect: {
      url: `https://test.peachpayments.com/redirect/${transactionId}`,
      method: "GET",
    },
    timestamp: new Date().toISOString(),
    ndc: `${transactionId}_mock`,
  });
});

// Query endpoint - simulates payment status check
router.get("/v1/query/:transactionId", (req: Request, res: Response) => {
  const { transactionId } = req.params;

  // Simulate completed payment for valid transaction IDs
  if (transactionId && transactionId.startsWith("8ac7a4c8")) {
    res.status(200).json({
      id: transactionId,
      paymentType: "DB",
      paymentBrand: "PAYSHAP",
      amount: "100.00",
      currency: "ZAR",
      result: {
        code: "000.000.000",
        description: "Transaction succeeded",
      },
      timestamp: new Date().toISOString(),
      ndc: `${transactionId}_mock`,
    });
  } else {
    // Return error for invalid transaction IDs
    res.status(200).json({
      result: {
        code: "700.400.300",
        description: "Cannot find transaction",
      },
      timestamp: new Date().toISOString(),
      ndc: "mock_error",
    });
  }
});

// Refund endpoint
router.post("/v1/payments/:transactionId", (req: Request, res: Response) => {
  const { transactionId } = req.params;
  const refundId = `8ac7a4c8${Date.now().toString(16)}`;

  res.status(200).json({
    id: refundId,
    referencedId: transactionId,
    paymentType: "RF",
    amount: req.body?.amount || "100.00",
    currency: "ZAR",
    result: {
      code: "000.000.000",
      description: "Transaction succeeded",
    },
    timestamp: new Date().toISOString(),
    ndc: `${refundId}_mock`,
  });
});

export default router;
