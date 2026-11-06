/* eslint-disable no-console */
import * as express from "express";
import type { NextFunction, Request, Response, Router } from "express";

// Mock of the ACI / OPPWA "Server-to-Server" card API, enough for the card
// flows WITHOUT 3DS. Point the router at it with
//   ROUTER__CONNECTORS__ACI__BASE_URL=http://localhost:3010/aci/
// (the trailing slash matters: the connector appends `v1/...` directly).
//
// Contract, derived from crates/hyperswitch_connectors/src/connectors/aci.rs
// and aci/transformers.rs:
//
//   POST /v1/payments                     - authorize: paymentType DB (auto
//                                           capture) or PA (manual capture).
//                                           Also the SetupMandate flow (PA
//                                           with createRegistration=true).
//   POST /v1/registrations/:id/payments   - MIT charge against a stored
//                                           registration (minimal support)
//   POST /v1/payments/:id                 - back-office op on :id, selected by
//                                           paymentType: CP capture, RV void,
//                                           RF refund
//   GET  /v1/payments/:id?entityId=...    - transaction status (PSync)
//
// Every request carries `Authorization: Bearer <api_key>`. POST bodies are
// application/x-www-form-urlencoded and carry `entityId` (the MCA `key1`); the
// GET carries it as a query param. The connector maps the MCA BodyKey auth as
// `api_key` -> bearer token and `key1` -> entityId.
//
// Refund sync: `ConnectorIntegration<RSync, ..>` for ACI is an empty impl, so
// the router never calls ACI for it and keeps the status the refund returned.
// GET /v1/payments/:id answers for refund ids anyway.
//
// Result codes are the ones the connector classifies in aci_result_codes.rs:
//   000.100.110 success (OPPWA "Integrator Test Mode")  -> Succeeded
//   anything in FAILURE_CODES, returned with HTTP 400    -> error response
// Success on PA maps to `requires_capture`, on DB to `succeeded`, on RV to
// `cancelled`; CP maps to `succeeded`/`partially_captured` in the router.
//
// Response fields the transformer requires (deserialization fails without
// them): id, ndc, timestamp, buildNumber, result{code,description}. Capture
// additionally requires referencedId, paymentType, amount, currency and
// descriptor. `paymentBrand` must be one of the connector's PaymentBrand
// variants (VISA, MASTER, AMEX, ...), so it is only echoed / derived from the
// BIN when known.
//
// Cypress assertions fed by these responses:
//   payment_method_data.card.auth_code      <- resultDetails.AuthCode
//   payment_method_data.card.payment_checks <- result.cvvResponse and
//                                              resultDetails.AcquirerResponse
//   connector_transaction_id                <- id
//
// Magic triggers (only meaningful against this mock):
//   card 4012001037461114 (3DS "technical error" card from the permitted test
//   cards list)              -> 100.390.112 "Technical Error in 3D system"
//   card expiry in the past  -> 100.100.303 "card expired"
//
// 3DS is out of scope: a request with customParameters[3DS2_enrolled]=true
// and the standalone /v1/threeDSecure endpoints are rejected with 200.300.404.
//
// Card data: the PAN is kept only as BIN + last 4, CVV is never stored, and
// request logging masks the PAN and drops the CVV.

const router: Router = express.default.Router();

// Obvious dummy placeholders. Use these in the mock Creds.json entry:
//   api_key (bearer token) / key1 (entityId)
const MOCK_API_KEY = "aci-mock-api-key";
const MOCK_ENTITY_ID = "aci-mock-entity-id";

const SUCCESS_CODE = "000.100.110";
const SUCCESS_DESCRIPTION =
  "Request successfully processed in 'Merchant in Integrator Test Mode'";

const DECLINE_3DS_TECHNICAL_ERROR_CARD = "4012001037461114";

interface StoredCard {
  bin: string;
  last4Digits: string;
  holder?: string;
  expiryMonth: string;
  expiryYear: string;
}

interface Transaction {
  id: string;
  entityId: string;
  paymentType: "PA" | "DB" | "CP" | "RV" | "RF";
  amountMinor: number;
  currency: string;
  // Parent transaction for CP / RV / RF
  referencedId?: string;
  paymentBrand?: string;
  card?: StoredCard;
  merchantTransactionId?: string;
  registrationId?: string;
  // Book-keeping for PA / DB roots
  capturedMinor: number;
  refundedMinor: number;
  reversed: boolean;
  response: Record<string, any>;
}

