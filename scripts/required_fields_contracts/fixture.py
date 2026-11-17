"""Explicit local disposable fixtures only. No image pulls or API seeding.

run: create isolated owned containers from already cached immutable images,
verify them, run the locked Rust probe, then destroy only those containers.
verify: read-only identity guard called again by Rust before every dispatch.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
import uuid
from urllib.parse import urlparse

LABEL = "io.hyperswitch.required-fields-contract-run"
ROLE = "io.hyperswitch.required-fields-contract-role"
VERSIONS = {"0.102.0", "0.113.0", "0.115.5"}


def require(ok, message):
    if not ok:
        raise ValueError(message)


def exact_digest(value):
    return bool(re.fullmatch(r"sha256:[0-9a-f]{64}", value))


def registration_shape(r):
    require(set(r) == {
        "schema_version", "run_id", "docker_host", "server_image", "server_image_id",
        "server_version", "source_revision", "server_container", "redis_container",
        "network", "endpoint", "org", "workspace",
    }, "unknown/missing registration fields")
    require(r["schema_version"] == 1, "registration version")
    require(bool(re.fullmatch(r"[0-9a-f]{32}", r["run_id"])), "run identity")
    require(r["docker_host"].startswith("unix:///"), "local Unix Docker socket required")
    require(bool(re.fullmatch(r"ghcr.io/juspay/superposition-demo@sha256:[0-9a-f]{64}", r["server_image"])), "exact demo image required")
    require(exact_digest(r["server_image_id"]), "image ID required")
    require(r["server_version"] in VERSIONS, "unregistered version")
    require(bool(re.fullmatch(r"[0-9a-f]{40}", r["source_revision"])), "source revision required")
    for key in ("server_container", "redis_container", "network"):
        require(bool(re.fullmatch(r"[0-9a-f]{64}", r[key])), "exact resource identity")
    require(r["org"] == "localorg" and r["workspace"] == "rf_contract_" + r["run_id"], "fixture workspace mismatch")
    u = urlparse(r["endpoint"])
    require(u.scheme == "http" and u.hostname == "127.0.0.1" and u.port
            and u.path in ("", "/") and not u.username and not u.password
            and not u.query and not u.fragment, "literal loopback endpoint only")


class Docker:
    def __init__(self, host):
        require(host.startswith("unix:///"), "Docker remote contexts forbidden")
        self.host = host

    def call(self, *args, timeout=20):
        env = {k: os.environ[k] for k in ("PATH", "HOME", "TMPDIR") if k in os.environ}
        result = subprocess.run(["docker", "--host", self.host, *args],
                                capture_output=True, text=True, timeout=timeout, env=env)
        require(result.returncode == 0, "local Docker operation failed: " + args[0])
        require(len(result.stdout) <= 1024 * 1024, "Docker output cap")
        return result.stdout.strip()

    def inspect(self, kind, identity):
        rows = json.loads(self.call(kind, "inspect", identity))
        require(isinstance(rows, list) and len(rows) == 1, "ambiguous Docker identity")
        return rows[0]


def verify(r, docker):
    registration_shape(r)
    network = docker.inspect("network", r["network"])
    require(network["Id"] == r["network"] and network["Internal"] is True
            and network["Driver"] == "bridge" and network["Labels"].get(LABEL) == r["run_id"],
            "not an owned internal fixture network")
    require(set(network.get("Containers", {})) == {r["server_container"], r["redis_container"]},
            "unregistered network peer")
    for key, role in (("server_container", "server"), ("redis_container", "redis")):
        c = docker.inspect("container", r[key])
        require(c["Id"] == r[key] and c["State"]["Running"] is True, "fixture container not running")
        require(c["Config"]["Labels"].get(LABEL) == r["run_id"] and c["Config"]["Labels"].get(ROLE) == role,
                "container owner/role mismatch")
        require(not c["Mounts"] and not c["HostConfig"].get("Privileged")
                and not c["HostConfig"].get("Binds") and not c["HostConfig"].get("VolumesFrom")
                and not c["HostConfig"].get("PidMode") and not c["HostConfig"].get("Devices"),
                "host/shared mount or privilege refused")
        nets = c["NetworkSettings"]["Networks"]
        require(len(nets) == 1 and next(iter(nets.values()))["NetworkID"] == r["network"], "network escape")
        if role == "server":
            require(c["Config"]["Image"] == r["server_image"] and c["Image"] == r["server_image_id"], "server image drift")
            ports = c["NetworkSettings"]["Ports"]
            bound = [p for values in ports.values() if values for p in values]
            require(bound == [{"HostIp": "127.0.0.1", "HostPort": str(urlparse(r["endpoint"]).port)}]
                    and ports.get("8080/tcp") == bound, "unexpected exposed port")
        else:
            require("@sha256:" in c["Config"]["Image"] and not any(c["NetworkSettings"]["Ports"].values()), "redis identity/exposure")
    image = docker.inspect("image", r["server_image"])
    require(image["Id"] == r["server_image_id"] and r["server_image"] in image.get("RepoDigests", []), "image digest mismatch")
    labels = image["Config"].get("Labels") or {}
    require(labels.get("org.opencontainers.image.revision") == r["source_revision"], "source provenance unavailable/mismatch")
    require(labels.get("org.opencontainers.image.version") in (r["server_version"], "v" + r["server_version"]), "server version provenance unavailable/mismatch")


def cleanup(docker, run_id, network_id):
    # Includes a container whose create response was lost, but only this nonce.
    ids = docker.call("ps", "-aq", "--filter", f"label={LABEL}={run_id}").splitlines()
    for identity in ids:
        c = docker.inspect("container", identity)
        require(c["Config"]["Labels"].get(LABEL) == run_id
                and c["Config"]["Labels"].get(ROLE) in ("server", "redis"), "cleanup ownership mismatch")
        docker.call("rm", "--force", "--volumes", c["Id"])
    networks = docker.call("network", "ls", "--quiet", "--filter", f"label={LABEL}={run_id}").splitlines()
    for identity in networks:
        n = docker.inspect("network", identity)
        require((network_id is None or n["Id"] == network_id) and n["Labels"].get(LABEL) == run_id
                and not n.get("Containers"), "cleanup network ownership mismatch")
        docker.call("network", "rm", n["Id"])
    require(not docker.call("ps", "-aq", "--filter", f"label={LABEL}={run_id}"), "owned container cleanup incomplete")


def run(args):
    from check_lock import check
    check()
    require(args.server_version in VERSIONS, "unregistered server version")
    require(bool(re.fullmatch(r"ghcr.io/juspay/superposition-demo@sha256:[0-9a-f]{64}", args.server_image)), "server digest required")
    require(bool(re.fullmatch(r"[a-z0-9./_-]+@sha256:[0-9a-f]{64}", args.redis_image)), "redis digest required")
    require(bool(re.fullmatch(r"[0-9a-f]{40}", args.source_revision)), "source commit required")
    require(Path(args.probe).is_file(), "build the locked probe first")
    docker = Docker(args.docker_host)
    # Read-only cache lookup. Missing images fail without pulling/starting anything.
    image = docker.inspect("image", args.server_image)
    docker.inspect("image", args.redis_image)
    labels = image["Config"].get("Labels") or {}
    require(args.server_image in image.get("RepoDigests", []) and labels.get("org.opencontainers.image.revision") == args.source_revision
            and labels.get("org.opencontainers.image.version") in (args.server_version, "v" + args.server_version), "exact source/image/version proof unavailable")
    root = Path(tempfile.mkdtemp(prefix="required-fields-api-"))
    run_id = uuid.uuid4().hex
    (root / "owner.json").write_text(json.dumps({"run_id": run_id, "root": str(root), "cleanup_scope": str(root), "kind": "disposable-api-fixture"}) + "\n")
    network = None
    result = None
    try:
        network = docker.call("network", "create", "--internal", "--label", f"{LABEL}={run_id}", "rf-contract-" + run_id)
        common = ["run", "--detach", "--pull=never", "--network", network, "--label", f"{LABEL}={run_id}"]
        redis = docker.call(*common, "--label", f"{ROLE}=redis", "--network-alias", "rf-redis", args.redis_image)
        server = docker.call(*common, "--label", f"{ROLE}=server", "--publish", "127.0.0.1::8080",
                             "--env", "REDIS_URL=redis://rf-redis:6379", "--env", "API_HOSTNAME=http://127.0.0.1:8080", args.server_image)
        c = docker.inspect("container", server)
        port = c["NetworkSettings"]["Ports"]["8080/tcp"][0]["HostPort"]
        r = dict(schema_version=1, run_id=run_id, docker_host=args.docker_host, server_image=args.server_image,
                 server_image_id=image["Id"], server_version=args.server_version, source_revision=args.source_revision,
                 server_container=server, redis_container=redis, network=network,
                 endpoint="http://127.0.0.1:" + port, org="localorg", workspace="rf_contract_" + run_id)
        registration = root / "registration.json"
        registration.write_text(json.dumps(r, indent=2) + "\n")
        verify(r, docker)
        # Bounded readiness using a local TCP connection only; no API write.
        import socket
        deadline = time.monotonic() + 30
        while True:
            try:
                with socket.create_connection(("127.0.0.1", int(port)), timeout=1):
                    break
            except OSError:
                require(time.monotonic() < deadline, "fixture readiness deadline")
                time.sleep(0.25)
        verify(r, docker)
        env = {k: os.environ[k] for k in ("PATH", "HOME", "TMPDIR") if k in os.environ}
        result = subprocess.run([str(Path(args.probe).resolve()), "--registration", str(registration), "--report", str(root / "report.json")],
                                env=env, timeout=600, check=False)
    finally:
        # Cleanup even failed/unknown test requests by stopping the owned server;
        # this is disposal, not certification of production request termination.
        try:
            cleanup(docker, run_id, network)
            (root / "cleanup.json").write_text(json.dumps({"run_id": run_id, "owned_resources_removed": True}) + "\n")
        except Exception:
            (root / "cleanup.json").write_text(json.dumps({"run_id": run_id, "owned_resources_removed": False}) + "\n")
            raise
        finally:
            print("Fixture evidence retained at", root)
    return result.returncode if result else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    check = sub.add_parser("verify")
    check.add_argument("--registration", required=True, type=Path)
    launch = sub.add_parser("run")
    for name in ("docker-host", "server-image", "server-version", "source-revision", "redis-image", "probe"):
        launch.add_argument("--" + name, required=True)
    args = parser.parse_args()
    if args.command == "verify":
        require(args.registration.stat().st_size <= 16384, "registration byte cap")
        r = json.loads(args.registration.read_text())
        verify(r, Docker(r["docker_host"]))
        return 0
    return run(args)


if __name__ == "__main__":
    raise SystemExit(main())
