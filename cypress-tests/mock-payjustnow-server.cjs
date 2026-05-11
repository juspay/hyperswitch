#!/usr/bin/env node
/**
 * Mock PayJustNow Server
 * Simulates PayJustNow API endpoints for local testing
 */

const http = require("http");
const url = require("url");

const PORT = 9999;

// In-memory storage for checkouts
const checkouts = new Map();

function generateToken() {
  return "pjn_" + Math.random().toString(36).substring(2, 15);
}

function generateRequestId() {
  return "req_" + Math.random().toString(36).substring(2, 15);
}

const server = http.createServer((req, res) => {
  // Enable CORS
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization");

  if (req.method === "OPTIONS") {
    res.writeHead(200);
    res.end();
    return;
  }

  const parsedUrl = url.parse(req.url, true);
  const path = parsedUrl.pathname;

  console.log(`[${new Date().toISOString()}] ${req.method} ${path}`);

  // Parse request body
  let body = "";
  req.on("data", (chunk) => {
    body += chunk.toString();
  });

  req.on("end", () => {
    let requestData = {};
    if (body) {
      try {
        requestData = JSON.parse(body);
      } catch (e) {
        console.error("Failed to parse request body:", e.message);
      }
    }

    // Route handlers
    if (path === "/api/v1/checkouts" && req.method === "POST") {
      // Create checkout
      const checkoutToken = generateToken();
      const requestId = requestData.request_id || generateRequestId();

      const checkoutData = {
        checkout_token: checkoutToken,
        payment_url: `http://localhost:${PORT}/mock-checkout/${checkoutToken}`,
        request_id: requestId,
        status: "PENDING_ORDER",
        created_at: new Date().toISOString(),
        payjustnow: requestData.payjustnow || {},
      };

      checkouts.set(checkoutToken, checkoutData);

      console.log("Created checkout:", checkoutToken);

      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          payment_url: checkoutData.payment_url,
          checkout_token: checkoutToken,
        })
      );
      return;
    }

    // Get checkout status (sync)
    const syncMatch = path.match(/^\/api\/v1\/checkouts\/([^\/]+)$/);
    if (syncMatch && req.method === "GET") {
      const checkoutToken = syncMatch[1];
      const checkout = checkouts.get(checkoutToken);

      if (!checkout) {
        res.writeHead(404, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: "Checkout not found" }));
        return;
      }

      // Simulate successful payment after some time
      const createdTime = new Date(checkout.created_at).getTime();
      const now = Date.now();
      const elapsedSeconds = (now - createdTime) / 1000;

      let status = checkout.status;
      let paymentReference = null;

      // Auto-approve after 5 seconds (simulating user completing payment)
      if (elapsedSeconds > 5 && status === "PENDING_ORDER") {
        status = "PAID";
        paymentReference = Math.floor(Math.random() * 1000000000);
        checkout.status = status;
        checkout.payment_reference = paymentReference;
      }

      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          checkout_token: checkoutToken,
          payment_url: checkout.payment_url,
          checkout_payment_status: status,
          payment_reference: paymentReference,
        })
      );
      return;
    }

    // Refund
    const refundMatch = path.match(/^\/api\/v1\/checkouts\/([^\/]+)\/refund$/);
    if (refundMatch && req.method === "POST") {
      const checkoutToken = refundMatch[1];
      const checkout = checkouts.get(checkoutToken);

      if (!checkout) {
        res.writeHead(404, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: "Checkout not found" }));
        return;
      }

      const requestId = requestData.request_id || generateRequestId();

      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          request_id: requestId,
          refunded_amount_cents: requestData.refund_amount_cents || 0,
          refund_status: "SUCCESS",
          refund_status_at: new Date().toISOString(),
          refund_status_description: "Refund processed successfully",
        })
      );
      return;
    }

    // Mock checkout page (redirect simulation)
    const checkoutPageMatch = path.match(/^\/mock-checkout\/(.+)$/);
    if (checkoutPageMatch) {
      const checkoutToken = checkoutPageMatch[1];
      const checkout = checkouts.get(checkoutToken);

      if (!checkout) {
        res.writeHead(404, { "Content-Type": "text/html" });
        res.end("<h1>Checkout not found</h1>");
        return;
      }

      // Update status to paid when user visits the checkout page
      checkout.status = "PAID";
      checkout.payment_reference = Math.floor(Math.random() * 1000000000);

      const returnUrl = checkout.payjustnow?.confirm_redirect_url || "https://example.com/return";

      res.writeHead(200, { "Content-Type": "text/html" });
      res.end(`
<!DOCTYPE html>
<html>
<head>
  <title>PayJustNow Mock Checkout</title>
  <style>
    body { font-family: Arial, sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; }
    .success { color: green; }
    button { padding: 10px 20px; font-size: 16px; cursor: pointer; }
  </style>
</head>
<body>
  <h1>PayJustNow Mock Checkout</h1>
  <p>This is a simulated PayJustNow checkout page for testing.</p>
  <p class="success">Payment approved!</p>
  <p>Checkout Token: ${checkoutToken}</p>
  <p>Amount: ${checkout.payjustnow?.order_amount_cents || 0} cents</p>
  <button onclick="completePayment()">Complete Payment</button>
  <script>
    function completePayment() {
      window.location.href = "${returnUrl}?checkout_token=${checkoutToken}&status=success";
    }
    // Auto-redirect after 2 seconds
    setTimeout(completePayment, 2000);
  </script>
</body>
</html>
      `);
      return;
    }

    // Health check
    if (path === "/health" && req.method === "GET") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ status: "healthy" }));
      return;
    }

    // Default 404
    res.writeHead(404, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: "Not found" }));
  });
});

server.listen(PORT, () => {
  console.log(`Mock PayJustNow server running on http://localhost:${PORT}`);
  console.log("Endpoints:");
  console.log("  POST /api/v1/checkouts - Create checkout");
  console.log("  GET  /api/v1/checkouts/:token - Get checkout status");
  console.log("  POST /api/v1/checkouts/:token/refund - Process refund");
  console.log("  GET  /health - Health check");
});

// Graceful shutdown
process.on("SIGINT", () => {
  console.log("\nShutting down mock server...");
  server.close(() => {
    console.log("Mock server stopped");
    process.exit(0);
  });
});
