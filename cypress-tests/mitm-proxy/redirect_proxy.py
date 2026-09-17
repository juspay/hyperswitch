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
CAPTURE_DIR = os.environ.get(
    "CAPTURE_DIR",
    os.path.normpath(os.path.join(SCRIPT_DIR, "captures")),
)
LISTEN_PORT = int(os.environ.get("REDIRECT_PROXY_PORT", "9001"))
ADMIN_PORT = int(os.environ.get("REDIRECT_PROXY_ADMIN_PORT", "9002"))
UPSTREAM_HOST = os.environ.get("REDIRECT_PROXY_UPSTREAM_HOST", "localhost")
UPSTREAM_PORT = int(os.environ.get("REDIRECT_PROXY_UPSTREAM_PORT", "8080"))

_HOP_BY_HOP = frozenset(("host", "content-length", "transfer-encoding", "connection"))

_lock = threading.Lock()
_reserved: dict[str, dict] = {}
_redirect_count: dict[str, int] = {}


# ── Path helpers ──────────────────────────────────────────────────────────────

def _is_redirect_complete(path: str) -> bool:
    return "/redirect/complete/" in path or "/redirect/response/" in path


def _extract_payment_id(path: str) -> str:
    parts = path.split("/")
    return parts[2] if len(parts) > 2 else ""


def _extract_redirect_segment(path: str) -> str | None:
    parts = path.split("/")
    try:
        idx = parts.index("redirect")
        return "/".join(parts[idx:])
    except ValueError:
        return None


# ── Body parsing ──────────────────────────────────────────────────────────────

def _parse_post_body(body: bytes, content_type: str) -> dict | str:
    ct = content_type.lower()
    try:
        if "json" in ct:
            return json.loads(body)
        if "form" in ct or "x-www-form-urlencoded" in ct:
            return dict(urllib.parse.parse_qsl(body.decode("utf-8")))
        return body.decode("utf-8", errors="replace")
    except Exception:
        return body.decode("utf-8", errors="replace")


def _build_redirect_data(
    method: str,
    path_only: str,
    query_string: str,
    body: bytes,
    content_type: str,
) -> dict | None:
    redirect_segment = _extract_redirect_segment(path_only)

    if method == "GET":
        query_params = dict(urllib.parse.parse_qsl(query_string)) if query_string else {}
        data: dict = {"__redirect_method": "GET", "__query": query_params}
    else:
        if not body:
            return None
        data = {"__redirect_method": "POST", "__body": _parse_post_body(body, content_type)}

    if redirect_segment:
        data["__redirect_segment"] = redirect_segment
    return data


# ── Persistence ───────────────────────────────────────────────────────────────

def _write_json(path: str, data: dict) -> None:
    with open(path, "w") as f:
        json.dump(data, f, indent=2)


def _write_to_captures(data: dict, filename: str) -> None:
    redirect_segment = data.get("__redirect_segment")
    if not redirect_segment:
        return

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
    captures_path = os.path.realpath(os.path.join(real_dir, os.path.basename(filename)))

    if not captures_path.startswith(real_dir + os.sep):
        print(f"[redirect-proxy] WARNING: resolved file path {captures_path!r} escapes target dir — skipping")
        return

    _write_json(captures_path, data)
    print(f"[redirect-proxy] body also saved → {captures_path}")


def _save_redirect(
    test_id_hash: str,
    method: str,
    path_only: str,
    query_string: str,
    body: bytes,
    content_type: str,
) -> None:
    data = _build_redirect_data(method, path_only, query_string, body, content_type)
    if data is None:
        return

    with _lock:
        _redirect_count[test_id_hash] = _redirect_count.get(test_id_hash, 0) + 1
        seq = str(_redirect_count[test_id_hash]).zfill(3)

    filename = f"{test_id_hash}-{seq}-redirect-body.json"

    _write_to_captures(data, filename)


# ── Proxy handler ─────────────────────────────────────────────────────────────

