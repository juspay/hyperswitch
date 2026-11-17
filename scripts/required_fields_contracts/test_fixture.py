"""Ownership guard/cleanup unit tests. Fake Docker metadata is NOT image proof."""
import copy
import unittest

import fixture


def sample():
    r = dict(schema_version=1, run_id="a" * 32, docker_host="unix:///fixture/docker.sock",
             server_image="ghcr.io/juspay/superposition-demo@sha256:" + "b" * 64,
             server_image_id="sha256:" + "c" * 64, server_version="0.115.5", source_revision="d" * 40,
             server_container="e" * 64, redis_container="f" * 64, network="1" * 64,
             endpoint="http://127.0.0.1:34567", org="localorg", workspace="rf_contract_" + "a" * 32)
    objects = {}
    objects["network", r["network"]] = dict(Id=r["network"], Internal=True, Driver="bridge",
                                           Labels={fixture.LABEL: r["run_id"]},
                                           Containers={r["server_container"]: {}, r["redis_container"]: {}})
    for key, role in (("server_container", "server"), ("redis_container", "redis")):
        objects["container", r[key]] = {
            "Id": r[key], "State": {"Running": True}, "Mounts": [], "HostConfig": {},
            "Image": r["server_image_id"],
            "Config": {"Image": r["server_image"] if role == "server" else "redis@sha256:" + "2" * 64,
                       "Labels": {fixture.LABEL: r["run_id"], fixture.ROLE: role}},
            "NetworkSettings": {"Networks": {"fixture": {"NetworkID": r["network"]}},
                                "Ports": {"8080/tcp": [{"HostIp": "127.0.0.1", "HostPort": "34567"}]} if role == "server" else {}},
        }
    objects["image", r["server_image"]] = {
        "Id": r["server_image_id"], "RepoDigests": [r["server_image"]],
        "Config": {"Labels": {"org.opencontainers.image.revision": r["source_revision"],
                              "org.opencontainers.image.version": r["server_version"]}},
    }
    return r, objects


class FakeDocker:
    def __init__(self, objects):
        self.objects = objects
        self.calls = []

    def inspect(self, kind, identity):
        self.calls.append((kind, "inspect", identity))
        return copy.deepcopy(self.objects[kind, identity])

    def call(self, *args):
        self.calls.append(args)
        if args[:2] == ("ps", "-aq"):
            return "\n".join(key for kind, key in self.objects if kind == "container")
        if args[:2] == ("network", "ls"):
            return "\n".join(key for kind, key in self.objects if kind == "network")
        if args[0] == "rm":
            identity = args[-1]
            del self.objects["container", identity]
            for (kind, _), obj in self.objects.items():
                if kind == "network":
                    obj["Containers"].pop(identity, None)
            return identity
        if args[:2] == ("network", "rm"):
            del self.objects["network", args[-1]]
            return args[-1]
        raise AssertionError("unexpected Docker operation")


class TargetTests(unittest.TestCase):
    def test_registered_owned_fixture_is_read_only_verified(self):
        r, objects = sample()
        docker = FakeDocker(objects)
        fixture.verify(r, docker)
        self.assertTrue(all(c[1] == "inspect" for c in docker.calls))

    def reject(self, mutate):
        r, objects = sample()
        mutate(r, objects)
        docker = FakeDocker(objects)
        with self.assertRaises((ValueError, KeyError)):
            fixture.verify(r, docker)
        self.assertFalse(any(c[0] in ("run", "rm", "exec") for c in docker.calls))

    def test_hosted_redirectable_and_credential_endpoints_refused(self):
        for endpoint in ("https://example.invalid", "http://localhost:34567", "http://127.0.0.1:34567/proxy", "http://a:b@127.0.0.1:34567", "http://127.0.0.1:34567/?target=live"):
            with self.subTest(endpoint=endpoint):
                self.reject(lambda r, _: r.update(endpoint=endpoint))

    def test_existing_workspace_and_remote_docker_refused(self):
        self.reject(lambda r, _: r.update(workspace="dev"))
        self.reject(lambda r, _: r.update(docker_host="tcp://example.invalid:2375"))

    def test_floating_or_mismatched_image_source_and_version_refused(self):
        self.reject(lambda r, _: r.update(server_image="ghcr.io/juspay/superposition-demo:0.115.5"))
        self.reject(lambda r, o: o["container", r["server_container"]].update(Image="sha256:" + "0" * 64))
        self.reject(lambda r, o: o["image", r["server_image"]]["Config"].update(Labels={}))
        self.reject(lambda r, _: r.update(server_version="latest"))

    def test_wrong_owner_or_stopped_container_refused(self):
        self.reject(lambda r, o: o["container", r["server_container"]]["Config"].update(Labels={fixture.LABEL: "another-owner"}))
        self.reject(lambda r, o: o["container", r["server_container"]]["State"].update(Running=False))

    def test_mounts_privilege_and_external_network_refused(self):
        self.reject(lambda r, o: o["container", r["server_container"]].update(Mounts=[{"Source": "/shared"}]))
        self.reject(lambda r, o: o["container", r["server_container"]]["HostConfig"].update(Privileged=True))
        self.reject(lambda r, o: o["network", r["network"]].update(Internal=False))
        self.reject(lambda r, o: o["container", r["server_container"]]["NetworkSettings"]["Networks"].update(external={"NetworkID": "other"}))

    def test_unregistered_peer_or_exposed_port_refused(self):
        self.reject(lambda r, o: o["network", r["network"]]["Containers"].update(unowned={}))
        self.reject(lambda r, o: o["container", r["server_container"]]["NetworkSettings"]["Ports"]["8080/tcp"][0].update(HostIp="0.0.0.0"))
        self.reject(lambda r, o: o["container", r["redis_container"]]["NetworkSettings"].update(Ports={"6379/tcp": [{"HostIp": "127.0.0.1", "HostPort": "33333"}]}))

    def test_cleanup_only_exact_owned_resources_including_lost_create_response(self):
        for known_network in (True, False):
            r, objects = sample()
            docker = FakeDocker(objects)
            fixture.cleanup(docker, r["run_id"], r["network"] if known_network else None)
            self.assertEqual([c[-1] for c in docker.calls if c[0] == "rm"], [r["server_container"], r["redis_container"]])
            self.assertIn(("network", "rm", r["network"]), docker.calls)
            self.assertFalse(any("prune" in c or "compose" in c for c in docker.calls))

    def test_cleanup_refuses_unowned_container(self):
        r, objects = sample()
        objects["container", r["server_container"]]["Config"]["Labels"][fixture.LABEL] = "someone-else"
        docker = FakeDocker(objects)
        with self.assertRaises(ValueError):
            fixture.cleanup(docker, r["run_id"], r["network"])
        self.assertFalse(any(c[0] == "rm" for c in docker.calls))

    def test_unknown_registration_fields_do_not_supply_a_force_bypass(self):
        self.reject(lambda r, _: r.update(force=True))


if __name__ == "__main__":
    unittest.main()
