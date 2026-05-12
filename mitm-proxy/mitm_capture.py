"""
Test-aware HTTP capture addon for mitmproxy.

Captures full request/response details (headers, body, timing, HTTP version,
URL components) and writes one JSON file per round-trip. Output is organised
as:

    {CAPTURE_DIR}/{connector}/{test_name}/{request_id}/{NNN}.json

Matching key on replay is `x-request-id` (propagated end-to-end by Cypress →
Hyperswitch → connector via the `trace_header.id_reuse_strategy="use_incoming"`
router config). The `test_name` folder is organisational only; the matcher
does not read it.

Configuration via environment variables:
    CAPTURE_BASE_URLS  Comma-separated URL prefixes to capture (optional).
                       If unset, captures every flow that reaches the proxy.
                       e.g. "https://api.stripe.com,https://checkout-test.adyen.com"
    CONNECTOR          Tag for the connector being tested (e.g. "cybersource").
                       Optional - if omitted, inferred from the URL host.
    CAPTURE_DIR        Output directory (default: <script_dir>/captures).
    ADMIN_PORT         Port for test-correlation admin server (default: 8081).

Cypress hooks:
    POST http://127.0.0.1:8081/test/start  body: {"test": "name"}
    POST http://127.0.0.1:8081/test/end

Run with:
    mitmdump -s mitm_capture.py --mode regular@8080
"""

import base64
import json
import os
import re
import threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse

from mitmproxy import http


# ───── config from env ─────
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.environ.get("CAPTURE_DIR") or os.path.join(SCRIPT_DIR, "captures")
BASE_URLS = [
    u.strip().rstrip("/")
    for u in os.environ.get("CAPTURE_BASE_URLS", "").split(",")
    if u.strip()
]
DEFAULT_CONNECTOR = os.environ.get("CONNECTOR", "")
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8081"))


# ───── shared state ─────
class State:
    current_test = None
    lock = threading.Lock()
    counter = {}


state = State()


# ───── admin HTTP server (for test correlation) ─────
class AdminHandler(BaseHTTPRequestHandler):
    def _send(self, code, body):
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(body).encode())

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length).decode() if length else "{}"
        try:
            body = json.loads(raw)
        except json.JSONDecodeError:
            body = {}

        with state.lock:
            if self.path == "/test/start":
                state.current_test = body.get("test", "unknown")
                state.counter[state.current_test] = 0
                print(f"[capture] ▶ START: {state.current_test}")
                self._send(200, {"ok": True, "test": state.current_test})
            elif self.path == "/test/end":
                print(f"[capture] ⏹ END:   {state.current_test}")
                state.current_test = None
                self._send(200, {"ok": True})
            elif self.path == "/status":
                self._send(200, {
                    "current_test": state.current_test,
                    "captured_per_test": state.counter,
                    "base_urls": BASE_URLS,
                    "output_dir": OUT_DIR,
                })
            else:
                self._send(404, {"error": "unknown admin path"})

    def log_message(self, fmt, *args):
        return  # silence default per-request logging


def start_admin_server():
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[capture] admin server  http://127.0.0.1:{ADMIN_PORT}")


# ───── helpers ─────
def matches(url: str) -> bool:
    if not BASE_URLS:
        return True
    return any(url.startswith(b) for b in BASE_URLS)


def connector_for(url: str, headers: dict | None = None) -> str:
    if DEFAULT_CONNECTOR:
        return DEFAULT_CONNECTOR
    # Hyperswitch adds x-connector to every outbound connector request
    if headers:
        from_header = headers.get("x-connector", "").strip()
        if from_header:
            return from_header
    host = urlparse(url).hostname or "unknown"
    if "stripe" in host:
        return "stripe"
    if "adyen" in host:
        return "adyen"
    if "paypal" in host:
        return "paypal"
    if "braintree" in host:
        return "braintree"
    if "cybersource" in host:
        return "cybersource"
    return host.replace(".", "_")


def safe_name(s: str) -> str:
    return re.sub(r"[^\w\-]+", "_", s)[:80]


def iso(ts):
    """Convert a unix-epoch float to an ISO-8601 UTC string."""
    if ts is None:
        return None
    return datetime.fromtimestamp(ts, tz=timezone.utc).isoformat()


