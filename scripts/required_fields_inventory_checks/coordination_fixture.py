"""Owned loopback wire fixture, NOT a Kubernetes server or certification substitute."""
import argparse
import http.server
import json
import os
from pathlib import Path
import socket
import threading


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--child", required=True)
    args = parser.parse_args()
    root = args.root
    assert root.is_absolute() and root.resolve() == root
    owner = json.loads((root / "owner.json").read_text())
    assert owner["purpose"] == "Step 9 isolated coordination checks"
    assert owner["root"] == owner["cleanup_scope"] == str(root)
    assert args.child.startswith("coordinator-wire-") and args.child.removeprefix("coordinator-wire-").isdigit()
    child = root / args.child
    assert child.resolve() == child and child.parent == root
    file = child / "lease.json"
    data = json.loads(file.read_text())
    assert data["metadata"]["uid"] == "fixture-object-uid"
    assert data["metadata"]["namespace"] == "rf-fixture"
    assert data["metadata"]["name"] == "rf-target"
    lock = threading.Lock()

    def persist(value):
        temporary = child / "next-owned.json"
        with temporary.open("x") as output:
            json.dump(value, output, separators=(",", ":"))
            output.flush()
            os.fsync(output.fileno())
        temporary.replace(file)

    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *_args):
            pass

        def reply(self, status, value):
            body = json.dumps(value, separators=(",", ":")).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("X-Required-Fields-Fixture", owner["owner"])
            self.end_headers()
            self.wfile.write(body)

        def handle_request(self, write):
            with lock:
                if self.path != "/apis/coordination.k8s.io/v1/namespaces/rf-fixture/leases/rf-target":
                    self.reply(404, {})
                    return
                mode = (child / "mode").read_text() if (child / "mode").exists() else "normal"
                length = int(self.headers.get("Content-Length", "0"))
                if length > 1024 * 1024:
                    self.reply(413, {})
                    return
                body = self.rfile.read(length)
                if mode == "deny":
                    self.reply(403, {})
                    return
                current = json.loads(file.read_text())
                if not write:
                    self.reply(200, current)
                    return
                if self.headers.get("X-Required-Fields-Fixture-Write") != owner["owner"]:
                    self.reply(403, {})
                    return
                new = json.loads(body)
                for key in ["uid", "name", "namespace", "resourceVersion"]:
                    if new["metadata"][key] != current["metadata"][key]:
                        self.reply(409, {})
                        return
                new["metadata"]["resourceVersion"] = str(int(current["metadata"]["resourceVersion"]) + 1)
                persist(new)
                counter = child / "writes"
                counter.write_text(str((int(counter.read_text()) if counter.exists() else 0) + 1))
                if mode == "lost-ack":
                    self.close_connection = True
                    self.connection.shutdown(socket.SHUT_RDWR)
                    self.connection.close()
                    return
                self.reply(200, new)

        def do_GET(self):
            self.handle_request(False)

        def do_PUT(self):
            self.handle_request(True)

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    print(server.server_port, flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