interface Registration {
  id: string;
  entityId: string;
  card?: StoredCard;
  paymentBrand?: string;
}

const mockData: {
  transactions: Record<string, Transaction>;
  registrations: Record<string, Registration>;
} = {
  transactions: {},
  registrations: {},
};

function generateHexId(): string {
  let id = "";
  for (let i = 0; i < 32; i += 1) {
    id += Math.floor(Math.random() * 16).toString(16);
  }
  return id;
}

function generateDigits(length: number): string {
  let out = "";
  for (let i = 0; i < length; i += 1) {
    out += Math.floor(Math.random() * 10).toString();
  }
  return out;
}

// OPPWA timestamp format, e.g. "2026-10-30 10:15:00.123+0000"
function getAciTimestamp(): string {
  return new Date().toISOString().replace("T", " ").replace("Z", "+0000");
}

function envelope(entityId: string): Record<string, string> {
  return {
    buildNumber: "mock-aci-build",
    timestamp: getAciTimestamp(),
    ndc: `${entityId}_${generateHexId()}`,
  };
}

// StringMajorUnit amounts, always two decimals ("60.00")
const AMOUNT_REGEX = /^\d{1,10}\.\d{2}$/;

function toMinor(amount: string): number {
  const [major, minor] = amount.split(".");
  return parseInt(major, 10) * 100 + parseInt(minor, 10);
}

function toMajor(minor: number): string {
  return `${Math.floor(minor / 100)}.${String(minor % 100).padStart(2, "0")}`;
}

// The app-level urlencoded parser may be `extended` (qs) or not, so
// `customParameters[x]` arrives either flat or nested.
function getCustomParam(body: any, name: string): string | undefined {
  const flat = body[`customParameters[${name}]`];
  if (flat !== undefined) return String(flat);
  const nested = body.customParameters?.[name];
  return nested !== undefined ? String(nested) : undefined;
}

function customParametersOf(body: any): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(body)) {
    const match = /^customParameters\[(.+)\]$/.exec(key);
    if (match) out[match[1]] = String(value);
  }
  if (body.customParameters && typeof body.customParameters === "object") {
    for (const [key, value] of Object.entries(body.customParameters)) {
      out[key] = String(value);
    }
  }
  return out;
}

function luhnValid(pan: string): boolean {
  let sum = 0;
  let double = false;
  for (let i = pan.length - 1; i >= 0; i -= 1) {
    let digit = parseInt(pan[i], 10);
    if (double) {
      digit *= 2;
      if (digit > 9) digit -= 9;
    }
    sum += digit;
    double = !double;
  }
  return sum % 10 === 0;
}

// Only values the connector's `PaymentBrand` enum can deserialize.
function brandFromBin(pan: string): string | undefined {
  if (pan.startsWith("4")) return "VISA";
  if (/^5[1-5]/.test(pan) || /^2[2-7]/.test(pan)) return "MASTER";
  if (/^3[47]/.test(pan)) return "AMEX";
  return undefined;
}

function maskPan(pan: unknown): string {
  const digits = String(pan ?? "");
  return digits.length >= 4 ? `****${digits.slice(-4)}` : "****";
}

// Never log card data (PCI): mask the PAN to last 4, drop the CVV, holder,
// expiry and any 3DS cryptogram.
const REDACTED_CARD_FIELDS = ["cvv", "holder", "expiryMonth", "expiryYear"];

function sanitizeForLog(body: any): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(body ?? {})) {
    if (
      REDACTED_CARD_FIELDS.some((field) => key === `card.${field}`) ||
      key.startsWith("threeDSecure.")
    ) {
      out[key] = "[redacted]";
    } else if (key === "card.number") {
      out[key] = maskPan(value);
    } else if (key === "card" && value && typeof value === "object") {
      const card = value as Record<string, unknown>;
      out[key] = {
        ...card,
        ...Object.fromEntries(
          REDACTED_CARD_FIELDS.map((field) => [field, "[redacted]"])
        ),
        number: maskPan(card.number),
      };
    } else {
      out[key] = value;
    }
  }
  return out;
}

function sendError(
  res: Response,
  httpStatus: number,
  entityId: string,
  code: string,
  description: string,
  extra: Record<string, any> = {}
): void {
  res.status(httpStatus).json({
    ...extra,
    result: { code, description, ...(extra.result ?? {}) },
    ...envelope(entityId),
  });
}

function parameterError(
  res: Response,
  entityId: string,
  parameterErrors: { name: string; value?: string; message: string }[]
): void {
  sendError(res, 400, entityId, "200.300.404", "invalid or missing parameter", {
    result: { parameterErrors },
  });
}

