/* eslint-disable no-console */
import * as express from "express";
import type { NextFunction, Request, Response, Router } from "express";

// Mock of the Peach Payments "Payments API" (api-v2.peachpayments.com) used
// for APMs. Point `connectors.peachpayments.secondary_base_url` at
// `http://localhost:3010/peachpayments` to run the connector against it.
//
// Contract:
//   POST /payments        - create payment (JSON body, nested `authentication`)
//   GET  /payments/:id    - payment status (auth via `authentication.*` query params)
//   POST /payments/:id    - refund (JSON body, paymentType "RF")

const router: Router = express.default.Router();

interface MockData {
  payments: Record<string, any>;
}

const mockData: MockData = {
  payments: {},
};

const VALID_CREDENTIALS: Record<string, { userId: string; password: string }> =
  {
    // 32-char hex ids, matching the live API format
    abcdef0123456789abcdef0123456789: {
      userId: "11112222333344445555666677778888",
      password: "peach-test-password",
    },
  };

const PAYMENT_BRANDS: string[] = [
  "CAPITECPAY",
  "PAYSHAP",
  "NEDBANKDIRECTEFT",
  "PEACHEFT",
  "PAYFLEX",
  "ZEROPAY",
  "FLOAT",
  "HAPPYPAY",
  "MOBICRED",
  "RCS",
  "APLUS",
  "MPESA",
  "BLINKBYEMTEL",
  "MCBJUICE",
  "MASTERPASS",
  "MAUCAS",
  "1FORYOU",
  "MONEYBADGER",
];

// 1FORYOU is the only synchronous (non-redirect) brand
const SYNCHRONOUS_BRANDS: string[] = ["1FORYOU"];

const MERCHANT_TRANSACTION_ID_REGEX = /^[a-zA-Z0-9]{8,16}$/;

function generateHexId(): string {
  let id = "";
  for (let i = 0; i < 32; i += 1) {
    id += Math.floor(Math.random() * 16).toString(16);
  }
  return id;
}

function getCurrentTimestamp(): string {
  return new Date().toISOString();
}

function validationError(res: Response, parameterErrors: any[]): void {
  res.status(400).json({
    result: {
      code: "800.400.100",
      description: "Request parameter validation failed",
      parameterErrors,
    },
  });
}

function authenticationError(res: Response, description: string): void {
  res.status(401).json({
    result: {
      code: "800.900.300",
      description,
    },
  });
}

function isValidAuthentication(authentication: any): boolean {
  if (
    !authentication ||
    !authentication.entityId ||
    !authentication.userId ||
    !authentication.password
  ) {
    return false;
  }
  const credentials = VALID_CREDENTIALS[authentication.entityId];
  return (
    credentials !== undefined &&
    credentials.userId === authentication.userId &&
    credentials.password === authentication.password
  );
}

router.use(express.default.json());

router.use((req: Request, res: Response, next: NextFunction): void => {
  console.log(`${getCurrentTimestamp()} - ${req.method} ${req.path}`);
  if (req.body && Object.keys(req.body).length > 0) {
    console.log("Body:", JSON.stringify(req.body, null, 2));
  }
  next();
});

router.get("/health", (req: Request, res: Response): void => {
  res.json({ status: "OK", timestamp: getCurrentTimestamp() });
});

