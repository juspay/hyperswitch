/* eslint-disable no-console */
import * as express from "express";
import cors from "cors";
import { Server } from "http";
import router from "./router.ts";

// Type declarations
type Express = express.Express;
// @ts-ignore - Ignore the missing type declaration for router

// Create the Express application
const app: Express = express.default();
const MOCKSERVER_PORT: number = parseInt(
  process.env.MOCKSERVER_PORT || "3010",
  10
);

// Apply middleware
app.use(cors());
app.use(express.default.json());
app.use(express.default.urlencoded({ extended: true }));

// Use the router for all routes
app.use(router);

// Function to start the server
function startServer(): Server {
  const server: Server = app.listen(MOCKSERVER_PORT, () => {
    console.log(`🚀 Mock Server running on port ${MOCKSERVER_PORT}`);
    console.log(`📍 Server URL: http://localhost:${MOCKSERVER_PORT}`);
    console.log("\n📋 Available Routes:");
    console.log("  GET  /health - Health check");
  });

  // Handle port already in use error
  // eslint-disable-next-line no-undef
  server.on("error", (error: NodeJS.ErrnoException) => {
    if (error.code === "EADDRINUSE") {
      console.error(`❌ ERROR: Port ${MOCKSERVER_PORT} is already in use!`);
      console.error(
        `Please make sure the port is available or set a different port using the MOCKSERVER_PORT environment variable.`
      );
      process.exit(1);
    } else {
      console.error(`❌ ERROR: Failed to start server:`, error.message);
      process.exit(1);
    }
  });

  // Handle graceful shutdown
  process.on("SIGINT", () => {
    console.log("Shutting down mock server...");
    server.close(() => {
      console.log("Mock server has been terminated");
      process.exit(0);
    });
  });

  return server;
}

startServer();
