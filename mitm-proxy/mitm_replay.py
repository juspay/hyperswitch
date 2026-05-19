"""
mitmproxy replay addon.

Intercepts connector requests from Hyperswitch and answers them with the
recorded response identified by the propagated ``x-request-id`` header.
Method and path act as tie-breakers when one request_id covers multiple
outbound calls.

  Matching key : (connector, request_id, method, path)
  Duplicates   : last file wins (later recording = successful Cypress retry)

Run with:
    mitmdump -s mitm_replay.py --listen-port 8888
"""

import base64
import glob
import json
import os
import threading
from collections import defaultdict, deque
from http.server import BaseHTTPRequestHandler, HTTPServer

from mitmproxy import http

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CAPTURE_DIR = os.environ.get("CAPTURE_DIR") or os.path.join(SCRIPT_DIR, "captures")
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8001"))

# Headers mitmproxy manages itself — don't replay them from the recording.
_RESERVED_HEADERS = frozenset({"content-length", "transfer-encoding", "connection"})

CassetteKey = tuple[str, str, str, str]  # (connector, request_id, method, path)


# ───── cassette store ─────
class CassetteStore:
    """Loads cassettes from disk and dispenses them by request key."""

    def __init__(self, capture_dir: str):
        self._capture_dir = capture_dir
        self._queues: dict[CassetteKey, deque] = defaultdict(deque)
        self._lock = threading.Lock()

    def load(self) -> None:
        pattern = os.path.join(self._capture_dir, "**", "*.json")
        skipped = 0

        for fpath in sorted(glob.glob(pattern, recursive=True)):
            with open(fpath) as f:
                record = json.load(f)

            request_id = (record.get("request_id") or "").strip()
            if not request_id:
                skipped += 1
                continue

            key: CassetteKey = (
                record["connector"],
                request_id,
                record["request"]["method"],
                record["request"]["path"],
            )
            # Files share a key when Cypress retried a step.  Sorted glob
            # processes them lexically, so reassigning here keeps the last
            # (successful) recording and discards the earlier failed one.
            self._queues[key] = deque([record["response"]])

        total = sum(len(q) for q in self._queues.values())
        connectors = sorted({k[0] for k in self._queues})
        print(f"[replay] loaded {total} cassettes from {self._capture_dir}/")
        print(f"[replay] connectors: {connectors}")
        if skipped:
            print(f"[replay] skipped {skipped} cassettes with no request_id")

    def pop(self, key: CassetteKey) -> dict | None:
        with self._lock:
            queue = self._queues.get(key)
            return queue.popleft() if queue else None


# ───── response materialisation ─────
def _decode_body(body, encoding: str | None) -> bytes:
    if encoding == "json":
        return json.dumps(body, separators=(",", ":")).encode("utf-8")
    if encoding == "base64":
        return base64.b64decode(body)
    return (body or "").encode("utf-8")


def _build_response(recorded: dict) -> http.Response:
    headers = {
        k: v
        for k, v in recorded.get("headers", {}).items()
        if k.lower() not in _RESERVED_HEADERS
    }
    headers["X-Cassette"] = "HIT"
    return http.Response.make(
        recorded["status"],
        _decode_body(recorded.get("body"), recorded.get("body_encoding")),
        headers,
    )


# ───── admin server (logging-only) ─────
class AdminHandler(BaseHTTPRequestHandler):
    """Acknowledges Cypress test-lifecycle pings so the same hook script
    works in capture and replay mode.  Replay matches on request_id alone,
    so this server is purely for log breadcrumbs."""

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        try:
            body = json.loads(self.rfile.read(length) or b"{}")
        except json.JSONDecodeError:
            body = {}

        if self.path == "/test/start":
            title = " > ".join(body.get("titlePath") or []) or body.get("test", "unknown")
            print(f"[replay] ▶ {title}")

        self._send(200, {"ok": True})

    def _send(self, code: int, payload: dict) -> None:
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(payload).encode())

    def log_message(self, *_args, **_kwargs):
        return  # silence per-request access logs


def _start_admin_server() -> None:
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[replay] admin server http://127.0.0.1:{ADMIN_PORT}")


# ───── boot ─────
_store = CassetteStore(CAPTURE_DIR)
_store.load()
_start_admin_server()


# ───── mitmproxy hook ─────
def request(flow: http.HTTPFlow) -> None:
    connector = flow.request.headers.get("x-connector", "").strip()
    if not connector:
        return  # not a Hyperswitch connector call — pass through

    request_id = flow.request.headers.get("x-request-id", "").strip()
    method = flow.request.method
    path = flow.request.path.split("?", 1)[0]

    if not request_id:
        print(f"[replay] LIVE no-rid [{connector}] {method} {path}")
        flow.request.headers["X-Cassette"] = "LIVE-no-request-id"
        return

    recorded = _store.pop((connector, request_id, method, path))
    if recorded is None:
        print(f"[replay] MISS       [{connector}] {method} {path} (rid={request_id})")
        flow.request.headers["X-Cassette"] = "LIVE-no-cassette"
        return

    flow.response = _build_response(recorded)
    print(
        f"[replay] HIT        [{connector}] {method} {path} "
        f"(rid={request_id}) → {recorded['status']}"
    )
