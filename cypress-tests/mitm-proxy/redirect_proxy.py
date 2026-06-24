"""
Redirect proxy.

Transparent HTTP reverse proxy that sits in front of Hyperswitch.
When the ACS (3DS sandbox) POSTs to /payments/.../redirect/complete/...,
this proxy injects the pre-registered Cypress X-Request-ID so the MITM
capture proxy can attribute the outbound connector call to the right
test cassette.

The real response from Hyperswitch (302 with actual status) is passed
back to the browser unchanged — no synthetic status, no hardcoding.

The captured form body is saved to fixtures/proxy-bodies/{testIdHash}-redirect-body.json
for use during replay (cy.readFile in commands.js).

Admin (default port 9002):
  POST /test/start  { testIdHash }          — reset state for a new test
  POST /reserve     { rid, testIdHash }     — register RID for next redirect/complete
  GET  /status                              — health check

Proxy (default port 9001) → upstream Hyperswitch (default :8080).

Run:
  python redirect_proxy.py
  REDIRECT_PROXY_PORT=9001 REDIRECT_PROXY_UPSTREAM_PORT=8080 python redirect_proxy.py
"""

import http.client
import json
import os
import re
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, HTTPServer

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
FIXTURES_DIR = os.environ.get(
    "REDIRECT_BODY_DIR",
    os.path.normpath(os.path.join(SCRIPT_DIR, "..", "cypress", "fixtures", "proxy-bodies")),
)
CAPTURE_DIR = os.environ.get(
    "CAPTURE_DIR",
    os.path.normpath(os.path.join(SCRIPT_DIR, "captures")),
)
LISTEN_PORT = int(os.environ.get("REDIRECT_PROXY_PORT", "9001"))
ADMIN_PORT = int(os.environ.get("REDIRECT_PROXY_ADMIN_PORT", "9002"))
UPSTREAM_HOST = os.environ.get("REDIRECT_PROXY_UPSTREAM_HOST", "localhost")
UPSTREAM_PORT = int(os.environ.get("REDIRECT_PROXY_UPSTREAM_PORT", "8080"))

# Matches both /redirect/complete/{connector} (ACS form POST / GET, e.g. Redsys, Cybersource)
# and /redirect/response/{connector} (JS iframe return URL, e.g. Stripe).
_REDIRECT_COMPLETE_RE = re.compile(r".*/redirect/(complete|response)/[^/?]+")

_lock = threading.Lock()
# testIdHash → rid reserved for the next redirect/complete POST
_reserved: dict[str, str] = {}


def _save_redirect(
    test_id_hash: str,
    method: str,
    path_only: str,
    query_string: str,
    body: bytes,
    content_type: str,
) -> None:
    """Persist the redirect/complete or redirect/response request for replay.

    GET connectors (e.g. Cybersource DDC, Stripe 3DS): saves
      {"__redirect_method": "GET", "__redirect_segment": "redirect/response/stripe", "__query": {...}}.
    POST connectors (e.g. Redsys ACS form): saves the decoded form fields directly
    (no wrapper, for backwards compat with existing cassettes).
    """
    if method == "GET":
        query_params = dict(urllib.parse.parse_qsl(query_string)) if query_string else {}
        # Extract the redirect segment (redirect/complete/foo or redirect/response/foo)
        # from a path like /payments/{payId}/{merchantId}/redirect/response/stripe.
        parts = path_only.split("/")
        try:
            idx = parts.index("redirect")
            redirect_segment = "/".join(parts[idx:])
        except ValueError:
            redirect_segment = None
        data: dict = {"__redirect_method": "GET", "__query": query_params}
        if redirect_segment:
            data["__redirect_segment"] = redirect_segment
    else:
        if not body:
            return
        ct = content_type.lower()
        try:
            if "json" in ct:
                form_data = json.loads(body)
            elif "form" in ct or "x-www-form-urlencoded" in ct:
                form_data = dict(urllib.parse.parse_qsl(body.decode("utf-8")))
            else:
                form_data = body.decode("utf-8", errors="replace")
        except Exception:
            form_data = body.decode("utf-8", errors="replace")
        parts = path_only.split("/")
        try:
            idx = parts.index("redirect")
            redirect_segment = "/".join(parts[idx:])
        except ValueError:
            redirect_segment = None
        data = {"__redirect_method": "POST", "__body": form_data}
        if redirect_segment:
            data["__redirect_segment"] = redirect_segment

    # Save to fixtures/proxy-bodies/ for local replay
    os.makedirs(FIXTURES_DIR, exist_ok=True)
    path = os.path.join(FIXTURES_DIR, f"{test_id_hash}-redirect-body.json")
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
    print(f"[redirect-proxy] body saved → {path}")

    # Also save inside captures/{connector}/Payment/redirect-bodies/ so it gets
    # packaged with the cassettes tarball and is available in CI.
    redirect_segment = data.get("__redirect_segment")
    if redirect_segment:
        connector = redirect_segment.split("/")[-1]
        captures_body_dir = os.path.join(CAPTURE_DIR, connector, "Payment", "redirect-bodies")
        os.makedirs(captures_body_dir, exist_ok=True)
        captures_path = os.path.join(captures_body_dir, f"{test_id_hash}-redirect-body.json")
        with open(captures_path, "w") as f:
            json.dump(data, f, indent=2)
        print(f"[redirect-proxy] body also saved → {captures_path}")


