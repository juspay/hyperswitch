"""
mitmproxy replay addon.

Intercepts connector requests from Hyperswitch and returns the recorded
cassette response for the propagated `x-request-id` instead of forwarding
to the real connector.

The admin server is still alive for capture-mode compatibility and useful
logging, but replay does not depend on the active test — matching is based
on `x-request-id` alone (plus method+path as a tie-breaker for the rare
case of multiple connector outbounds sharing one request_id):

  Matching key: (connector, request_id, method, path)
  Duplicates:   last-file-wins (later recording = successful Cypress retry)

Run with:
  mitmdump -s mitm_replay.py --listen-port 8888
"""

import base64
import glob
import json
import os
import re
import threading
from collections import defaultdict, deque
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse

from mitmproxy import http

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CAPTURE_DIR = os.environ.get("CAPTURE_DIR") or os.path.join(SCRIPT_DIR, "captures")
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8001"))

# Headers mitmproxy manages itself — don't copy from recording
_SKIP_HEADERS = {"content-length", "transfer-encoding", "connection"}


# ───── shared state ─────
class State:
    current_test = None
    lock = threading.Lock()


state = State()

# (connector, request_id, method, path) -> deque[response_dict]
_cassettes: dict[tuple, deque] = defaultdict(deque)
# Browser/connector redirect callbacks reach HS without Cypress's X-Request-ID
# in capture mode, so HS stamps a server UUID. Replay synthesizes the same HS
# callback from Cypress; when exact rid matching misses, this test-scoped
# fallback lets those server-rid cassettes replay without manual relocation.
_server_rid_cassettes: dict[tuple, deque] = defaultdict(deque)

CYPRESS_RID_RE = re.compile(r"^[0-9a-f]{8}-\d{3}$")


def _infer_connector_from_host(host: str) -> str:
    host = (host or "").strip().lower()
    if not host:
        return "unknown"
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


def _cassette_connector(record: dict) -> str:
    request = record.get("request") or {}
    headers = {
        str(k).lower(): v
        for k, v in (request.get("headers") or {}).items()
    }
    from_header = str(headers.get("x-connector", "")).strip()
    if from_header:
        return from_header

    host = request.get("host")
    if not host:
        host = urlparse(request.get("url", "")).hostname or ""
    inferred = _infer_connector_from_host(host)
    if inferred != "unknown":
        return inferred

    return record.get("connector", "unknown")


def _load_cassettes():
    pattern = os.path.join(CAPTURE_DIR, "**", "*.json")
    files = sorted(glob.glob(pattern, recursive=True))  # sort keeps 000 < 001

    skipped = 0
    for fpath in files:
        with open(fpath) as f:
            record = json.load(f)

        request_id = record.get("request_id", "").strip()
        if not request_id:
            skipped += 1
            continue

        connector = _cassette_connector(record)
        method = record["request"]["method"]
        path = record["request"]["path"]
        response = record["response"]

        key = (connector, request_id, method, path)
        # When the same (connector, rid, method, path) appears in multiple files
        # (e.g. a Cypress test retried, producing two recordings of the same
        # step), files are processed in ascending numeric order so the last file
        # wins.  That last file is always from the successful run whose
        # downstream cassettes (PSync, Capture, …) share its PI/resource ID.
        # Using a fresh deque here discards the stale first-attempt entry.
        _cassettes[key] = deque([response])

        if request_id and not CYPRESS_RID_RE.match(request_id):
            test = record.get("test", "")
            _server_rid_cassettes[(connector, test, method, path)].append(response)

    total = sum(len(v) for v in _cassettes.values())
    connectors = sorted({k[0] for k in _cassettes})
    print(f"[replay] Loaded {total} cassettes from {CAPTURE_DIR}/")
    print(f"[replay] Connectors : {connectors}")
    if skipped:
        print(f"[replay] Skipped {skipped} cassettes with no request_id "
              f"(re-record with use_incoming trace_header and Cypress wrapper)")


# ───── admin server ─────
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
                if title_path:
                    state.current_test = " > ".join(title_path)
                else:
                    state.current_test = body.get("test", "unknown")
                print(f"[replay] ▶ {state.current_test}")
                self._send(200, {"ok": True})
            elif self.path == "/test/end":
                state.current_test = None
                self._send(200, {"ok": True})
            else:
                self._send(404, {"error": "unknown path"})

    def log_message(self, fmt, *args):
        return  # silence per-request logs


def _start_admin_server():
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[replay] admin server  http://127.0.0.1:{ADMIN_PORT}")


# ───── mitmproxy hook ─────
def request(flow: http.HTTPFlow):
    connector = flow.request.headers.get("x-connector", "").strip()
    if not connector:
        return  # not a connector call — pass through

    request_id = flow.request.headers.get("x-request-id", "").strip()
    method = flow.request.method
    path = flow.request.path.split("?", 1)[0]

    if not request_id:
        print(f"[replay] WARN  no x-request-id  [{connector}] {method} {path} — going LIVE")
        flow.request.headers["X-Cassette"] = "LIVE-no-request-id"
        return

    key = (connector, request_id, method, path)

    matched_server_rid = False
    with state.lock:
        queue = _cassettes.get(key)
        recorded = queue.popleft() if queue else None
        if recorded is None:
            cb_key = (connector, state.current_test or "", method, path)
            cb_queue = _server_rid_cassettes.get(cb_key)
            recorded = cb_queue.popleft() if cb_queue else None
            matched_server_rid = recorded is not None

    if recorded is None:
        print(f"[replay] MISS  [{connector}] {method} {path} (rid={request_id}) — going LIVE")
        flow.request.headers["X-Cassette"] = "LIVE-no-cassette"
        return

    # Reconstruct response body
    body = recorded.get("body")
    encoding = recorded.get("body_encoding")

    if encoding == "json":
        body_bytes = json.dumps(body, separators=(",", ":")).encode("utf-8")
    elif encoding == "base64":
        body_bytes = base64.b64decode(body)
    else:
        body_bytes = (body or "").encode("utf-8")

    # Copy headers, skip ones mitmproxy controls
    headers = {
        k: v
        for k, v in recorded.get("headers", {}).items()
        if k.lower() not in _SKIP_HEADERS
    }
    headers["X-Cassette"] = "HIT"

    flow.response = http.Response.make(
        recorded["status"],
        body_bytes,
        headers,
    )

    hit_kind = "HIT-server" if matched_server_rid else "HIT"
    print(f"[replay] {hit_kind:<10} [{connector}] {method} {path} (rid={request_id}) → {recorded['status']}")


_load_cassettes()
_start_admin_server()