function isAuthorized(req: Request): boolean {
  return req.headers.authorization === `Bearer ${MOCK_API_KEY}`;
}

function authenticationError(res: Response): void {
  sendError(
    res,
    401,
    MOCK_ENTITY_ID,
    "800.900.300",
    "invalid authentication information"
  );
}

function successResultDetails(): Record<string, string> {
  return {
    ExtendedDescription: "Transaction succeeded",
    AcquirerResponse: "00",
    AuthCode: generateDigits(6),
    ConnectorTxID1: generateDigits(12),
  };
}

function rootOf(tx: Transaction): Transaction {
  if (tx.paymentType === "CP" && tx.referencedId) {
    return mockData.transactions[tx.referencedId] ?? tx;
  }
  return tx;
}

router.use(express.default.urlencoded({ extended: true }));

router.use((req: Request, _res: Response, next: NextFunction): void => {
  console.log(`${getAciTimestamp()} - ACI ${req.method} ${req.path}`);
  if (req.body && Object.keys(req.body).length > 0) {
    console.log("Body:", JSON.stringify(sanitizeForLog(req.body), null, 2));
  }
  next();
});

router.get("/health", (_req: Request, res: Response): void => {
  res.json({ status: "OK", timestamp: getAciTimestamp() });
});

router.use((req: Request, res: Response, next: NextFunction): void => {
  if (!isAuthorized(req)) {
    authenticationError(res);
    return;
  }
  const entityId =
    req.method === "GET" ? req.query.entityId : req.body?.entityId;
  if (entityId !== MOCK_ENTITY_ID) {
    // OPPWA answers an unknown/mismatched entity with the same auth code
    authenticationError(res);
    return;
  }
  next();
});

// Shared by the initial payment and the registration (MIT) charge.
function createPayment(
  req: Request,
  res: Response,
  registration?: Registration
): void {
  const body = req.body;
  const entityId = String(body.entityId);
  const { amount, currency, paymentType } = body;

  const errors: { name: string; value?: string; message: string }[] = [];
  if (!amount || !AMOUNT_REGEX.test(String(amount))) {
    errors.push({
      name: "amount",
      value: amount,
      message: "must match ^[0-9]{1,10}(\\.[0-9]{2})?$",
    });
  }
  if (!currency || !/^[A-Z]{3}$/.test(String(currency))) {
    errors.push({ name: "currency", value: currency, message: "invalid" });
  }
  if (paymentType !== "DB" && paymentType !== "PA") {
    errors.push({
      name: "paymentType",
      value: paymentType,
      message: "must be DB or PA for an initial payment",
    });
  }
  if (getCustomParam(body, "3DS2_enrolled") === "true") {
    errors.push({
      name: "customParameters[3DS2_enrolled]",
      value: "true",
      message: "3DS flows are not supported by the ACI mock",
    });
  }

  let card: StoredCard | undefined = registration?.card;
  let paymentBrand: string | undefined =
    body.paymentBrand ?? registration?.paymentBrand;
  let pan = "";
  if (!registration) {
    pan = String(body["card.number"] ?? body.card?.number ?? "");
    const holder = body["card.holder"] ?? body.card?.holder;
    const expiryMonth = String(
      body["card.expiryMonth"] ?? body.card?.expiryMonth ?? ""
    );
    const expiryYear = String(
      body["card.expiryYear"] ?? body.card?.expiryYear ?? ""
    );
    if (!/^\d{12,19}$/.test(pan) || !luhnValid(pan)) {
      errors.push({
        name: "card.number",
        value: maskPan(pan),
        message: "invalid card number",
      });
    }
    if (!holder) {
      errors.push({ name: "card.holder", message: "is required" });
    }
    if (!/^(0[1-9]|1[0-2])$/.test(expiryMonth)) {
      errors.push({
        name: "card.expiryMonth",
        value: expiryMonth,
        message: "must be MM",
      });
    }
    if (!/^\d{4}$/.test(expiryYear)) {
      errors.push({
        name: "card.expiryYear",
        value: expiryYear,
        message: "must be YYYY",
      });
    }
    card = {
      bin: pan.slice(0, 6),
      last4Digits: pan.slice(-4),
      holder: holder ? String(holder) : undefined,
      expiryMonth,
      expiryYear,
    };
    paymentBrand = paymentBrand ?? brandFromBin(pan);
  }
  if (errors.length > 0) {
    parameterError(res, entityId, errors);
    return;
  }

  const id = generateHexId();
  const amountMinor = toMinor(String(amount));
  const hasCvv = body["card.cvv"] !== undefined || body.card?.cvv !== undefined;
  const common = {
    id,
    paymentType,
    paymentBrand,
    amount: toMajor(amountMinor),
    currency,
    descriptor: body.descriptor ?? `MOCK ${id.slice(0, 8)}`,
    merchantTransactionId: body.merchantTransactionId,
    card,
    customParameters: customParametersOf(body),
  };

  // Decline triggers
  const now = new Date();
  const expired =
    card &&
    (parseInt(card.expiryYear, 10) < now.getUTCFullYear() ||
      (parseInt(card.expiryYear, 10) === now.getUTCFullYear() &&
        parseInt(card.expiryMonth, 10) < now.getUTCMonth() + 1));
  let decline: { code: string; description: string } | undefined;
  if (pan === DECLINE_3DS_TECHNICAL_ERROR_CARD) {
    decline = {
      code: "100.390.112",
      description: "Technical Error in 3D system",
    };
  } else if (expired) {
    decline = { code: "100.100.303", description: "card expired" };
  }
  if (decline) {
    sendError(res, 400, entityId, decline.code, decline.description, {
      ...common,
      resultDetails: {
        ExtendedDescription: decline.description,
        AcquirerResponse: "05",
      },
    });
    return;
  }

  let registrationId: string | undefined;
  if (String(body.createRegistration) === "true" && !registration) {
    registrationId = generateHexId();
    mockData.registrations[registrationId] = {
      id: registrationId,
      entityId,
      card,
      paymentBrand,
    };
  }

  const response: Record<string, any> = {
    ...common,
    ...(registrationId ? { registrationId } : {}),
    result: {
      code: SUCCESS_CODE,
      description: SUCCESS_DESCRIPTION,
      ...(hasCvv ? { cvvResponse: "M" } : {}),
    },
    resultDetails: successResultDetails(),
    ...envelope(entityId),
  };

  mockData.transactions[id] = {
    id,
    entityId,
    paymentType,
    amountMinor,
    currency,
    paymentBrand,
    card,
    merchantTransactionId: body.merchantTransactionId,
    registrationId: registrationId ?? registration?.id,
    capturedMinor: paymentType === "DB" ? amountMinor : 0,
    refundedMinor: 0,
    reversed: false,
    response,
  };

  res.status(200).json(response);
}

