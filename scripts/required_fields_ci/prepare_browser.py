#!/usr/bin/env python3
"""Pin-check and archive local W/U sources into a new registered fixture. No network/install."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import uuid

H = Path(__file__).resolve().parents[2]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def lock():
    value = json.loads((Path(__file__).parent / "verifier.lock.json").read_text())
    assert value["schema_version"] == 1
    assert sha((H / value["case_pack"]).read_bytes()) == value["case_pack_sha256"], "case pack drift"
    return value


def artifact_members(pins, data):
    assert len(data) <= 1024 * 1024 and sha(data) == pins["artifact_sha256"], "verifier artifact drift"
    expected = {row["path"]: row["sha256"] for row in pins["artifact_files"]}
    assert len(expected) == len(pins["artifact_files"]) <= 64
    result = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as archive:
        for member in archive.getmembers():
            path = PurePosixPath(member.name)
            assert member.isfile() and not path.is_absolute() and ".." not in path.parts
            assert "\\" not in member.name and member.name in expected and member.name not in result
            assert member.size <= 256 * 1024
            content = archive.extractfile(member).read()
            assert sha(content) == expected[member.name]
            result[member.name] = content
    assert set(result) == set(expected), "missing verifier source"
    return result


def git(source, *args):
    return subprocess.check_output(["git", "-C", str(source), *args], timeout=60)


def archive_source(source, destination):
    # Exclude dotenv/npm credentials before creating the archive. Never run lifecycle scripts.
    files = git(source, "ls-files", "-z").decode().split("\0")
    files = [f for f in files if f and not any(p.startswith(".env") or p == ".npmrc" for p in PurePosixPath(f).parts)]
    blob = git(source, "archive", "HEAD", "--", *files)
    assert len(blob) <= 128 * 1024 * 1024
    with tarfile.open(fileobj=io.BytesIO(blob)) as archive:
        for member in archive.getmembers():
            path = PurePosixPath(member.name)
            assert not path.is_absolute() and ".." not in path.parts
            assert member.isdir() or member.isfile(), "source links require explicit review"
            target = destination / path
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("xb") as output:
                    output.write(archive.extractfile(member).read())


def prepare(web, root):
    pins = lock()
    sources = artifact_members(pins, (H / pins["artifact"]).read_bytes())
    web = web.resolve(strict=True)
    shared = web / "shared-code"
    for source, revision in [(web, pins["web_revision"]), (shared, pins["shared_revision"])]:
        assert git(source, "rev-parse", "HEAD").decode().strip() == revision, "pinned source revision mismatch"
        subprocess.run(["git", "-C", str(source), "diff", "--quiet", "HEAD", "--"], check=True)
    gitlink = git(web, "ls-tree", "HEAD", "shared-code").decode().split()
    assert gitlink[:3] == ["160000", "commit", pins["shared_revision"]], "gitlink mismatch"
    for row in pins["dependency_locks"]:
        assert sha((web / row["path"]).read_bytes()) == row["sha256"], "dependency lock drift"
    root = root.absolute()
    root.mkdir()  # Refuse reuse, including symlinks. Cleanup is never implicit.
    root = root.resolve(strict=True)
    destination = root / "web"
    destination.mkdir()
    marker = {"owner": "required-fields-step1-" + str(uuid.uuid4()), "kind": "disposable-filesystem-build",
              "root": str(root), "web": str(destination), "cleanup_scope": str(root),
              "hosted_target": None, "server_mutations": False, "parent_step": 14}
    (root / "fixture-target.json").write_text(json.dumps(marker, indent=2) + "\n")
    for name in ["npm-user.npmrc", "npm-global.npmrc"]:
        (root / name).write_text("")
    archive_source(web, destination)
    archive_source(shared, destination / "shared-code")
    for path, data in sources.items():
        target = destination / path
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open("xb") as output:
            output.write(data)
    (root / "verifier-registration.json").write_text(json.dumps(pins, indent=2) + "\n")
    return marker


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--web-source", type=Path, required=True)
    parser.add_argument("--work-root", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.web_source, args.work_root)))
