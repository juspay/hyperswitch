PRAGMA journal_mode=WAL;
PRAGMA synchronous=FULL;
PRAGMA busy_timeout=5000;
CREATE TABLE IF NOT EXISTS runs (
  run_id TEXT PRIMARY KEY, recording_state TEXT NOT NULL DEFAULT 'incomplete',
  started_at INTEGER NOT NULL, ended_at INTEGER, enqueued INTEGER NOT NULL DEFAULT 0,
  persisted INTEGER NOT NULL DEFAULT 0, error TEXT, metadata TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS request_events (
  event_id INTEGER PRIMARY KEY, run_id TEXT NOT NULL, attempt_id INTEGER NOT NULL, flow_id TEXT NOT NULL,
  merchant_id TEXT, operation TEXT NOT NULL, role TEXT NOT NULL,
  observed_at INTEGER NOT NULL, method TEXT NOT NULL, url TEXT NOT NULL,
  customer_id TEXT, merchant_reference_id TEXT, payment_id TEXT, amount INTEGER,
  status TEXT, api_status_code INTEGER NOT NULL, request_id TEXT,
  latency_ms REAL NOT NULL, message TEXT, error_body TEXT, entity_type TEXT
);
CREATE TABLE IF NOT EXISTS customers (
  run_id TEXT NOT NULL, customer_id TEXT NOT NULL, merchant_id TEXT,
  merchant_reference_id TEXT, api_status_code INTEGER NOT NULL, request_id TEXT,
  message TEXT, observed_at INTEGER NOT NULL, response_event_id INTEGER NOT NULL,
  PRIMARY KEY (run_id, customer_id)
);
CREATE TABLE IF NOT EXISTS payments (
  run_id TEXT NOT NULL, payment_id TEXT NOT NULL, merchant_id TEXT, customer_id TEXT,
  amount INTEGER, status TEXT, role TEXT NOT NULL, api_status_code INTEGER NOT NULL,
  request_id TEXT, message TEXT, observed_at INTEGER NOT NULL, response_event_id INTEGER NOT NULL,
  PRIMARY KEY (run_id, payment_id)
);
