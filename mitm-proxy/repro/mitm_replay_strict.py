"""
mitmproxy replay addon — STRICT mode for CI/repro runs.

On cassette MISS or missing x-request-id it returns HTTP 599 instead of
forwarding to the live connector — no silent LIVE fallback, so replay
numbers are honest.

  Matching key: (connector, request_id, method, path)
  Tie-break:    FIFO within that key
"""

import base64
import glob
import json
import os
import re
import sys
import threading
from collections import defaultdict, deque
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from mitmproxy import http

from secret_redaction import creds_path, has_unresolved_placeholders, hydrate_record, redact_obj

_PATH_ID_PATTERNS = [
    re.compile(r"/pay_[A-Za-z0-9]{16,}(_[0-9]+)?"),
    re.compile(r"/ref_[A-Za-z0-9]{16,}"),
    re.compile(r"/att_[A-Za-z0-9]{16,}"),
]


def _norm_path(path: str) -> str:
    out = path
    for pat in _PATH_ID_PATTERNS:
        out = pat.sub("/_HS_ID_", out)
    return out


SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CAPTURE_DIR = os.environ.get("CAPTURE_DIR") or os.path.join(
    os.path.dirname(SCRIPT_DIR), "captures"
)
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8081"))

_SKIP_HEADERS = {"content-length", "transfer-encoding", "connection"}


class State:
    current_test = None
    lock = threading.Lock()


state = State()

_cassettes: dict[tuple, deque] = defaultdict(deque)
_cassettes_norm: dict[tuple, deque] = defaultdict(deque)
# Browser/connector redirect callbacks reach HS without Cypress's X-Request-ID
# in capture mode, so HS stamps a server UUID. Replay deliberately synthesizes
# the same HS callback from Cypress; the post-callback connector calls therefore
# cannot exact-match on rid. Keep a strict fallback index scoped by active test
# + connector + method + path for those server-rid cassettes.
_server_rid_cassettes: dict[tuple, deque] = defaultdict(deque)
_server_rid_cassettes_norm: dict[tuple, deque] = defaultdict(deque)
_last_get: dict[tuple, dict] = {}

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
    files = sorted(glob.glob(pattern, recursive=True))

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
        _cassettes[key].append(response)
        norm_key = (connector, request_id, method, _norm_path(path))
        if norm_key != key:
            _cassettes_norm[norm_key].append(response)

        if request_id and not CYPRESS_RID_RE.match(request_id):
            test = record.get("test", "")
            cb_key = (connector, test, method, path)
            _server_rid_cassettes[cb_key].append(response)
            cb_norm_key = (connector, test, method, _norm_path(path))
            if cb_norm_key != cb_key:
                _server_rid_cassettes_norm[cb_norm_key].append(response)

    total = sum(len(v) for v in _cassettes.values())
    connectors = sorted({k[0] for k in _cassettes})
    print(f"[replay] Loaded {total} cassettes from {CAPTURE_DIR}/")
    print(f"[replay] Connectors : {connectors}")
    if skipped:
        print(f"[replay] Skipped {skipped} cassettes with no request_id")


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
                print(f"[replay] ▶ {state.current_test}")
                self._send(200, {"ok": True})
            elif self.path == "/test/end":
                state.current_test = None
                self._send(200, {"ok": True})
            else:
                self._send(404, {"error": "unknown path"})

    def log_message(self, fmt, *args):
        return


def _start_admin_server():
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[replay] admin server  http://127.0.0.1:{ADMIN_PORT}")


