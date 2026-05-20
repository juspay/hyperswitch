"""
mitmproxy replay addon.

Intercepts connector requests from Hyperswitch and answers them with the
recorded response identified by the propagated ``x-request-id`` header.
Method and path act as tie-breakers when one request_id covers multiple
outbound calls.

  Matching key : (request_id, method, path)
                 — connector-agnostic, so ancillary connector calls (vault,
                 fraud, …) made during a primary-connector test resolve
                 against cassettes filed under the primary connector's
                 folder. The ``x-connector`` header is still used as the
                 gate "is this a connector call I should try to replay?".
  Duplicates   : last file wins (later recording = successful Cypress retry)
  Reuse        : cassettes are peeked, not popped — repeated requests for
                 the same key (Cypress retries, parallel same-connector
                 shards) get the same recorded response.

Modes:
  permissive (default) : MISS / no-rid → request forwarded to live connector
  strict (REPLAY_STRICT=1) : MISS / no-rid → synthetic 599 response,
                             connector is never called.

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
STRICT_MODE = os.environ.get("REPLAY_STRICT", "").strip().lower() in (
    "1",
    "true",
    "yes",
    "on",
)

# Headers mitmproxy manages itself — don't replay them from the recording.
_RESERVED_HEADERS = frozenset({"content-length", "transfer-encoding", "connection"})

CassetteKey = tuple[str, str, str]  # (request_id, method, path)


# ───── cassette store ─────
class CassetteStore:
    """Loads cassettes from disk and dispenses them by request key."""

    def __init__(self, capture_dir: str):
        self._capture_dir = capture_dir
        self._queues: dict[CassetteKey, deque] = defaultdict(deque)
        self._connectors_seen: set[str] = set()
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
                request_id,
                record["request"]["method"],
                record["request"]["path"],
            )
            # Files share a key when Cypress retried a step.  Sorted glob
            # processes them lexically, so reassigning here keeps the last
            # (successful) recording and discards the earlier failed one.
            self._queues[key] = deque([record["response"]])
            recorded_connector = record.get("connector")
            if recorded_connector:
                self._connectors_seen.add(recorded_connector)

        total = sum(len(q) for q in self._queues.values())
        print(f"[replay] loaded {total} cassettes from {self._capture_dir}/")
        print(f"[replay] connectors (tag in cassette files): {sorted(self._connectors_seen)}")
        if skipped:
            print(f"[replay] skipped {skipped} cassettes with no request_id")

    def get(self, key: CassetteKey) -> dict | None:
        """Peek at the cassette for ``key`` without consuming it.

        Repeated calls for the same key return the same recording, so
        Cypress retries and same-connector parallel shards both work.
        """
        with self._lock:
            queue = self._queues.get(key)
            return queue[0] if queue else None


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


def _strict_block_response(reason: str, tag: str) -> http.Response:
    """Synthetic response returned in strict mode when no cassette matches."""
    body = json.dumps(
        {"error": "mitm replay strict mode: connector call blocked", "reason": reason}
    ).encode("utf-8")
    return http.Response.make(
        599,
        body,
        {"Content-Type": "application/json", "X-Cassette": tag},
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
print(f"[replay] mode {'STRICT (no live fallthrough)' if STRICT_MODE else 'permissive'}")


# ───── mitmproxy hook ─────
def request(flow: http.HTTPFlow) -> None:
    connector = flow.request.headers.get("x-connector", "").strip()
    if not connector:
        return  # not a Hyperswitch connector call — pass through

    request_id = flow.request.headers.get("x-request-id", "").strip()
    method = flow.request.method
    path = flow.request.path.split("?", 1)[0]

    if not request_id:
        if STRICT_MODE:
            flow.response = _strict_block_response(
                f"missing x-request-id for {method} {path}",
                tag="STRICT-no-request-id",
            )
            print(f"[replay] BLOCK no-rid [{connector}] {method} {path}")
            return
        print(f"[replay] LIVE no-rid [{connector}] {method} {path}")
        flow.request.headers["X-Cassette"] = "LIVE-no-request-id"
        return

    recorded = _store.get((request_id, method, path))
    if recorded is None:
        if STRICT_MODE:
            flow.response = _strict_block_response(
                f"no cassette for [{connector}] {method} {path} (rid={request_id})",
                tag="STRICT-no-cassette",
            )
            print(f"[replay] BLOCK miss  [{connector}] {method} {path} (rid={request_id})")
            return
        print(f"[replay] MISS       [{connector}] {method} {path} (rid={request_id})")
        flow.request.headers["X-Cassette"] = "LIVE-no-cassette"
        return

    flow.response = _build_response(recorded)
    print(
        f"[replay] HIT        [{connector}] {method} {path} "
        f"(rid={request_id}) → {recorded['status']}"
    )
