"""
mitmproxy replay addon.

Serves cassettes recorded by mitm_capture.py back to Hyperswitch on its
outbound connector calls. Match key is x-request-id alone; cassettes sharing a
rid are served in capture order via a per-rid cursor, reset on /test/start so
Cypress retries replay from the start.

Permissive (default): MISS / unknown-rid → live connector.
Strict (REPLAY_STRICT=1): MISS / unknown-rid → synthetic 599, never live.

Run: mitmdump -s mitm_replay.py --listen-port 8888
"""

import base64
import glob
import json
import os
import threading
from collections import defaultdict
from http.server import BaseHTTPRequestHandler, HTTPServer

from mitmproxy import http


# ───── config ─────
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CAPTURE_DIR = os.environ.get("CAPTURE_DIR") or os.path.join(SCRIPT_DIR, "captures")
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8001"))
STRICT_MODE = os.environ.get("REPLAY_STRICT", "").strip().lower() in (
    "1", "true", "yes", "on",
)

# Headers mitmproxy manages itself — don't replay them.
_RESERVED_HEADERS = frozenset({"content-length", "transfer-encoding", "connection"})


def hash_from_rid(rid: str) -> str:
    return rid.split("-", 1)[0] if "-" in rid else rid


# ───── cassette store ─────
class CassetteStore:
    """Cassettes indexed by rid → ordered responses, served via a per-rid
    cursor that resets on /test/start (clean Cypress retry replay)."""

    def __init__(self, capture_dir: str):
        self._capture_dir = capture_dir
        self._cassettes: dict[str, list[dict]] = defaultdict(list)
        self._cursor: dict[str, int] = {}
        self._lock = threading.Lock()

    def load(self) -> None:
        """Load all cassettes from disk into rid → [response, …]."""
        pattern = os.path.join(self._capture_dir, "**", "*.json")
        # Collect by rid, then sort each rid's list by the "sequence" field
        # (robust against legacy filenames without the seq suffix).
        by_rid: dict[str, list[tuple[int, dict]]] = defaultdict(list)
        skipped = 0
        for fpath in glob.glob(pattern, recursive=True):
            try:
                with open(fpath) as f:
                    record = json.load(f)
            except (OSError, json.JSONDecodeError):
                skipped += 1
                continue
            rid = (record.get("request_id") or "").strip()
            if not rid or "response" not in record:
                skipped += 1
                continue
            seq = record.get("sequence", 0)
            by_rid[rid].append((seq, record["response"]))

        for rid, entries in by_rid.items():
            entries.sort(key=lambda e: e[0])
            self._cassettes[rid] = [r for _, r in entries]

        total = sum(len(v) for v in self._cassettes.values())
        print(f"[replay] loaded {total} cassettes from {self._capture_dir}/")
        print(f"[replay] indexed {len(self._cassettes)} unique rids")
        if skipped:
            print(f"[replay] skipped {skipped} cassettes (malformed or no rid)")

    def serve(self, rid: str) -> dict | None:
        """Pop the next cassette for ``rid`` (FIFO via per-rid cursor)."""
        with self._lock:
            cassettes = self._cassettes.get(rid)
            if not cassettes:
                return None
            cursor = self._cursor.get(rid, 0)
            if cursor >= len(cassettes):
                return None
            self._cursor[rid] = cursor + 1
            return cassettes[cursor]

    def reset_for_hash(self, test_hash: str) -> int:
        """Rewind cursors for every rid whose prefix matches this hash."""
        prefix = f"{test_hash}-"
        with self._lock:
            keys = [r for r in self._cursor if r.startswith(prefix)]
            for k in keys:
                del self._cursor[k]
            return len(keys)


_store = CassetteStore(CAPTURE_DIR)


# ───── response materialisation ─────
def _decode_body(body, encoding):
    if encoding == "json":
        return json.dumps(body, separators=(",", ":")).encode("utf-8")
    if encoding == "base64":
        return base64.b64decode(body)
    return (body or "").encode("utf-8")


def _build_response(recorded: dict) -> http.Response:
    headers = {
        k: v for k, v in recorded.get("headers", {}).items()
        if k.lower() not in _RESERVED_HEADERS
    }
    headers["X-Cassette"] = "HIT"
    return http.Response.make(
        recorded["status"],
        _decode_body(recorded.get("body"), recorded.get("body_encoding")),
        headers,
    )


def _strict_block(reason: str, tag: str) -> http.Response:
    body = json.dumps({
        "error": "mitm replay strict mode: connector call blocked",
        "reason": reason,
    }).encode("utf-8")
    return http.Response.make(
        599, body, {"Content-Type": "application/json", "X-Cassette": tag},
    )


# ───── admin server ─────
class AdminHandler(BaseHTTPRequestHandler):
    """Cypress /test/start resets cursors for that test's rids; /test/end is a no-op."""

    def _send(self, code, payload):
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(payload).encode())

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        try:
            body = json.loads(self.rfile.read(length) or b"{}")
        except json.JSONDecodeError:
            body = {}

        if self.path == "/test/start":
            title = " > ".join(body.get("titlePath") or []) or "unknown"
            test_id_hash = (body.get("testIdHash") or "").strip()
            connector = (body.get("connector") or "").strip()
            tag = f"[{connector}] " if connector else ""
            reset = _store.reset_for_hash(test_id_hash) if test_id_hash else 0
            suffix = f" (reset {reset} rid cursors)" if reset else ""
            print(f"[replay] ▶ {tag}{title}{suffix}")

        self._send(200, {"ok": True})

    def log_message(self, *_a, **_kw):
        return


def _start_admin_server():
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[replay] admin server http://127.0.0.1:{ADMIN_PORT}")


# ───── boot ─────
_store.load()
_start_admin_server()
print(f"[replay] mode {'STRICT (no live fallthrough)' if STRICT_MODE else 'permissive'}")


# ───── mitmproxy hook ─────
def http_connect(flow: http.HTTPFlow) -> None:
    """Accept CONNECT tunnels locally so we can intercept the inner HTTPS;
    request() then decides HIT / MISS / LIVE."""
    flow.response = http.Response.make(200, b"", {})


def request(flow: http.HTTPFlow) -> None:
    # Gate: only handle Hyperswitch → connector outbounds (tagged x-connector).
    if not flow.request.headers.get("x-connector", "").strip():
        return

    rid = flow.request.headers.get("x-request-id", "").strip()
    method = flow.request.method
    path = flow.request.path.split("?", 1)[0]

    if not rid:
        if STRICT_MODE:
            flow.response = _strict_block(
                f"missing x-request-id for {method} {path}", "STRICT-no-rid",
            )
            print(f"[replay] BLOCK no-rid  {method} {path}")
        else:
            flow.request.headers["X-Cassette"] = "LIVE-no-rid"
            print(f"[replay] LIVE  no-rid  {method} {path}")
        return

    recorded = _store.serve(rid)
    if recorded is None:
        if STRICT_MODE:
            flow.response = _strict_block(
                f"no cassette for {method} {path} (rid={rid})", "STRICT-miss",
            )
            print(f"[replay] BLOCK miss    {method} {path} rid={rid}")
        else:
            flow.request.headers["X-Cassette"] = "LIVE-miss"
            print(f"[replay] MISS          {method} {path} rid={rid}")
        return

    flow.response = _build_response(recorded)
    print(f"[replay] HIT           {method} {path} rid={rid} → {recorded['status']}")