// Create payment (paymentType "DB")
router.post("/payments", (req: Request, res: Response): void => {
  try {
    const {
      authentication,
      amount,
      currency,
      paymentType,
      paymentBrand,
      merchantTransactionId,
      shopperResultUrl,
    } = req.body;

    if (!isValidAuthentication(authentication)) {
      authenticationError(res, "Invalid authentication information");
      return;
    }

    const parameterErrors: any[] = [];
    if (!amount || !/^\d{1,8}(\.\d{2})?$/.test(String(amount))) {
      parameterErrors.push({
        name: "amount",
        value: amount,
        message: "amount must be a decimal string with two decimal places",
      });
    }
    if (!currency) {
      parameterErrors.push({
        name: "currency",
        value: currency,
        message: "currency is required",
      });
    }
    if (paymentType !== "DB") {
      parameterErrors.push({
        name: "paymentType",
        value: paymentType,
        message: "paymentType must be DB",
      });
    }
    if (!paymentBrand || !PAYMENT_BRANDS.includes(paymentBrand)) {
      parameterErrors.push({
        name: "paymentBrand",
        value: paymentBrand,
        message: "paymentBrand is not supported",
      });
    }
    if (
      !merchantTransactionId ||
      !MERCHANT_TRANSACTION_ID_REGEX.test(merchantTransactionId)
    ) {
      parameterErrors.push({
        name: "merchantTransactionId",
        value: merchantTransactionId,
        message: "merchantTransactionId must be 8-16 alphanumeric characters",
      });
    }
    if (parameterErrors.length > 0) {
      validationError(res, parameterErrors);
      return;
    }

    const paymentId = generateHexId();
    const now = getCurrentTimestamp();
    const isSynchronous = SYNCHRONOUS_BRANDS.includes(paymentBrand);

    const payment: Record<string, any> = {
      id: paymentId,
      paymentType,
      paymentBrand,
      amount,
      currency,
      merchantTransactionId,
      timestamp: now,
      result: isSynchronous
        ? {
            code: "000.000.000",
            description: "Transaction succeeded",
          }
        : {
            code: "000.200.000",
            description:
              "Transaction pending - redirect shopper to complete payment",
          },
    };

    if (!isSynchronous) {
      payment.redirect = {
        url: `https://testsecure.peachpayments.com/redirect/${paymentId}`,
        method: "GET",
        parameters: [],
      };
    }

    mockData.payments[paymentId] = {
      ...payment,
      shopperResultUrl,
      status: isSynchronous ? "succeeded" : "pending",
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

// Payment status (authentication via query parameters)
router.get("/payments/:paymentId", (req: Request, res: Response): void => {
  try {
    const { paymentId } = req.params;
    const authentication = {
      entityId: req.query["authentication.entityId"] as string,
      userId: req.query["authentication.userId"] as string,
      password: req.query["authentication.password"] as string,
    };

    if (!isValidAuthentication(authentication)) {
      authenticationError(res, "Invalid authentication information");
      return;
    }

    const payment = mockData.payments[paymentId];
    if (!payment) {
      res.status(404).json({
        result: {
          code: "700.400.580",
          description: "Cannot find transaction",
        },
      });
      return;
    }

    // Like the live API, the payment stays pending until the shopper
    // completes the redirect (simulated via POST /test/payments/:id/complete)
    const result =
      payment.status === "succeeded"
        ? {
            code: "000.000.000",
            description: "Transaction succeeded",
          }
        : {
            code: "000.200.000",
            description: "Transaction pending",
          };

    res.status(200).json({
      id: payment.id,
      paymentType: payment.paymentType,
      paymentBrand: payment.paymentBrand,
      amount: payment.amount,
      currency: payment.currency,
      merchantTransactionId: payment.merchantTransactionId,
      result,
      timestamp: getCurrentTimestamp(),
    });
  } catch (error: any) {
    res.status(500).json({
      result: {
        code: "900.100.100",
        description: `Internal server error: ${error.message}`,
      },
    });
  }
});

// Test-only hook: simulate the shopper completing the redirect, so refund
// and success-path flows can be exercised against the mock
router.post(
  "/test/payments/:paymentId/complete",
  (req: Request, res: Response): void => {
    const { paymentId } = req.params;
    const payment = mockData.payments[paymentId];
    if (!payment) {
      res.status(404).json({
        result: {
          code: "700.400.580",
          description: "Cannot find transaction",
        },
      });
      return;
    }
    payment.status = "succeeded";
    res.status(200).json({ id: paymentId, status: "succeeded" });
  }
);

// Refund (paymentType "RF")
router.post("/payments/:paymentId", (req: Request, res: Response): void => {
  try {
    const { paymentId } = req.params;
    const { authentication, amount, currency, paymentType } = req.body;

    if (!isValidAuthentication(authentication)) {
      authenticationError(res, "Invalid authentication information");
      return;
    }

    if (paymentType !== "RF") {
      validationError(res, [
        {
          name: "paymentType",
          value: paymentType,
          message: "paymentType must be RF for refunds",
        },
      ]);
      return;
    }

    const originalPayment = mockData.payments[paymentId];
    if (!originalPayment) {
      res.status(404).json({
        result: {
          code: "700.400.580",
          description: "Cannot find transaction",
        },
      });
      return;
    }

    const refundId = generateHexId();
    res.status(200).json({
      id: refundId,
      referencedId: paymentId,
      paymentType: "RF",
      amount,
      currency,
      result: {
        code: "000.000.000",
        description: "Transaction succeeded",
      },
      timestamp: getCurrentTimestamp(),
    });
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

console.log("\n📋 Peachpayments Payments API (APM) Mock Endpoints:");
console.log("  GET  /health - Health check");
console.log(
  "  POST /payments - Create APM payment (JSON, nested authentication)"
);
console.log(
  "  GET  /payments/{id} - Payment status (authentication.* query params)"
);
console.log("  POST /payments/{id} - Refund payment (paymentType RF)");
console.log("\n🔐 Test credentials:");
console.log("  entityId: abcdef0123456789abcdef0123456789");
console.log("  userId:   11112222333344445555666677778888");
console.log("  password: peach-test-password");

export default router;
