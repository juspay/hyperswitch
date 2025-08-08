/* eslint-disable no-console */
import express from "express";
import cors from "cors";
import router from "./router.js";

// Create the Express application
const app = express();
const MOCKSERVER_PORT = process.env.MOCKSERVER_PORT || 3010;

// Apply middleware
app.use(cors());
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Use the router for all routes
app.use(router);

// Function to start the server
function startServer() {
  const server = app.listen(MOCKSERVER_PORT, () => {
    console.log(`🚀 Mock Server running on port ${MOCKSERVER_PORT}`);
    console.log(`📍 Server URL: http://localhost:${MOCKSERVER_PORT}`);
    console.log("\n📋 Available Routes:");
    console.log("  GET  /health - Health check");
  });

  // Handle port already in use error
  server.on("error", (error) => {
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