router.post("/v1/payments", (req: Request, res: Response): void => {
  createPayment(req, res);
});

router.post(
  "/v1/registrations/:registrationId/payments",
  (req: Request, res: Response): void => {
    const registration =
      mockData.registrations[req.params.registrationId as string];
    if (!registration || registration.entityId !== req.body.entityId) {
      sendError(
        res,
        400,
        String(req.body.entityId),
        "700.400.580",
        "cannot find transaction"
      );
      return;
    }
    createPayment(req, res, registration);
  }
);

// Back-office operations on an existing transaction
router.post("/v1/payments/:id", (req: Request, res: Response): void => {
  const body = req.body;
  const entityId = String(body.entityId);
  const target = mockData.transactions[req.params.id as string];
  if (!target || target.entityId !== entityId) {
    sendError(res, 400, entityId, "700.400.580", "cannot find transaction");
    return;
  }

  const { paymentType } = body;
  if (paymentType !== "CP" && paymentType !== "RV" && paymentType !== "RF") {
    parameterError(res, entityId, [
      {
        name: "paymentType",
        value: paymentType,
        message: "must be CP, RV or RF for a referenced transaction",
      },
    ]);
    return;
  }

  // RV carries no amount; CP and RF must
  let amountMinor = 0;
  if (paymentType !== "RV") {
    const errors: { name: string; value?: string; message: string }[] = [];
    if (!body.amount || !AMOUNT_REGEX.test(String(body.amount))) {
      errors.push({ name: "amount", value: body.amount, message: "invalid" });
    }
    if (body.currency !== target.currency) {
      errors.push({
        name: "currency",
        value: body.currency,
        message: `must match the referenced transaction (${target.currency})`,
      });
    }
    if (errors.length > 0) {
      parameterError(res, entityId, errors);
      return;
    }
    amountMinor = toMinor(String(body.amount));
  }

  const root = rootOf(target);
  let workflowError: { code: string; description: string } | undefined;
  if (paymentType === "CP") {
    if (
      target.paymentType !== "PA" ||
      target.reversed ||
      amountMinor <= 0 ||
      target.capturedMinor + amountMinor > target.amountMinor
    ) {
      workflowError = {
        code: "700.400.100",
        description:
          "cannot capture (PA value exceeded, PA reverted or invalid workflow?)",
      };
    }
  } else if (paymentType === "RV") {
    if (
      target.paymentType !== "PA" ||
      target.reversed ||
      target.capturedMinor > 0
    ) {
      workflowError = {
        code: "700.400.300",
        description:
          "cannot reverse (already refunded|reversed, invalid workflow or amount exceeded)",
      };
    }
  } else if (
    // RF: only against money that actually moved (DB, CP, or a captured PA),
    // never more than what is still unrefunded
    !["DB", "CP", "PA"].includes(target.paymentType) ||
    root.reversed ||
    amountMinor <= 0 ||
    root.refundedMinor + amountMinor > root.capturedMinor
  ) {
    workflowError = {
      code: "700.400.200",
      description:
        "cannot refund (refund volume exceeded or tx reversed or invalid workflow?)",
    };
  }

  const id = generateHexId();
  const common = {
    id,
    referencedId: target.id,
    paymentType,
    paymentBrand: target.paymentBrand,
    amount: toMajor(paymentType === "RV" ? target.amountMinor : amountMinor),
    currency: target.currency,
    descriptor: `MOCK ${id.slice(0, 8)}`,
    merchantTransactionId: target.merchantTransactionId,
  };

  if (workflowError) {
    sendError(
      res,
      400,
      entityId,
      workflowError.code,
      workflowError.description,
      common
    );
    return;
  }

  if (paymentType === "CP") {
    target.capturedMinor += amountMinor;
  } else if (paymentType === "RV") {
    target.reversed = true;
  } else {
    root.refundedMinor += amountMinor;
  }

  const response: Record<string, any> = {
    ...common,
    result: { code: SUCCESS_CODE, description: SUCCESS_DESCRIPTION },
    resultDetails:
      paymentType === "RF"
        ? {
            ExtendedDescription: "Transaction succeeded",
            AcquirerResponse: "00",
          }
        : successResultDetails(),
    ...envelope(entityId),
  };

  mockData.transactions[id] = {
    id,
    entityId,
    paymentType,
    amountMinor: paymentType === "RV" ? target.amountMinor : amountMinor,
    currency: target.currency,
    referencedId: target.id,
    paymentBrand: target.paymentBrand,
    merchantTransactionId: target.merchantTransactionId,
    // A capture is itself refundable up to its own amount via its root PA
    capturedMinor: 0,
    refundedMinor: 0,
    reversed: false,
    response,
  };

  res.status(200).json(response);
});

