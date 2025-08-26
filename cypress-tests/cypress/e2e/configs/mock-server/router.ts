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

// Define direct routes
router.get("/health", healthCheck);

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