class ProxyHandler(BaseHTTPRequestHandler):

    def _read_body(self) -> bytes:
        length = int(self.headers.get("Content-Length", 0))
        return self.rfile.read(length) if length else b""

    def _forward_headers(self) -> dict[str, str]:
        return {k: v for k, v in self.headers.items() if k.lower() not in _HOP_BY_HOP}

    def _inject_rid(self, path_only: str, body: bytes, headers: dict) -> None:
        payment_id = _extract_payment_id(path_only)
        with _lock:
            entry = _reserved.pop(payment_id, None)

        if entry:
            headers["X-Request-ID"] = entry["rid"]
            print(f"[redirect-proxy] injected RID {entry['rid']} for paymentId={payment_id}")
            query_string = self.path.split("?", 1)[1] if "?" in self.path else ""
            _save_redirect(
                entry["testIdHash"],
                self.command,
                path_only,
                query_string,
                body,
                self.headers.get("Content-Type", ""),
            )
        else:
            print(f"[redirect-proxy] WARNING: no reserved RID for paymentId={payment_id} path={self.path}")

    def _call_upstream(self, headers: dict, body: bytes):
        conn = http.client.HTTPConnection(UPSTREAM_HOST, UPSTREAM_PORT, timeout=30)
        try:
            conn.request(self.command, self.path, body=body or None, headers=headers)
            resp = conn.getresponse()
            return resp.status, list(resp.getheaders()), resp.read()
        finally:
            try:
                conn.close()
            except Exception:
                pass

    def _forward(self):
        body = self._read_body()
        path_only = self.path.split("?", 1)[0]
        headers = self._forward_headers()

        if _is_redirect_complete(path_only):
            self._inject_rid(path_only, body, headers)

        try:
            status, resp_headers, resp_body = self._call_upstream(headers, body)
        except Exception as exc:
            self.send_error(502, f"Upstream error: {exc}")
            return

        self.send_response(status)
        for k, v in resp_headers:
            if k.lower() not in ("transfer-encoding", "connection"):
                self.send_header(k, v)
        self.end_headers()
        self.wfile.write(resp_body)

    do_GET = do_POST = do_PUT = do_PATCH = do_DELETE = do_HEAD = _forward

    def log_message(self, *_):
        pass


# ── Admin handler ─────────────────────────────────────────────────────────────

class AdminHandler(BaseHTTPRequestHandler):

    def _json(self, code: int, payload: dict) -> None:
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json_body(self) -> dict:
        length = int(self.headers.get("Content-Length", 0))
        try:
            return json.loads(self.rfile.read(length) or b"{}")
        except json.JSONDecodeError:
            return {}

    def _handle_test_start(self, data: dict) -> None:
        test_id_hash = (data.get("testIdHash") or "").strip()
        with _lock:
            _redirect_count.pop(test_id_hash, None)
        print(f"[redirect-proxy] test/start hash={test_id_hash or '(none)'}")
        self._json(200, {"ok": True})

    def _handle_reserve(self, data: dict) -> None:
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

    def do_POST(self):
        data = self._read_json_body()
        handlers = {
            "/test/start": self._handle_test_start,
            "/reserve": self._handle_reserve,
        }
        handler = handlers.get(self.path)
        if handler:
            handler(data)
        else:
            self._json(404, {"error": "unknown path"})

    def do_GET(self):
        if self.path in ("/status", "/"):
            with _lock:
                self._json(200, {
                    "reserved": dict(_reserved),
                    "capture_dir": CAPTURE_DIR,
                    "upstream": f"{UPSTREAM_HOST}:{UPSTREAM_PORT}",
                })
        else:
            self._json(404, {"error": "unknown path"})

    def log_message(self, *_):
        pass


# ── Entry point ───────────────────────────────────────────────────────────────

def _start_admin() -> None:
    server = HTTPServer(("127.0.0.1", ADMIN_PORT), AdminHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"[redirect-proxy] admin  http://127.0.0.1:{ADMIN_PORT}")


def main() -> None:
    _start_admin()
    proxy = HTTPServer(("0.0.0.0", LISTEN_PORT), ProxyHandler)
    print(f"[redirect-proxy] proxy  http://0.0.0.0:{LISTEN_PORT} → http://{UPSTREAM_HOST}:{UPSTREAM_PORT}")
    print(f"[redirect-proxy] bodies {CAPTURE_DIR}")
    proxy.serve_forever()


if __name__ == "__main__":
    main()
