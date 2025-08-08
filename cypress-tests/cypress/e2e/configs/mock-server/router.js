/* eslint-disable no-console */
import express from "express";
import silverflowApp from "./Silverflow.js";

const mockRouters = {
  silverflow: silverflowApp,
};
// Create a router
const router = express.Router();

// Log requests to the router
function logRequest(req, res, next) {
  console.log(`Router: ${req.method} ${req.path}`);
  next();
}

// Health check function
function healthCheck(req, res) {
  res.json({
    status: "OK",
    service: "Router",
    timestamp: new Date().toISOString(),
    message: "Router is functioning correctly",
  });
}

// Error handling function
// eslint-disable-next-line no-unused-vars
function handleErrors(err, req, res, next) {
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
  router.use(`/${name}`, (req, res, next) => {
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