// Transaction status (PSync). Returns the stored transaction as OPPWA does,
// with a fresh envelope.
router.get("/v1/payments/:id", (req: Request, res: Response): void => {
  const entityId = String(req.query.entityId);
  const tx = mockData.transactions[req.params.id as string];
  if (!tx || tx.entityId !== entityId) {
    sendError(res, 400, entityId, "700.400.580", "cannot find transaction");
    return;
  }
  res.status(200).json({ ...tx.response, ...envelope(entityId) });
});

// Standalone 3DS (PreAuthentication / PostAuthentication) is out of scope
router.all(/^\/v1\/threeDSecure/, (req: Request, res: Response): void => {
  parameterError(res, String(req.body?.entityId ?? req.query.entityId), [
    {
      name: "threeDSecure",
      message: "3DS flows are not supported by the ACI mock",
    },
  ]);
});

// 404 handler
router.use((req: Request, res: Response): void => {
  sendError(
    res,
    404,
    MOCK_ENTITY_ID,
    "200.300.404",
    `Endpoint ${req.method} ${req.originalUrl} not found in ACI mock`
  );
});

console.log("\n📋 ACI (OPPWA) Card API Mock Endpoints:");
console.log("  GET  /health - Health check");
console.log("  POST /v1/payments - Authorize (paymentType DB or PA)");
console.log(
  "  POST /v1/registrations/{id}/payments - Charge a stored registration"
);
console.log("  POST /v1/payments/{id} - Capture (CP), void (RV), refund (RF)");
console.log("  GET  /v1/payments/{id}?entityId=... - Transaction status");
console.log("\n🔐 Test credentials (dummy placeholders):");
console.log(`  api_key (Bearer): ${MOCK_API_KEY}`);
console.log(`  key1 (entityId):  ${MOCK_ENTITY_ID}`);

export default router;