def encode_body(content: bytes, content_type: str = ""):
    """
    Return a dict describing a body.
    - JSON content-type -> parsed JSON object (encoding: "json")
    - Valid UTF-8        -> plain string      (encoding: "utf-8")
    - Binary            -> base64 string      (encoding: "base64")
    """
    if content is None:
        return {"data": None, "encoding": None, "size_bytes": 0}
    size = len(content)
    if size == 0:
        return {"data": None, "encoding": None, "size_bytes": 0}

    is_json_type = "json" in content_type.lower()

    try:
        text = content.decode("utf-8")
    except UnicodeDecodeError:
        return {
            "data": base64.b64encode(content).decode("ascii"),
            "encoding": "base64",
            "size_bytes": size,
        }

    if is_json_type or text.lstrip().startswith(("{", "[")):
        try:
            return {"data": json.loads(text), "encoding": "json", "size_bytes": size}
        except json.JSONDecodeError:
            pass

    return {"data": text, "encoding": "utf-8", "size_bytes": size}


# ───── mitmproxy hook ─────
def response(flow: http.HTTPFlow):
    if not matches(flow.request.url):
        return

    connector = connector_for(flow.request.url, dict(flow.request.headers))
    request_id = flow.request.headers.get("x-request-id", "").strip()

    with state.lock:
        test = state.current_test or "_untagged"
        # Counter is scoped per (test, request_id) so multiple connector
        # outbounds sharing a request_id get NNN-suffixed in chronological order
        counter_key = (test, request_id or "_no_request_id")
        idx = state.counter.get(counter_key, 0)
        state.counter[counter_key] = idx + 1

    folder = os.path.join(
        OUT_DIR,
        connector,
        safe_name(test),
        safe_name(request_id) if request_id else "_no_request_id",
    )
    os.makedirs(folder, exist_ok=True)

    req = flow.request
    res = flow.response

    req_started = req.timestamp_start
    res_completed = res.timestamp_end
    duration_ms = (
        round((res_completed - req_started) * 1000, 2)
        if (req_started and res_completed)
        else None
    )

    req_body = encode_body(req.content, req.headers.get("content-type", ""))
    res_body = encode_body(res.content, res.headers.get("content-type", ""))

    record = {
        "captured_at": iso(req_started) or datetime.now(timezone.utc).isoformat(),
        "test": test,
        "request_id": request_id,
        "connector": connector,
        "timing": {
            "request_started_at": iso(req_started),
            "request_completed_at": iso(req.timestamp_end),
            "response_started_at": iso(res.timestamp_start),
            "response_completed_at": iso(res_completed),
            "duration_ms": duration_ms,
        },
        "request": {
            "http_version": req.http_version,
            "method": req.method,
            "url": req.url,
            "scheme": req.scheme,
            "host": req.host,
            "port": req.port,
            "path": req.path.split("?", 1)[0],
            "query": dict(req.query.items(multi=True)) if req.query else {},
            "headers": dict(req.headers),
            "body": req_body["data"],
            "body_encoding": req_body["encoding"],
            "body_size_bytes": req_body["size_bytes"],
        },
        "response": {
            "http_version": res.http_version,
            "status": res.status_code,
            "reason": res.reason,
            "headers": dict(res.headers),
            "body": res_body["data"],
            "body_encoding": res_body["encoding"],
            "body_size_bytes": res_body["size_bytes"],
        },
    }

    out_path = os.path.join(folder, f"{idx:03d}.json")
    with open(out_path, "w") as f:
        json.dump(record, f, indent=2)
    rid_tag = f"  rid={request_id}" if request_id else "  rid=(none)"
    print(
        f"[capture] {req.method} {req.url} "
        f"→ {res.status_code} {res.reason or ''} "
        f"({duration_ms}ms){rid_tag} → {out_path}"
    )


# ───── boot ─────
start_admin_server()
print(f"[capture] base URLs     {BASE_URLS or '(none — capturing all)'}")
print(f"[capture] output dir    {OUT_DIR}")
print(f"[capture] default tag   {DEFAULT_CONNECTOR or '(infer from host)'}")