"""
mitmproxy capture addon.

Records all Hyperswitch outbound HTTP into JSON cassettes grouped by the
Cypress test that issued the call.

Grouping is by ``testIdHash`` — a stable hash of
``(CYPRESS_CONNECTOR, titlePath)`` sent by Cypress on /test/start. Every
outbound during the test carries ``X-Request-ID: {testIdHash}-{NNN}``
(Hyperswitch propagates the incoming id to its outbound calls with
``id_reuse_strategy = use_incoming``), so we can attribute calls back to
the test that issued them — even when they land asynchronously long
after /test/end (e.g. External Vault save-card vault writes that defer).

  Filename : {OUT_DIR}/{cypress_connector}/{rel_path}/{rid}-{seq:02d}.json
  Match key on replay : rid alone (handled by mitm_replay.py)
  Fan-out  : multiple outbounds sharing one rid → seq=00, 01, 02 in
             capture order; replay serves in the same order.
  Retries  : on /test/start, prior cassettes for that testHash in the
             same folder are wiped so a Cypress retry produces a clean
             set rather than appending alongside the failed run.

Skipped:
  - No X-Request-ID (cannot match anything on replay).
  - Rid not in {testHash}-{NNN} format (Hyperswitch background activity:
    PSync schedulers, webhook handlers — not Cypress-driven, not
    replayable since the UUID changes every run).
  - testHash never registered via /test/start (orphan).
"""

import base64
import glob
import json
import os
import re
import threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, HTTPServer

from mitmproxy import http


# ───── config ─────
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.environ.get("CAPTURE_DIR") or os.path.join(SCRIPT_DIR, "captures")
BASE_URLS = [
    u.strip().rstrip("/")
    for u in os.environ.get("CAPTURE_BASE_URLS", "").split(",")
    if u.strip()
]
ADMIN_PORT = int(os.environ.get("ADMIN_PORT", "8001"))

_SPEC_EXTENSIONS = (".cy.js", ".cy.ts", ".spec.js", ".spec.ts")
# Cypress rid format: 8 hex chars (djb2 hash) + "-" + 3 digits (stepCounter).
_CYPRESS_RID = re.compile(r"^[0-9a-fA-F]{8}-\d{3}$")


# ───── state ─────
class TestContext:
    __slots__ = ("test", "spec", "connector", "folder")

    def __init__(self, test: str, spec: str, connector: str, folder: str):
        self.test = test
        self.spec = spec
        self.connector = connector
        self.folder = folder


_lock = threading.Lock()
_tests_by_hash: dict[str, TestContext] = {}
# Per-rid sequence counter for the *current* run of a test. Cleared on
# /test/start (so a retry starts at seq=00 again, overwriting cleanly).
_seq_by_rid: dict[str, int] = {}


# ───── helpers ─────
def safe_name(s: str) -> str:
    return re.sub(r"[^\w\-]+", "_", s).strip("_") or "_untagged"


def safe_spec(name: str) -> str:
    for ext in _SPEC_EXTENSIONS:
        if name.endswith(ext):
            name = name[: -len(ext)]
            break
    name = re.sub(r"^.*?[/\\]spec[/\\]", "", name)
    parts = [safe_name(p) for p in re.split(r"[/\\]", name) if p]
    return os.path.join(*parts) if parts else "_untagged"


def build_rel_path(spec: str, title_path: list) -> str:
    """spec + titlePath → connector-relative folder."""
    if not spec or spec == "__all":
        spec_part = safe_name(title_path[0]) if title_path else "_untagged"
    else:
        spec_part = safe_spec(spec)
    if not title_path:
        return os.path.join(spec_part, "_untagged")
    contexts = title_path[1:-1] if len(title_path) > 2 else []
    if not contexts:
        return os.path.join(spec_part, safe_name(title_path[-1]))
    return os.path.join(spec_part, *[safe_name(c) for c in contexts])


def url_matches(url: str) -> bool:
    if not BASE_URLS:
        return True
    return any(url.startswith(b) for b in BASE_URLS)


def hash_from_rid(rid: str) -> str:
    return rid.split("-", 1)[0]


def iso(ts):
    return datetime.fromtimestamp(ts, tz=timezone.utc).isoformat() if ts else None


def encode_body(content: bytes, content_type: str = ""):
    if not content:
        return {"data": None, "encoding": None, "size_bytes": 0}
    size = len(content)
    try:
        text = content.decode("utf-8")
    except UnicodeDecodeError:
        return {
            "data": base64.b64encode(content).decode("ascii"),
            "encoding": "base64",
            "size_bytes": size,
        }
    if "json" in content_type.lower() or text.lstrip().startswith(("{", "[")):
        try:
            return {"data": json.loads(text), "encoding": "json", "size_bytes": size}
        except json.JSONDecodeError:
            pass
    return {"data": text, "encoding": "utf-8", "size_bytes": size}


