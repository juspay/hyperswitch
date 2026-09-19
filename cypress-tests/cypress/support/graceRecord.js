// Per-command execution record, enabled only when GRACE_RECORD is set.
//
// A connector config entry with TRIGGER_SKIP makes the command log and return,
// and the `it` block still passes, so the mochawesome report cannot tell a
// skipped flow from an executed one. This appends one JSON line per API call
// (and per TRIGGER_SKIP) to the file named by GRACE_RECORD:
//   {flow, connector, trigger_skip, request_id, http_status, execution_path}
// execution_path is not exposed by the API; it is resolved from the router log
// by request_id.
const GRACE_RECORD = Cypress.env("GRACE_RECORD");

// Map a Hyperswitch API call to the router flow it drives.
function flowFor(method, url) {
  const path = String(url || "")
    .replace(/^https?:\/\/[^/]+/, "")
    .split("?")[0];
  const m = String(method || "GET").toUpperCase();
  if (/^\/payments\/[^/]+\/capture$/.test(path)) return "Capture";
  if (/^\/payments\/[^/]+\/cancel$/.test(path)) return "Void";
  if (/^\/payments\/[^/]+\/confirm$/.test(path)) return "Authorize";
  if (m === "POST" && path === "/payments") return "Authorize";
  if (m === "GET" && /^\/payments\/[^/]+$/.test(path)) return "PSync";
  if (m === "POST" && path === "/refunds") return "Execute";
  if (m === "GET" && /^\/refunds\/[^/]+$/.test(path)) return "RSync";
  if (/^\/webhooks\//.test(path)) return "IncomingWebhook";
  return null;
}

// Records are buffered and flushed after each test: a cy.task issued from
// inside the overwritten request would nest cy commands in a command.
const pending = [];

if (GRACE_RECORD) {
  Cypress.Commands.overwrite("request", (originalFn, ...args) => {
    const opts =
      args.length === 1 && typeof args[0] === "object"
        ? args[0]
        : { method: args.length > 1 ? args[0] : "GET", url: args.at(-1) };
    const flow = flowFor(opts.method, opts.url);
    return originalFn(...args).then((response) => {
      if (flow) {
        pending.push({
          flow,
          connector: Cypress.env("CONNECTOR") || null,
          trigger_skip: false,
          request_id: response?.headers?.["x-request-id"] || null,
          http_status: response?.status ?? null,
          execution_path: null,
          spec: Cypress.spec.relative,
          test: Cypress.currentTest?.titlePath?.join(" > ") || null,
        });
      }
      return response;
    });
  });

  afterEach(() => {
    const records = pending.splice(0, pending.length);
    records.forEach((record) =>
      cy.task("grace_record", record, { log: false })
    );
  });
}
