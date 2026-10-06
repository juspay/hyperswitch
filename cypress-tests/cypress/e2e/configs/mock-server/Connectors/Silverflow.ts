/* eslint-disable no-console */
import * as express from "express";
import type { NextFunction, Request, Response, Router } from "express";

const router: Router = express.default.Router();

// Mock data storage
interface MockData {
  charges: Record<string, any>;
  processorTokens: Record<string, any>;
}

const mockData: MockData = {
  charges: {},
  processorTokens: {},
};

// Mock API credentials
const validCredentials: Record<string, string> = {
  "apk-1wtRxni5IsPsSpBLWpwr": "FWtnOOHAjbD6rNxWWEeVOCj7JXSEPGJQ",
  "apk-testkey123": "testsecret456",
  "api-silverflow": "depends_on_mockserver",
};

// Helper functions
function generateId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).substr(2, 16)}`;
}

function generateAuthCode(): string {
  return Math.floor(100000 + Math.random() * 900000).toString();
}

function getCurrentTimestamp(): string {
  return new Date().toISOString();
}

// Authentication middleware
function authenticateBasic(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  const authHeader = req.headers.authorization;

  if (!authHeader || !authHeader.startsWith("Basic ")) {
    res.status(401).json({
      error: {
        code: "AUTHENTICATION_REQUIRED",
        message: "Missing or invalid Authorization header",
      },
    });
    return;
  }

  try {
    const base64Credentials = authHeader.split(" ")[1];
    const credentials = Buffer.from(base64Credentials, "base64").toString(
      "ascii"
    );
    const [apiKey, apiSecret] = credentials.split(":");

    if (!validCredentials[apiKey] || validCredentials[apiKey] !== apiSecret) {
      res.status(401).json({
        error: {
          code: "INVALID_CREDENTIALS",
          message: "Invalid API key or secret",
        },
      });
      return;
    }

    (req as any).apiKey = apiKey;
    next();
  } catch (error: any) {
    res.status(401).json({
      error: {
        code: "AUTHENTICATION_ERROR",
        message: `Invalid authorization format: ${error.message}`,
      },
    });
    return;
  }
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
  res.json({ status: "OK", timestamp: getCurrentTimestamp() });
});

// 1. POST /charges - Create payment authorization
router.post(
  "/charges",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { merchantAcceptorResolver, card, amount, type, clearingMode } =
        req.body;

      // Basic validation
      if (!merchantAcceptorResolver || !card || !amount || !type) {
        res.status(400).json({
          error: {
            code: "BAD_REQUEST",
            message:
              "Missing required fields: merchantAcceptorResolver, card, amount, type",
          },
        });
        return;
      }

      // Check for fail card - reject payment if fail card is used
      const failCard = "4000000000000002";
      if (card.number === failCard) {
        const chargeKey = generateId("chg");
        const now = getCurrentTimestamp();

        const failedCharge = {
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
            authorization: "declined",
            clearing: "failed",
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
            responseCode: "05",
            responseCodeDescription: "Do not honor",
            authorizationCode: "",
            networkCode: "0005",
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
          actions: [],
        };

        mockData.charges[chargeKey] = failedCharge;

        res.status(201).json(failedCharge);
        return;
      }

      // Idempotency key is accepted but not enforced in mock server

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
          clearing: clearingMode === "manual" ? "pending" : "cleared",
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
        actions: [],
      };

      mockData.charges[chargeKey] = charge;

      res.status(201).json(charge);
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// 2. POST /charges/:chargeKey/clear - Manually clear charge
router.post(
  "/charges/:chargeKey/clear",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { chargeKey } = req.params;
      const { amount, closeCharge, clearAfter, reference } = req.body;

      const charge = mockData.charges[chargeKey];
      if (!charge) {
        res.status(404).json({
          error: {
            code: "NOT_FOUND",
            message: "Charge not found",
          },
        });
        return;
      }

      // Validate amount if provided
      if (
        amount &&
        (typeof amount !== "number" || amount < 1 || amount > 999999999999)
      ) {
        res.status(400).json({
          error: {
            code: "BAD_REQUEST",
            message:
              "Invalid amount: must be integer between 1 and 999999999999",
          },
        });
        return;
      }

      // Determine clearing amount - if not provided, use full charge amount
      const clearingAmount = amount || charge.amount.value;

      // Check if charge is too old (6 months) - mock validation
      const chargeDate = new Date(charge.created);
      const sixMonthsAgo = new Date();
      sixMonthsAgo.setMonth(sixMonthsAgo.getMonth() - 6);

      if (chargeDate < sixMonthsAgo) {
        res.status(409).json({
          error: {
            code: "CONFLICT",
            message: "Charge is too old to be manually cleared",
            details: {
              field: "charge",
              issue: "too-old",
            },
          },
        });
        return;
      }

      const actionKey = generateId("act");
      const clearAction = {
        type: "clearing",
        key: actionKey,
        chargeKey,
        status: "completed",
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
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// 3. POST /charges/:chargeKey/refund - Process refunds
router.post(
  "/charges/:chargeKey/refund",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { chargeKey } = req.params;
      const { refundAmount, reference, clearAfter, dynamicDescriptor } =
        req.body;

      const charge = mockData.charges[chargeKey];
      if (!charge) {
        res.status(404).json({
          error: {
            code: "NOT_FOUND",
            message: "Charge not found",
          },
        });
        return;
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
        res.status(400).json({
          error: {
            code: "BAD_REQUEST",
            message:
              "Invalid refundAmount: must be integer between 1 and 999999999999",
          },
        });
        return;
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
        status: "success",
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
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// 4. POST /charges/:chargeKey/reverse - Reverse charge (void)
router.post(
  "/charges/:chargeKey/reverse",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { chargeKey } = req.params;
      const { replacementAmount = 0, reference } = req.body;

      const charge = mockData.charges[chargeKey];
      if (!charge) {
        res.status(404).json({
          error: {
            code: "NOT_FOUND",
            message: "Charge not found",
          },
        });
        return;
      }

      // Validate replacementAmount if provided
      if (
        replacementAmount &&
        (typeof replacementAmount !== "number" ||
          replacementAmount < 0 ||
          replacementAmount > 999999999999)
      ) {
        res.status(400).json({
          error: {
            code: "BAD_REQUEST",
            message:
              "Invalid replacementAmount: must be integer between 0 and 999999999999",
          },
        });
        return;
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
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// 5. GET /charges/:chargeKey - Get charge status (sync)
router.get(
  "/charges/:chargeKey",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { chargeKey } = req.params;

      const charge = mockData.charges[chargeKey];
      if (!charge) {
        res.status(404).json({
          error: {
            code: "NOT_FOUND",
            message: "Charge not found",
          },
        });
        return;
      }

      res.json(charge);
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// 5a. GET /charges/:chargeKey/actions/:actionKey - Get action status (refund sync)
router.get(
  "/charges/:chargeKey/actions/:actionKey",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { chargeKey, actionKey } = req.params;

      const charge = mockData.charges[chargeKey];
      if (!charge) {
        res.status(404).json({
          error: {
            code: "NOT_FOUND",
            message: "Charge not found",
          },
        });
        return;
      }

      const action = charge.actions.find((a: any) => a.key === actionKey);
      if (!action) {
        res.status(404).json({
          error: {
            code: "NOT_FOUND",
            message: "Action not found",
          },
        });
        return;
      }

      res.json(action);
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// 6. POST /processorTokens - Create processor tokens for vaulting
router.post(
  "/processorTokens",
  authenticateBasic,
  (req: Request, res: Response): void => {
    try {
      const { reference, cardData } = req.body;

      if (!cardData || !cardData.number) {
        res.status(400).json({
          error: {
            code: "BAD_REQUEST",
            message: "Missing cardData or number",
          },
        });
        return;
      }

      const processorTokenKey = generateId("ptk");
      const cardNumber = cardData.number.toString();

      // Determine card network based on first digits
      let network = "unknown";
      if (cardNumber.startsWith("4")) network = "visa";
      else if (cardNumber.startsWith("5") || cardNumber.startsWith("2"))
        network = "mastercard";
      else if (cardNumber.startsWith("3")) network = "amex";
      else if (cardNumber.startsWith("6")) network = "discover";

      const processorToken = {
        key: processorTokenKey,
        agentKey: generateId("agt"),
        last4: cardNumber.substring(cardNumber.length - 4),
        status: "active",
        reference,
        cardInfo: [
          {
            infoSource: "card",
            network,
            primaryNetwork: true,
          },
        ],
        created: getCurrentTimestamp(),
        cvcPresent: !!cardData.cvc,
        version: 1,
      };

      mockData.processorTokens[processorTokenKey] = processorToken;

      res.status(201).json(processorToken);
    } catch (error: any) {
      res.status(500).json({
        error: {
          code: "INTERNAL_SERVER_ERROR",
          message: error.message,
        },
      });
    }
  }
);

// Error handling middleware
router.use(
  (err: Error, req: Request, res: Response, _next: NextFunction): void => {
    console.error("Error:", err);
    res.status(500).json({
      error: {
        code: "INTERNAL_SERVER_ERROR",
        message: "An unexpected error occurred",
      },
    });
  }
);

// 404 handler
router.use((req: Request, res: Response): void => {
  res.status(404).json({
    error: {
      code: "NOT_FOUND",
      message: `Endpoint ${req.method} ${req.originalUrl} not found`,
    },
  });
});

// Log available endpoints for debugging purposes
console.log("\n📋 Silverflow Mock API Endpoints:");
console.log("  GET  /health - Health check");
console.log("  POST /charges - Create payment authorization");
console.log("  POST /charges/{chargeKey}/clear - Manually clear charges");
console.log("  POST /charges/{chargeKey}/refund - Process refunds");
console.log("  POST /charges/{chargeKey}/reverse - Reverse charge (void)");
console.log("  GET  /charges/{chargeKey} - Get charge status");
console.log(
  "  GET  /charges/{chargeKey}/actions/{actionKey} - Get action status"
);
console.log("  POST /processorTokens - Create processor tokens");
console.log("\n🔐 Authentication (SignatureKey format):");
console.log("  auth_type: SignatureKey");
console.log("  api_key: apk-testkey123");
console.log("  key1: testsecret456 (merchant_acceptor_key)");
console.log("  api_secret: testsecret456");
console.log("\n📖 Use Basic Auth with base64 encoded api_key:api_secret");
console.log(
  "  Example: Authorization: Basic <base64(apk-testkey123:testsecret456)>"
);

export default router;