def wipe_test_cassettes(folder: str, test_hash: str) -> int:
    """Delete prior cassettes for this testHash so a re-run starts clean.

    Matches both the new ``{hash}-NNN-SS.json`` layout and any legacy
    ``{hash}-NNN.json`` files lingering from earlier capture formats.
    Sibling tests (different it()s in the same context folder) keep
    their files — only this testHash's prefix is wiped.
    """
    if not os.path.isdir(folder):
        return 0
    removed = 0
    for path in glob.glob(os.path.join(folder, f"{test_hash}-*.json")):
        try:
            os.remove(path)
            removed += 1
        except OSError:
            pass
    return removed


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

        if self.path == "/test/start":
            test_id_hash = (body.get("testIdHash") or "").strip()
            title_path = body.get("titlePath") or []
            spec = body.get("spec") or "_untagged"
            connector = (body.get("connector") or "").strip()
            test = " > ".join(title_path) if title_path else "_untagged"

            if not test_id_hash:
                print(f"[capture] ▶ SKIP (no testIdHash): {test}")
                self._send(200, {"ok": False, "reason": "missing testIdHash"})
                return
            if not connector:
                print(f"[capture] ▶ SKIP (no connector): {test}")
                self._send(200, {"ok": False, "reason": "missing connector"})
                return

            rel_path = build_rel_path(spec, title_path)
            folder = os.path.join(OUT_DIR, connector, rel_path)
            os.makedirs(folder, exist_ok=True)
            wiped = wipe_test_cassettes(folder, test_id_hash)

            with _lock:
                _tests_by_hash[test_id_hash] = TestContext(
                    test=test, spec=spec, connector=connector, folder=folder,
                )
                # Drop seq counters for this test's rids so the next run
                # starts at seq=00 and the wipe + write produce a tidy set.
                for rid in [r for r in _seq_by_rid if hash_from_rid(r) == test_id_hash]:
                    del _seq_by_rid[rid]

            wipe_tag = f"(wiped {wiped}) " if wiped else ""
            print(f"[capture] ▶ START [{connector}] {wipe_tag}{test}")
            self._send(200, {"ok": True})

        elif self.path == "/test/end":
            # No-op. Map deliberately persists past /test/end — async
            # outbounds (e.g. VGS vault writes) land long after the test
            # ended and must still resolve to their issuing test.
            self._send(200, {"ok": True})

        elif self.path == "/status":
            with _lock:
                self._send(200, {
                    "tests_registered": len(_tests_by_hash),
                    "rids_in_flight": len(_seq_by_rid),
                    "out_dir": OUT_DIR,
                    "base_urls": BASE_URLS,
                })

        else:
            self._send(404, {"error": "unknown admin path"})

    def log_message(self, *_a, **_kw):
        return


def start_admin_server():
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[capture] admin server  http://127.0.0.1:{ADMIN_PORT}")


# ───── mitmproxy hooks ─────
def request(flow: http.HTTPFlow):
    """Claim a per-rid ``sequence`` slot in request-send order.

    Responses can complete out of order when Hyperswitch fans out (e.g.
    a fast VGS call beats a slow bluesnap call). If we numbered seq in
    response order, replay (which fires in request order and pops by
    cursor) would hand the wrong cassette body to the wrong outbound.
    Numbering in request order — which is what request() observes —
    keeps capture-order and replay-order aligned.
    """
    if not url_matches(flow.request.url):
        return
    rid = flow.request.headers.get("x-request-id", "").strip()
    if not rid or not _CYPRESS_RID.match(rid):
        return
    test_hash = hash_from_rid(rid)
    with _lock:
        if test_hash not in _tests_by_hash:
            return  # orphan rid — response() will log and drop
        seq = _seq_by_rid.get(rid, 0)
        _seq_by_rid[rid] = seq + 1
    flow.metadata["capture_seq"] = seq


def response(flow: http.HTTPFlow):
    # request() is the single gate: it decided whether to capture this
    # flow and claimed its sequence slot. If capture_seq wasn't set, the
    # flow isn't ours to record (filtered, orphan, or non-Cypress rid).
    seq = flow.metadata.get("capture_seq")
    if seq is None:
        return

    rid = flow.request.headers["x-request-id"].strip()
    test_hash = hash_from_rid(rid)
    with _lock:
        # Always present: request() verified the entry, and entries are
        # only ever added to _tests_by_hash, never removed.
        ctx = _tests_by_hash[test_hash]

    req = flow.request
    res = flow.response
    duration_ms = (
        round((res.timestamp_end - req.timestamp_start) * 1000, 2)
        if req.timestamp_start and res.timestamp_end else None
    )
    req_body = encode_body(req.content, req.headers.get("content-type", ""))
    res_body = encode_body(res.content, res.headers.get("content-type", ""))

    record = {
        "captured_at": iso(req.timestamp_start) or datetime.now(timezone.utc).isoformat(),
        "test": ctx.test,
        "spec": ctx.spec,
        "connector": ctx.connector,
        "request_id": rid,
        "sequence": seq,
        "timing": {
            "request_started_at": iso(req.timestamp_start),
            "response_completed_at": iso(res.timestamp_end),
            "duration_ms": duration_ms,
        },
        "request": {
            "http_version": req.http_version,
            "method": req.method,
            "url": req.url,
            "scheme": req.scheme,
            "host": req.host,
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

    out_path = os.path.join(ctx.folder, f"{rid}-{seq:02d}.json")
    with open(out_path, "w") as f:
        json.dump(record, f, indent=2)

    print(
        f"[capture] {req.method} {req.url} → {res.status_code} "
        f"({duration_ms}ms) rid={rid} seq={seq:02d} → {out_path}"
    )


# ───── boot ─────
start_admin_server()
print(f"[capture] base URLs   {BASE_URLS or '(none — capturing all)'}")
print(f"[capture] output dir  {OUT_DIR}")
