"""
Redirect proxy — sits in front of Hyperswitch on port 9001.
Injects X-Request-ID on redirect/complete and redirect/response calls so the
MITM proxy can attribute the outbound connector call to the right cassette.
Saves the redirect body for replay.

Admin (port 9002): POST /test/start, POST /reserve, GET /status
Run: python redirect_proxy.py
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

_REDIRECT_COMPLETE_RE = re.compile(r"/redirect/(?:complete|response)/[^/?]+")

_lock = threading.Lock()
_reserved: dict[str, dict] = {}
_redirect_count: dict[str, int] = {}


def _save_redirect(
    test_id_hash: str,
    method: str,
    path_only: str,
    query_string: str,
    body: bytes,
    content_type: str,
) -> None:
    if method == "GET":
        query_params = dict(urllib.parse.parse_qsl(query_string)) if query_string else {}
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

    with _lock:
        _redirect_count[test_id_hash] = _redirect_count.get(test_id_hash, 0) + 1
        seq = str(_redirect_count[test_id_hash]).zfill(3)
    filename = f"{test_id_hash}-{seq}-redirect-body.json"

    os.makedirs(FIXTURES_DIR, exist_ok=True)
    path = os.path.join(FIXTURES_DIR, filename)
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
    print(f"[redirect-proxy] body saved → {path}")

    redirect_segment = data.get("__redirect_segment")
    if redirect_segment:
        raw_connector = redirect_segment.split("/")[-1]
        connector = re.sub(r"[^a-zA-Z0-9\-]", "", raw_connector)
        if not connector:
            return
        captures_body_dir = os.path.join(CAPTURE_DIR, connector, "Payment", "redirect-bodies")
        real_dir = os.path.realpath(captures_body_dir)
        real_base = os.path.realpath(CAPTURE_DIR)
        if not real_dir.startswith(real_base + os.sep):
            print(f"[redirect-proxy] WARNING: resolved path {real_dir!r} escapes CAPTURE_DIR — skipping")
            return
        os.makedirs(real_dir, exist_ok=True)
        safe_filename = os.path.basename(filename)
        captures_path = os.path.realpath(os.path.join(real_dir, safe_filename))
        if not captures_path.startswith(real_dir + os.sep):
            print(f"[redirect-proxy] WARNING: resolved file path {captures_path!r} escapes target dir — skipping")
            return
        with open(captures_path, "w") as f:
            json.dump(data, f, indent=2)
        print(f"[redirect-proxy] body also saved → {captures_path}")


class ProxyHandler(BaseHTTPRequestHandler):
    def _forward(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""

        path_only = self.path.split("?", 1)[0]
        is_redirect_complete = bool(_REDIRECT_COMPLETE_RE.search(path_only))

        fwd: dict[str, str] = {}
        for k, v in self.headers.items():
            if k.lower() in ("host", "content-length", "transfer-encoding", "connection"):
                continue
            fwd[k] = v

        if is_redirect_complete:
            # Extract payment_id from path: /payments/{payment_id}/{merchant_id}/redirect/...
            parts = path_only.split("/")
            payment_id = parts[2] if len(parts) > 2 else ""

            with _lock:
                entry = _reserved.pop(payment_id, None)

            if entry:
                rid = entry["rid"]
                test_id_hash = entry["testIdHash"]
                fwd["X-Request-ID"] = rid
                print(f"[redirect-proxy] injected RID {rid} for paymentId={payment_id}")
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
                print(f"[redirect-proxy] WARNING: no reserved RID for paymentId={payment_id} path={self.path}")

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
                _redirect_count.pop(test_id_hash, None)
            print(f"[redirect-proxy] test/start hash={test_id_hash or '(none)'}")
            self._json(200, {"ok": True})

        elif self.path == "/reserve":
            rid = (data.get("rid") or "").strip()
            test_id_hash = (data.get("testIdHash") or "").strip()
            payment_id = (data.get("paymentId") or "").strip()
            if rid and test_id_hash and payment_id:
                with _lock:
                    _reserved[payment_id] = {"rid": rid, "testIdHash": test_id_hash}
                print(f"[redirect-proxy] reserved {rid} for paymentId={payment_id} hash={test_id_hash}")
                self._json(200, {"ok": True})
            else:
                self._json(400, {"ok": False, "reason": "missing rid, testIdHash, or paymentId"})

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
