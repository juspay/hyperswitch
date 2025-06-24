#!/usr/bin/env node
/* eslint-disable no-console */

import fs from "fs";
import http from "http";
import path, { dirname } from "path";
import { fileURLToPath } from "url";

// Get the directory of the current module
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Define the port
const PORT = process.env.PORT || 3333;

// MIME types
const mimeTypes = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
};

// Create server
const server = http.createServer((req, res) => {
  // Parse URL
  let filePath = path.join(__dirname, "..", req.url);

  // Default to index.html for root
  if (req.url === "/") {
    filePath = path.join(__dirname, "..", "dashboard", "index.html");
  }

  // Check if path is a directory and append index.html
  if (fs.existsSync(filePath) && fs.statSync(filePath).isDirectory()) {
    filePath = path.join(filePath, "index.html");
  }

  // Get file extension
  const extname = String(path.extname(filePath)).toLowerCase();
  const contentType = mimeTypes[extname] || "application/octet-stream";

  // Read and serve the file
  fs.readFile(filePath, (error, content) => {
    if (error) {
      if (error.code === "ENOENT") {
        res.writeHead(404, { "Content-Type": "text/html" });
        res.end("<h1>404 - File Not Found</h1>", "utf-8");
      } else {
        res.writeHead(500);
        res.end(`Server Error: ${error.code}`, "utf-8");
      }
    } else {
      res.writeHead(200, { "Content-Type": contentType });
      res.end(content, "utf-8");
    }
  });
});

// Start server
server.listen(PORT, () => {
  console.log(`\n🚀 Dashboard server running at http://localhost:${PORT}/`);
  console.log(
    `📊 Open http://localhost:${PORT}/dashboard/ to view the dashboard`
  );
  console.log("\nPress Ctrl+C to stop the server\n");
});

// Handle graceful shutdown
process.on("SIGINT", () => {
  console.log("\n\nShutting down server...");
  server.close(() => {
    console.log("Server closed");
    process.exit(0);
  });
});