def request(flow: http.HTTPFlow):
    connector = flow.request.headers.get("x-connector", "").strip()
    if not connector:
        return

    request_id = flow.request.headers.get("x-request-id", "").strip()
    method = flow.request.method
    path = flow.request.path.split("?", 1)[0]
    match_path, _ = redact_obj(path)
    norm_match_path = _norm_path(match_path)

    if not request_id:
        print(f"[replay] WARN  no x-request-id  [{connector}] {method} {match_path} — failing offline")
        flow.response = http.Response.make(
            599,
            json.dumps({
                "error": "mitm-replay: no x-request-id on connector call",
                "connector": connector, "method": method, "path": match_path,
            }).encode("utf-8"),
            {"Content-Type": "application/json", "X-Cassette": "FAIL-no-request-id"},
        )
        return

    key = (connector, request_id, method, match_path)
    norm_key = (connector, request_id, method, norm_match_path)
    matched_norm = False
    matched_replay = False

    with state.lock:
        queue = _cassettes.get(key)
        recorded = queue.popleft() if queue else None
        if recorded is None and norm_key != key:
            nqueue = _cassettes_norm.get(norm_key)
            recorded = nqueue.popleft() if nqueue else None
            if recorded is not None:
                matched_norm = True
        if recorded is None:
            current_test = state.current_test or ""
            cb_key = (connector, current_test, method, match_path)
            cb_queue = _server_rid_cassettes.get(cb_key)
            recorded = cb_queue.popleft() if cb_queue else None
            if recorded is None:
                cb_norm_key = (connector, current_test, method, norm_match_path)
                cb_norm_queue = _server_rid_cassettes_norm.get(cb_norm_key)
                recorded = cb_norm_queue.popleft() if cb_norm_queue else None
            if recorded is not None:
                matched_replay = "server-rid"
        if recorded is None and method == "GET":
            sticky_key = (connector, method, match_path)
            sticky_norm = (connector, method, norm_match_path)
            sticky = _last_get.get(sticky_key) or _last_get.get(sticky_norm)
            if sticky is not None:
                recorded = sticky
                matched_replay = "sticky-get"

    if recorded is not None and method == "GET":
        with state.lock:
            _last_get[(connector, method, match_path)] = recorded
            _last_get[(connector, method, norm_match_path)] = recorded

    if recorded is None:
        print(f"[replay] MISS  [{connector}] {method} {match_path} (rid={request_id}) — failing offline")
        flow.response = http.Response.make(
            599,
            json.dumps({
                "error": "mitm-replay: no cassette",
                "connector": connector, "request_id": request_id,
                "method": method, "path": match_path,
            }).encode("utf-8"),
            {"Content-Type": "application/json", "X-Cassette": "FAIL-no-cassette"},
        )
        return

    recorded, hydrated_count = hydrate_record(recorded)
    if has_unresolved_placeholders(recorded):
        print(f"[replay] SECRET-MISS [{connector}] {method} {match_path} (rid={request_id}) — missing creds for cassette placeholders")
        flow.response = http.Response.make(
            599,
            json.dumps({
                "error": "mitm-replay: cassette contains unresolved credential placeholders",
                "connector": connector,
                "request_id": request_id,
                "method": method,
                "path": match_path,
            }).encode("utf-8"),
            {"Content-Type": "application/json", "X-Cassette": "FAIL-missing-creds"},
        )
        return

    body = recorded.get("body")
    encoding = recorded.get("body_encoding")

    if encoding == "json":
        body_bytes = json.dumps(body, separators=(",", ":")).encode("utf-8")
    elif encoding == "base64":
        body_bytes = base64.b64decode(body)
    else:
        body_bytes = (body or "").encode("utf-8")

    headers = {
        k: v for k, v in recorded.get("headers", {}).items()
        if k.lower() not in _SKIP_HEADERS
    }
    headers["X-Cassette"] = "HIT"

    flow.response = http.Response.make(recorded["status"], body_bytes, headers)

    if matched_replay == "server-rid":
        hit_kind = "HIT-server"
    elif matched_replay == "sticky-get":
        hit_kind = "HIT-replay"
    elif matched_norm:
        hit_kind = "HIT-norm"
    else:
        hit_kind = "HIT"
    secret_tag = f" hydrated={hydrated_count}" if hydrated_count else ""
    print(f"[replay] {hit_kind:<8} [{connector}] {method} {match_path} (rid={request_id}) → {recorded['status']}{secret_tag}")


print(f"[replay] creds file    {creds_path()} ({'present' if creds_path().exists() else 'missing; placeholders will fail'})")
_load_cassettes()
_start_admin_server()
