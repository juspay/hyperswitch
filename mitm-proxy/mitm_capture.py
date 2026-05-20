"""
Test-aware HTTP capture addon for mitmproxy.

Captures full request/response details (headers, body, timing, HTTP version,
URL components) and writes one JSON file per round-trip. Output is organised
as:

    {CAPTURE_DIR}/{connector}/{spec}/{ctx1}/{ctx2}/.../{NNN}.json

Where:
  - spec   = Cypress.spec.name with .cy.js/.cy.ts extension stripped
  - ctx1…  = each context() nesting level (titlePath[1:-1])
  - NNN    = zero-padded sequential index within that test

The matching key on replay is `x-request-id` stored inside each JSON file.
The folder layout is for human navigation only.

Configuration via environment variables:
    CAPTURE_BASE_URLS  Comma-separated URL prefixes to capture (optional).
                       If unset, captures every flow that reaches the proxy.
                       e.g. "https://api.stripe.com,https://checkout-test.adyen.com"
    CONNECTOR          Tag for the connector being tested (e.g. "cybersource").
                       Optional - if omitted, inferred from the URL host.
    CAPTURE_DIR        Output directory (default: <script_dir>/captures).
    ADMIN_PORT         Port for test-correlation admin server (default: 8001).

Cypress hooks:
    POST http://127.0.0.1:8001/test/start  body: {"titlePath": [...], "spec": "..."}
    POST http://127.0.0.1:8001/test/end

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
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8001"))

_SPEC_EXTENSIONS = (".cy.js", ".cy.ts", ".spec.js", ".spec.ts")


# ───── shared state ─────
class State:
    current_test = None        # titlePath.join(" > ") — used as cassette "test" field
    current_title_path = []    # raw titlePath array — used to build folder path
    current_spec = None        # Cypress.spec.name (with extension)
    current_connector = ""     # CYPRESS_CONNECTOR — primary connector under test;
                               # tags every flow during this test, including calls
                               # to ancillary connectors (vault, fraud, etc.)
    lock = threading.Lock()
    counter = {}               # rel_path -> int


state = State()


# ───── helpers ─────
def matches(url: str) -> bool:
    if not BASE_URLS:
        return True
    return any(url.startswith(b) for b in BASE_URLS)


def connector_for(
    url: str,
    headers: dict | None = None,
    test_connector: str = "",
) -> str:
    """Pick the connector tag for a captured flow.

    Priority:
      1. ``test_connector`` — primary connector under test, sent by Cypress
         on /test/start. Ensures ancillary connector calls (e.g. an external
         vault hit during a Stripe ExternalVault test) are bundled with the
         primary connector's cassettes.
      2. ``CONNECTOR`` env var (DEFAULT_CONNECTOR) — manual override.
      3. ``x-connector`` header set by the router on the outbound flow.
      4. Host inference as a last resort.
    """
    if test_connector:
        return test_connector
    if DEFAULT_CONNECTOR:
        return DEFAULT_CONNECTOR
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
    return re.sub(r"[^\w\-]+", "_", s).strip("_")


def safe_spec(name: str) -> str:
    # Strip file extension
    for ext in _SPEC_EXTENSIONS:
        if name.endswith(ext):
            name = name[: -len(ext)]
            break
    # Strip the e2e spec root prefix so "cypress/e2e/spec/Platform/Foo" → "Platform/Foo"
    name = re.sub(r"^.*?[/\\]spec[/\\]", "", name)
    # Split on path separators, sanitise each part, rejoin as OS path
    parts = [safe_name(p) for p in re.split(r"[/\\]", name) if p]
    return os.path.join(*parts) if parts else "_untagged"


def build_rel_path(spec: str, title_path: list) -> str:
    """
    Map spec + titlePath to a connector-relative folder path.

      spec            → top-level folder (file path stripped to category/name)
      titlePath[0]    → skip when spec is a real path; used AS spec when Cypress
                        returns "__all" (run-all-specs mode)
      titlePath[1:-1] → one safe_name folder per context level
      titlePath[-1]   → skip (it() — just lists the steps)

    Edge case: no contexts (describe > it only) → use safe_name(it title) as folder.
    Edge case: empty title_path → "_untagged"
    """
    # Cypress returns "__all" for Cypress.spec.relative when running all specs
    # at once (experimentalRunAllSpecs / GUI run-all). Fall back to the describe
    # title (titlePath[0]) which is unique per spec file.
    if not spec or spec == "__all":
        spec_part = safe_name(title_path[0]) if title_path else "_untagged"
    else:
        spec_part = safe_spec(spec)

    if not title_path:
        return os.path.join(spec_part, "_untagged")

    contexts = title_path[1:-1] if len(title_path) > 2 else []
    if not contexts:
        # no context block — use the it() title as the leaf folder
        fallback = safe_name(title_path[-1]) if title_path else "_untagged"
        return os.path.join(spec_part, fallback)

    return os.path.join(spec_part, *[safe_name(c) for c in contexts])


def iso(ts):
    if ts is None:
        return None
    return datetime.fromtimestamp(ts, tz=timezone.utc).isoformat()


def encode_body(content: bytes, content_type: str = ""):
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


# ───── admin HTTP server ─────
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
                title_path = body.get("titlePath") or []
                spec = body.get("spec") or "_untagged"
                test_connector = (body.get("connector") or "").strip()
                # Derive flat test name for cassette "test" field and _server_rid matching
                state.current_test = " > ".join(title_path) if title_path else body.get("test", "unknown")
                state.current_title_path = title_path
                state.current_spec = spec
                state.current_connector = test_connector
                tag = f"[{test_connector}] " if test_connector else ""
                print(f"[capture] ▶ START: {tag}[{safe_spec(spec)}] {state.current_test}")
                self._send(200, {"ok": True, "test": state.current_test, "connector": test_connector})
            elif self.path == "/test/end":
                print(f"[capture] ⏹ END:   {state.current_test}")
                state.current_test = None
                state.current_title_path = []
                state.current_spec = None
                state.current_connector = ""
                self._send(200, {"ok": True})
            elif self.path == "/status":
                self._send(200, {
                    "current_test": state.current_test,
                    "current_spec": state.current_spec,
                    "captured_per_path": state.counter,
                    "base_urls": BASE_URLS,
                    "output_dir": OUT_DIR,
                })
            else:
                self._send(404, {"error": "unknown admin path"})

    def log_message(self, fmt, *args):
        return


def start_admin_server():
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[capture] admin server  http://127.0.0.1:{ADMIN_PORT}")


# ───── mitmproxy hook ─────
def response(flow: http.HTTPFlow):
    if not matches(flow.request.url):
        return

    request_id = flow.request.headers.get("x-request-id", "").strip()

    with state.lock:
        test = state.current_test or "_untagged"
        spec = state.current_spec or "_untagged"
        title_path = list(state.current_title_path)
        test_connector = state.current_connector

    connector = connector_for(
        flow.request.url, dict(flow.request.headers), test_connector
    )

    with state.lock:
        rel_path = build_rel_path(spec, title_path)
        counter_key = (connector, rel_path)
        idx = state.counter.get(counter_key, 0)
        state.counter[counter_key] = idx + 1

    folder = os.path.join(OUT_DIR, connector, rel_path)
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
        "spec": spec,
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

    # Collision-safe write: if another concurrent response already claimed idx, advance
    with state.lock:
        while True:
            out_path = os.path.join(folder, f"{idx:03d}.json")
            if not os.path.exists(out_path):
                state.counter[counter_key] = max(state.counter.get(counter_key, 0), idx + 1)
                break
            idx += 1

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