# ───── proxy handler ─────

class ProxyHandler(BaseHTTPRequestHandler):
    def _forward(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""

        # Match any method (GET or POST) to redirect/complete.
        # Redsys ACS POSTs a form; Cybersource DDC does a GET via window.location.href.
        path_only = self.path.split("?", 1)[0]
        is_redirect_complete = bool(_REDIRECT_COMPLETE_RE.search(path_only))

        # Build headers to forward (drop hop-by-hop)
        fwd: dict[str, str] = {}
        for k, v in self.headers.items():
            if k.lower() in ("host", "content-length", "transfer-encoding", "connection"):
                continue
            fwd[k] = v

        if is_redirect_complete:
            with _lock:
                # Pop the first reserved RID (one active test at a time)
                entry = next(iter(_reserved.items()), None)
                if entry:
                    test_id_hash, rid = entry
                    del _reserved[test_id_hash]
                else:
                    test_id_hash, rid = "", ""

            if rid:
                fwd["X-Request-ID"] = rid
                print(f"[redirect-proxy] injected RID {rid} for {self.command} {self.path}")
                query_string = self.path.split("?", 1)[1] if "?" in self.path else ""
                _save_redirect(
                    test_id_hash,
                    self.command,
                    path_only,
                    query_string,
                    body,
                    self.headers.get("Content-Type", ""),
                )
            else:
                print(f"[redirect-proxy] WARNING: no reserved RID for {self.path}")

        # Forward to Hyperswitch
        try:
            conn = http.client.HTTPConnection(UPSTREAM_HOST, UPSTREAM_PORT, timeout=30)
            conn.request(self.command, self.path, body=body or None, headers=fwd)
            resp = conn.getresponse()
            resp_body = resp.read()
            status = resp.status
            resp_headers = list(resp.getheaders())
        except Exception as exc:
            self.send_error(502, f"Upstream error: {exc}")
            return
        finally:
            try:
                conn.close()
            except Exception:
                pass

        self.send_response(status)
        for k, v in resp_headers:
            if k.lower() in ("transfer-encoding", "connection"):
                continue
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(resp_body)

    do_GET = do_POST = do_PUT = do_PATCH = do_DELETE = do_HEAD = _forward

    def log_message(self, *_):
        pass


# ───── admin handler ─────

class AdminHandler(BaseHTTPRequestHandler):
    def _json(self, code: int, payload: dict) -> None:
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        try:
            data = json.loads(self.rfile.read(length) or b"{}")
        except json.JSONDecodeError:
            data = {}

        if self.path == "/test/start":
            test_id_hash = (data.get("testIdHash") or "").strip()
            with _lock:
                _reserved.pop(test_id_hash, None)
            print(f"[redirect-proxy] test/start hash={test_id_hash or '(none)'}")
            self._json(200, {"ok": True})

        elif self.path == "/reserve":
            rid = (data.get("rid") or "").strip()
            test_id_hash = (data.get("testIdHash") or "").strip()
            if rid and test_id_hash:
                with _lock:
                    _reserved[test_id_hash] = rid
                print(f"[redirect-proxy] reserved {rid} for hash={test_id_hash}")
                self._json(200, {"ok": True})
            else:
                self._json(400, {"ok": False, "reason": "missing rid or testIdHash"})

        else:
            self._json(404, {"error": "unknown path"})

    def do_GET(self):
        if self.path in ("/status", "/"):
            with _lock:
                self._json(200, {
                    "reserved": dict(_reserved),
                    "fixtures_dir": FIXTURES_DIR,
                    "upstream": f"{UPSTREAM_HOST}:{UPSTREAM_PORT}",
                })
        else:
            self._json(404, {"error": "unknown path"})

    def log_message(self, *_):
        pass


# ───── boot ─────

def _start_admin() -> None:
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[redirect-proxy] admin  http://127.0.0.1:{ADMIN_PORT}")


def main() -> None:
    os.makedirs(FIXTURES_DIR, exist_ok=True)
    _start_admin()
    proxy = HTTPServer(("0.0.0.0", LISTEN_PORT), ProxyHandler)
    print(f"[redirect-proxy] proxy  http://0.0.0.0:{LISTEN_PORT} → http://{UPSTREAM_HOST}:{UPSTREAM_PORT}")
    print(f"[redirect-proxy] bodies {FIXTURES_DIR}")
    proxy.serve_forever()


if __name__ == "__main__":
    main()
