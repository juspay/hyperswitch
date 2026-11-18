"""Read-only check: every probe registry package must match H's current lock."""
from pathlib import Path
import hashlib
import json
import tomllib


def check():
    probe = Path(__file__).resolve().parent
    root = probe.parents[1]
    h_bytes = (root / "Cargo.lock").read_bytes()
    p_bytes = (probe / "Cargo.lock").read_bytes()
    h = tomllib.loads(h_bytes.decode())["package"]
    p = tomllib.loads(p_bytes.decode())["package"]
    identity = lambda v: (v["name"], v["version"], v.get("source"), v.get("checksum"))
    expected = {identity(v) for v in h}
    packages = [v for v in p if "source" in v]
    missing = [identity(v) for v in packages if identity(v) not in expected]
    if missing:
        raise ValueError("probe dependencies drifted from H: " + repr(missing))
    for packages_to_check in (h, p):
        sdk = [v for v in packages_to_check if v["name"] == "superposition_sdk"]
        if len(sdk) != 1 or sdk[0]["version"] != "0.119.0":
            raise ValueError("refresh the client contract before using a different locked SDK")
    return dict(client_version="0.119.0", registry_packages=len(packages),
                hyperswitch_lock_sha256=hashlib.sha256(h_bytes).hexdigest(),
                probe_lock_sha256=hashlib.sha256(p_bytes).hexdigest())


if __name__ == "__main__":
    print(json.dumps(check(), indent=2))
