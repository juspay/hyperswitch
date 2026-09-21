/* eslint-disable no-console */
import * as express from "express";
import type {
  Request,
  Response,
  NextFunction,
  Router,
  RequestHandler,
} from "express";
// Import from TypeScript version
import silverflowApp from "./Connectors/Silverflow.ts";
import celeroApp from "./Connectors/Celero.ts";
import affirmApp from "./Connectors/Affirm.ts";

// TODO: Update to import from TypeScript version once fully tested
// import silverflowApp from "./Silverflow";

interface MockRouters {
  [key: string]: RequestHandler;
}

const mockRouters: MockRouters = {
  silverflow: silverflowApp,
  celero: celeroApp,
  affirm: affirmApp,
};

// Create a router
const router: Router = express.default.Router();

// Log requests to the router
function logRequest(req: Request, res: Response, next: NextFunction): void {
  console.log(`Router: ${req.method} ${req.path}`);
  next();
}

// Health check function
function healthCheck(req: Request, res: Response): void {
  res.json({
    status: "OK",
    service: "Router",
    timestamp: new Date().toISOString(),
    message: "Router is functioning correctly",
  });
}

// Error handling function
function handleErrors(
  err: Error,
  req: Request,
  res: Response,
  _next: NextFunction
): void {
  console.error("Router Error:", err);
  res.status(500).json({
    error: {
      code: "ROUTER_ERROR",
      message: "An error occurred in the router",
      details: err.message,
    },
  });
}

// Apply middleware
router.use(logRequest);

// Outgoing webhook capture store, keyed by merchant_id
interface CapturedWebhook {
  headers: Record<string, unknown>;
  body: Record<string, unknown>;
  receivedAt: string;
}

const capturedWebhooks = new Map<string, CapturedWebhook[]>();

function captureOutgoingWebhook(req: Request, res: Response): void {
  const merchantId =
    typeof req.body?.merchant_id === "string"
      ? req.body.merchant_id
      : "_unknown";
  const entry: CapturedWebhook = {
    headers: req.headers as Record<string, unknown>,
    body: req.body,
    receivedAt: new Date().toISOString(),
  };
  const existing = capturedWebhooks.get(merchantId) || [];
  existing.push(entry);
  capturedWebhooks.set(merchantId, existing);
  res.status(200).json({ received: true });
}

function getCapturedOutgoingWebhooks(req: Request, res: Response): void {
  const merchantId =
    typeof req.query.merchant_id === "string" ? req.query.merchant_id : "";
  const captured = merchantId ? capturedWebhooks.get(merchantId) || [] : [];
  res.status(200).json({ captured });
}

function resetCapturedOutgoingWebhooks(_req: Request, res: Response): void {
  capturedWebhooks.clear();
  res.status(200).json({ reset: true });
}

// Define direct routes
router.get("/health", healthCheck);
router.post("/webhook", captureOutgoingWebhook);
router.get("/webhook/captured", getCapturedOutgoingWebhooks);
router.delete("/webhook/captured", resetCapturedOutgoingWebhooks);

// Forward routes for all mock routers
for (const routerName of Object.keys(mockRouters)) {
  const name = routerName;
  const routerApp = mockRouters[routerName];
  console.log(`CONNECTOR /${name}`);
  router.use(`/${name}`, (req: Request, res: Response, next: NextFunction) => {
    // Modify the path to remove the router name prefix
    const originalUrl = req.url;
    req.url = originalUrl.replace(new RegExp(`^\\/${name}`), "");

    // Forward to the appropriate app
    routerApp(req, res, next);

    // Restore the original URL after processing the request
    req.url = originalUrl;
  });
}

// Error handling
router.use(handleErrors);

export default router;
