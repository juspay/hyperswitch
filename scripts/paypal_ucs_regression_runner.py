#!/usr/bin/env python3
"""Drive actual HS APIs and readback oracles; fixture evidence is never vendor verification.

Operator context contains resource IDs and environment variable NAMES only. Request
bodies, API keys, provider headers and credentials are resolved in memory. A completed
journal operation is reconciled by GET, never replayed. Final mode requires both real
committed runtimes, not merely binaries left in a target directory.
"""
import argparse
import ast
import datetime
import importlib.util
import re
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import socket
import socketserver
import threading
import select
import stat
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'cypress-tests/cypress/fixtures/paypal_ucs_regressions.json'
PROTECTED = ('2K728570VG9010042', '0SG65781FC8888712', '8HH928939S447812G', '29P14441XM896945V')
FORBIDDEN = {'creds.json', 'creds_dummy.json'}
SUFFIXES = {'.rs', '.toml', '.lock', '.proto', '.py', '.json', '.yaml', '.yml', '.sh'}


def digest(value):
    return hashlib.sha256(value).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def atomic(path, value):
    temporary = path.with_name(path.name + '.tmp')
    with open(temporary, 'w', opener=lambda p, f: os.open(p, f, 0o600)) as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], stderr=subprocess.DEVNULL)


def owning(path):
    item = Path(path)
    relevant = item.parts[0] in {'crates', 'config', 'data', 'scripts', 'cypress-tests', 'examples', '.cargo', '.github'} or path in {'Cargo.toml', 'Cargo.lock', 'build.rs', 'rust-toolchain.toml'}
    return (relevant and item.name not in FORBIDDEN and not any(part in {'target', 'grace', '.git', '__pycache__'}
            for part in item.parts) and (item.suffix in SUFFIXES or item.name == 'Cargo.lock'))


def manifest(repo):
    tracked = {p for p in git(repo, 'ls-files', '-z').decode().split('\0') if p and owning(p)}
    head = {p for p in git(repo, 'ls-tree', '-r', '--name-only', 'HEAD').decode().splitlines() if owning(p)}
    untracked = {p for p in git(repo, 'ls-files', '--others', '--exclude-standard', '-z').decode().split('\0') if p and owning(p)}
    # HEAD paths stay in the manifest even when deleted. Missing is not clean.
    paths = tracked | head | untracked
    files = {p: digest((repo / p).read_bytes()) if (repo / p).is_file() else None for p in sorted(paths)}
    dirty = git(repo, 'diff', 'HEAD', '--name-only', '-z').decode().split('\0')
    clean = not untracked and tracked == head and not any(p in paths for p in dirty) and all(files.values())
    return {'commit': git(repo, 'rev-parse', 'HEAD').decode().strip(), 'files': files,
            'sha256': digest(canonical(files)), 'committed': bool(clean)}


def listener(endpoint, binary):
    parsed = urllib.parse.urlsplit(endpoint if '://' in endpoint else 'http://' + endpoint)
    assert parsed.hostname in {'127.0.0.1', 'localhost'} and parsed.port
    output = subprocess.check_output(['ss', '-H', '-ltnp', 'sport = :' + str(parsed.port)],
        stderr=subprocess.DEVNULL).decode()
    pids = {int(pid) for pid in re.findall(r'pid=(\d+)', output)}
    matching = [pid for pid in pids if Path(f'/proc/{pid}/exe').exists()
        and Path(f'/proc/{pid}/exe').resolve() == binary
        and digest(Path(f'/proc/{pid}/exe').read_bytes()) == digest(binary.read_bytes())]
    assert len(matching) == 1, 'actual endpoint is not owned by exactly the supplied binary'
    return matching[0]


def runtime(repo, receipt_path, target, endpoint):
    receipt = json.loads(receipt_path.read_text())
    command = receipt['command']
    expected = ['cargo', 'build', '-p', 'router' if target == 'router' else 'grpc-server', '--bin', target]
    assert command[:6] == expected and command.count('--bin') == 1
    assert not any(flag in command for flag in ['--lib', '--bins', '--all-targets'])
    assert any(flag in command for flag in ['--message-format=json', '--message-format=json-render-diagnostics'])
    # The UCS committed consumer owns its already-established receipt schema;
    # use that actual verifier instead of inventing a compatible-looking wrapper.
    if target == 'grpc-server':
        module_path = repo / 'scripts/paypal_regression_runner.py'
        spec = importlib.util.spec_from_file_location('committed_ucs_receipt', module_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        # Locate the actual binary from the actual Cargo artifact log first.
        source = module.bindings()
        assert source['source_matches_commit']
    else:
        source = manifest(repo)
        assert source['committed'], 'owning source is uncommitted, missing or untracked'
        assert receipt['schema'] == 'hs-actual-build-receipt/1'
        assert receipt['producer'] == 'owned_actual_build'
        assert receipt['commit'] == source['commit']
        assert receipt['source_sha256_before'] == source['files']
        assert receipt['source_sha256_after'] == source['files']
    assert receipt['exit_code'] == 0 and isinstance(receipt['build_pid'], int) and receipt['build_pid'] > 0
    start, end = receipt['started_at_epoch'], receipt['finished_at_epoch']
    assert isinstance(start, (int, float)) and not isinstance(start, bool)
    assert isinstance(end, (int, float)) and not isinstance(end, bool) and start <= end <= time.time() + 5
    log = Path(receipt['build_log']).read_bytes()
    assert receipt['build_log_sha256'] == digest(log)
    events = []
    for line in log.splitlines():
        try:
            entry = json.loads(line)
        except (ValueError, UnicodeDecodeError):
            continue
        if isinstance(entry, dict):
            events.append(entry)
    artifacts = [entry for entry in events if entry.get('reason') == 'compiler-artifact'
        and entry.get('target', {}).get('name') == target
        and entry.get('target', {}).get('kind') == ['bin'] and entry.get('executable')
        and entry['executable'] in entry.get('filenames', [])]
    assert artifacts and any(entry.get('reason') == 'build-finished' and entry.get('success') is True for entry in events)
    assert all(Path(entry['executable']).is_absolute() and Path(entry['target']['src_path']).resolve().is_relative_to(repo.resolve()) for entry in artifacts)
    binaries = {Path(entry['executable']).resolve(strict=True) for entry in artifacts}
    assert len(binaries) == 1
    binary = binaries.pop()
    assert binary.stat().st_mtime <= end + 5
    assert digest(binary.read_bytes()) == receipt['binary_sha256']
    if target == 'grpc-server':
        source = module.bindings(binary)
        module.verify_build_receipt(receipt_path, source, binary)
        manifest_sha = digest(canonical(source['source_sha256']))
    else:
        manifest_sha = source['sha256']
    pid = listener(endpoint, binary)
    return {'source_commit': source['commit'], 'source_manifest_sha256': manifest_sha,
        'binary_sha256': receipt['binary_sha256'], 'pid': pid,
        'process_start_ticks': Path(f'/proc/{pid}/stat').read_text().split(') ', 1)[1].split()[19],
        'receipt_sha256': digest(receipt_path.read_bytes())}


def dependency_pin_provenance(repo, expected_commit):
    """Validate actual member declarations and lock before any financial request.

    All target stanzas are checked conservatively: an inactive declaration is
    never an exemption from the operator's requirement that every UCS pin agree.
    Returned proof contains public source/dependency identities only.
    """
    try:
        import tomllib
    except ImportError:
        import tomli as tomllib
    repository = 'https://github.com/juspay/hyperswitch-prism'
    expected = {
        'crates/router/Cargo.toml': {'unified-connector-service-client': 'rust-grpc-client',
                                   'unified-connector-service-cards': 'ucs_cards'},
        'crates/hyperswitch_interfaces/Cargo.toml': {'unified-connector-service-client': 'rust-grpc-client'},
        'crates/external_services/Cargo.toml': {'unified-connector-service-client': 'rust-grpc-client'},
    }
    locked_packages = {'rust-grpc-client', 'ucs_cards', 'config_patch_derive',
                       'grpc-api-types', 'ucs_common_enums', 'ucs_common_utils'}
    if not isinstance(expected_commit, str) or not re.fullmatch(r'[0-9a-f]{40}', expected_commit):
        raise ValueError('UCS source commit must be a full immutable Git SHA')
    declarations = []
    source_hashes = {}
    for member, required in expected.items():
        wire = (repo / member).read_bytes()
        source_hashes[member] = digest(wire)
        document = tomllib.loads(wire.decode())
        tables = [('dependencies', document.get('dependencies', {}))]
        targets = document.get('target', {})
        if not isinstance(targets, dict):
            raise ValueError('Invalid member target dependency tables')
        for target, settings in targets.items():
            if not isinstance(settings, dict):
                raise ValueError('Invalid target dependency settings')
            tables.append(('target.' + target + '.dependencies', settings.get('dependencies', {})))
        seen = set()
        for scope, dependencies in tables:
            if not isinstance(dependencies, dict):
                raise ValueError('Invalid member dependency table')
            for alias, item in dependencies.items():
                package = item.get('package', alias) if isinstance(item, dict) else alias
                git_source = item.get('git', '') if isinstance(item, dict) else ''
                candidate = alias in required or package in locked_packages or (
                    isinstance(git_source, str) and 'hyperswitch-prism' in git_source)
                if not candidate:
                    continue
                if (not isinstance(item, dict) or alias not in required
                        or package != required[alias] or git_source != repository
                        or item.get('rev') != expected_commit
                        or any(key in item for key in ('branch', 'tag', 'path', 'workspace', 'registry'))):
                    raise ValueError('Invalid or unexpected immutable UCS member dependency')
                seen.add(alias)
                declarations.append({'file': member, 'scope': scope, 'alias': alias,
                                     'package': package, 'repository': repository, 'rev': expected_commit})
        if seen != set(required):
            raise ValueError('Missing actual UCS member dependency declaration')
    wire = (repo / 'Cargo.lock').read_bytes()
    source_hashes['Cargo.lock'] = digest(wire)
    lock = tomllib.loads(wire.decode())
    locked = set()
    for package in lock.get('package', []):
        name, source = package.get('name'), package.get('source', '')
        candidate = 'hyperswitch-prism' in source or (name in locked_packages and source.startswith('git+'))
        if not candidate:
            continue
        parsed = urllib.parse.urlsplit(source)
        if (name not in locked_packages or parsed.scheme != 'git+https'
                or parsed.netloc != 'github.com' or parsed.path != '/juspay/hyperswitch-prism'
                or urllib.parse.parse_qs(parsed.query) != {'rev': [expected_commit]}
                or parsed.fragment != expected_commit):
            raise ValueError('Invalid or conflicting locked UCS source')
        locked.add(name)
    if not {'rust-grpc-client', 'ucs_cards'} <= locked:
        raise ValueError('Missing required locked UCS packages')
    return {'source_commit': expected_commit, 'declarations': declarations,
            'locked_packages': sorted(locked), 'source_sha256': source_hashes,
            'all_target_stanzas_validated': True}


def actual_route(source, ucs_endpoint):
    # Read only this allowlisted nonsecret setting, never print process environs.
    prefix = b'ROUTER__GRPC_CLIENT__UNIFIED_CONNECTOR_SERVICE__BASE_URL='
    settings = Path('/proc/' + str(source['hs']['pid']) + '/environ').read_bytes().split(b'\0')
    value = next((entry[len(prefix):].decode() for entry in settings if entry.startswith(prefix)), None)
    assert value, 'actual HS UCS route must be explicitly bound in its runtime environment'
    expected = urllib.parse.urlsplit(ucs_endpoint if '://' in ucs_endpoint else 'http://' + ucs_endpoint)
    actual = urllib.parse.urlsplit(value)
    if (actual.hostname, actual.port) != (expected.hostname, expected.port):
        # Fault runs may interpose this exact committed runner's loopback TCP
        # proxy, whose actual command and upstream listener are corroborated.
        control = os.environ['PAYPAL_HS_TRANSPORT_CONTROL']
        proof = fault_control(control)
        assert proof['runner_sha256'] == digest(Path(__file__).read_bytes())
        assert actual.hostname in {'127.0.0.1', 'localhost'} and actual.port
        output = subprocess.check_output(['ss', '-H', '-ltnp', 'sport = :' + str(actual.port)], stderr=subprocess.DEVNULL).decode()
        pids = {int(pid) for pid in re.findall(r'pid=(\d+)', output)}
        assert len(pids) == 1
        pid = pids.pop()
        command = Path(f'/proc/{pid}/cmdline').read_bytes().decode().split('\0')
        assert str(Path(__file__).resolve()) in command and '--serve-fault-proxy' in command
        position = command.index('--serve-fault-proxy')
        assert command[position + 1:position + 4] == [str(actual.port), str(expected.port), str(urllib.parse.urlsplit(control).port)]
        source['transport_proxy'] = {'pid': pid, 'runner_sha256': proof['runner_sha256']}
    source['dependency_pins'] = dependency_pin_provenance(ROOT, source['ucs']['source_commit'])



def at(value, pointer):
    for part in pointer.strip('/').split('/') if pointer != '/' else []:
        part = part.replace('~1', '/').replace('~0', '~')
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def resolve(value, bindings):
    if isinstance(value, str) and value.startswith('@'):
        return bindings[value[1:]]
    if isinstance(value, dict):
        return {key: resolve(item, bindings) for key, item in value.items()}
    if isinstance(value, list):
        return [resolve(item, bindings) for item in value]
    return value


def request(base, method, path, body, header_env, key_env, idempotency, response_identity=None):
    assert path.startswith('/') and not path.startswith('//') and '\n' not in path
    parsed = urllib.parse.urlsplit(base)
    assert parsed.scheme in {'http', 'https'} and parsed.hostname and not parsed.username
    headers = {'Content-Type': 'application/json', 'api-key': os.environ[key_env],
               'x-request-id': idempotency, 'idempotency-key': idempotency}
    if header_env:
        headers.update(json.loads(os.environ[header_env]))
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *_args, **_kwargs):
            return None
    opener = urllib.request.build_opener(NoRedirect())
    raw = canonical(body) if body is not None else None
    req = urllib.request.Request(base.rstrip('/') + path, data=raw, method=method, headers=headers)
    try:
        with opener.open(req, timeout=90) as response:
            status, wire, response_headers = response.status, response.read(), response.headers
    except urllib.error.HTTPError as error:
        status, wire, response_headers = error.code, error.read(), error.headers
    if response_identity is not None:
        # Ingress may replace the supplied x-request-id. Capture only the actual
        # public response UUID; never persist arbitrary headers or response text.
        identifiers = response_headers.get_all('x-request-id', [])
        if len(identifiers) == 1 and re.fullmatch(
                r'[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}', identifiers[0]):
            response_identity.update(request_id=identifiers[0], source='response_header:x-request-id')
    return status, json.loads(wire) if wire else None, digest(wire)


def check(value, expected):
    assert expected, 'a real state oracle is mandatory'
    for pointer, wanted in expected.items():
        actual = at(value, pointer)
        assert type(actual) is type(wanted) and actual == wanted, f'oracle mismatch at {pointer}'


class LogSinkUnavailable(Exception):
    """Actual HS sink is disabled/unobservable; this is NOT_RUN evidence."""


def snapshot_log_sinks(operator, source):
    sinks = operator['log_sinks']
    if set(sinks) != {'application', 'connector_event'}:
        raise LogSinkUnavailable('both actual HS sinks are required')
    snapshots = {}
    pid = source['hs']['pid']
    for name, config in sinks.items():
        # Offsets are derived here before the request. Operator offsets and an
        # arbitrary empty file can never turn an unobserved sink into evidence.
        if set(config) != {'path', 'fd'} or not isinstance(config['fd'], int) or config['fd'] < 0:
            raise LogSinkUnavailable('sink needs actual HS regular-file descriptor')
        path = Path(config['path']).resolve(strict=True)
        descriptor = Path(f'/proc/{pid}/fd/{config["fd"]}')
        try:
            actual, selected = descriptor.stat(), path.stat()
        except OSError as error:
            raise LogSinkUnavailable('actual HS sink is not observable') from error
        if not stat.S_ISREG(actual.st_mode) or (actual.st_dev, actual.st_ino) != (selected.st_dev, selected.st_ino):
            raise LogSinkUnavailable('sink is not the bound HS process output')
        snapshots[name] = {'path': path, 'fd': config['fd'], 'pid': pid,
            'device': actual.st_dev, 'inode': actual.st_ino, 'offset': actual.st_size}
    return snapshots


def tracing_json_value(value):
    """Decode serde_json::Value Debug without dropping object ancestry or eval."""
    if len(value) > 1024 * 1024:
        raise ValueError('bounded tracing value exceeded')
    decoder = json.JSONDecoder()
    cursor = 0

    def consume(token):
        nonlocal cursor
        while cursor < len(value) and value[cursor].isspace():
            cursor += 1
        if not value.startswith(token, cursor):
            raise ValueError('unexpected tracing structure')
        cursor += len(token)

    def literal():
        nonlocal cursor
        while cursor < len(value) and value[cursor].isspace():
            cursor += 1
        result, cursor = decoder.raw_decode(value, cursor)
        return result

    def parse(depth):
        nonlocal cursor
        if depth > 64:
            raise ValueError('bounded tracing depth exceeded')
        while cursor < len(value) and value[cursor].isspace():
            cursor += 1
        if value.startswith('Object', cursor):
            consume('Object')
            consume('{')
            result = {}
            while True:
                while cursor < len(value) and value[cursor].isspace():
                    cursor += 1
                if value.startswith('}', cursor):
                    consume('}')
                    return result
                if result:
                    consume(',')
                key = literal()
                if not isinstance(key, str) or key in result:
                    raise ValueError('ambiguous tracing key')
                consume(':')
                result[key] = parse(depth + 1)
        if value.startswith('Array', cursor):
            consume('Array')
            consume('[')
            result = []
            while True:
                while cursor < len(value) and value[cursor].isspace():
                    cursor += 1
                if value.startswith(']', cursor):
                    consume(']')
                    return result
                if result:
                    consume(',')
                result.append(parse(depth + 1))
        for wrapper, expected in [('String', str), ('Number', (int, float)), ('Bool', bool)]:
            if value.startswith(wrapper + '(', cursor):
                consume(wrapper + '(')
                result = literal()
                if not isinstance(result, expected) or (wrapper == 'Number' and isinstance(result, bool)):
                    raise ValueError('unexpected tracing scalar')
                consume(')')
                return result
        consume('Null')
        return None

    result = parse(0)
    if value[cursor:].strip():
        raise ValueError('trailing tracing structure')
    return result


def wallet_log_mode(body):
    """Select the oracle from the actual submitted wallet, never an operator label."""
    wallet = body['payment_method_data']['wallet']
    assert isinstance(wallet, dict) and len(wallet) == 1
    if 'apple_pay' in wallet:
        data = wallet['apple_pay']['payment_data']
        if isinstance(data, str) and data:
            return 'apple_psp'
        assert isinstance(data, dict) and data.get('application_primary_account_number')
        assert isinstance(data.get('payment_data'), dict) and data['payment_data'].get('online_payment_cryptogram')
        return 'apple_hs'
    assert set(wallet) == {'google_pay'}
    data = wallet['google_pay']['tokenization_data']
    assert isinstance(data, dict)
    if isinstance(data.get('token'), str) and data['token']:
        assert not data.get('application_primary_account_number') and not data.get('cryptogram')
        return 'google_psp'
    assert data.get('application_primary_account_number') and data.get('cryptogram') and 'token' not in data
    return 'google_hs'


def masked_wallet_field(value, wallet_mode, sink_type, ancestry=(), depth=0, require_typed_marker=False):
    assert wallet_mode in {'apple_psp', 'apple_hs', 'google_psp', 'google_hs'}
    assert sink_type in {'ApiLogs', 'ConnectorApiLogs'}
    markers = ('***', '*** alloc::string::String ***') if sink_type == 'ApiLogs' else ('***',)
    if require_typed_marker:
        assert sink_type == 'ApiLogs'
        markers = ('*** alloc::string::String ***',)
    if depth > 64:
        return False
    if isinstance(value, str):
        try:
            decoded = json.loads(value)
        except (ValueError, TypeError, RecursionError):
            try:
                decoded = tracing_json_value(value)
            except (ValueError, TypeError, RecursionError):
                return False
        return masked_wallet_field(decoded, wallet_mode, sink_type, ancestry, depth + 1, require_typed_marker)
    if isinstance(value, list):
        return any(masked_wallet_field(item, wallet_mode, sink_type, ancestry, depth + 1, require_typed_marker) for item in value)
    if isinstance(value, dict):
        for key, item in value.items():
            normalized = ''.join(c.lower() for c in key if c.isalnum())
            path = ancestry + (normalized,)
            # ApiLogs uses the public wallet shape; ConnectorApiLogs uses the
            # actual protobuf *_pay_sdk arm. Only that selected wallet's request
            # secret counts, never another wallet or an unrelated masked field.
            public_wallet = 'applepay' if wallet_mode.startswith('apple_') else 'googlepay'
            proto_wallet = 'applepaysdk' if wallet_mode.startswith('apple_') else 'googlepaysdk'
            prefixes = {('request', 'paymentmethoddata', 'wallet', public_wallet),
                ('request', 'paymentmethod', 'paymentmethod', proto_wallet)}
            expected = {
                'apple_psp': {('paymentdata',), ('paymentdata', 'paymentdata', 'encrypteddata')},
                # The actual sanitizer masks the entire payment_data subtree.
                # The submitted body's mode proves whether this is decrypted.
                'apple_hs': {('paymentdata',),
                    ('paymentdata', 'paymentdata', 'onlinepaymentcryptogram'),
                    ('paymentdata', 'paymentdata', 'decrypteddata', 'paymentdata', 'onlinepaymentcryptogram')},
                'google_psp': {('tokenizationdata', 'token'),
                    ('tokenizationdata', 'tokenizationdata', 'encrypteddata'),
                    ('tokenizationdata', 'tokenizationdata', 'encrypteddata', 'token')},
                'google_hs': {('tokenizationdata', 'cryptogram'),
                    ('tokenizationdata', 'tokenizationdata', 'decrypteddata', 'cryptogram')},
            }
            if isinstance(item, str) and item in markers and any(path == prefix + relative
                    for prefix in prefixes for relative in expected[wallet_mode]):
                return True
            if masked_wallet_field(item, wallet_mode, sink_type, path, depth + 1, require_typed_marker):
                return True
    return False


def verify_log_sinks(snapshots, response_identity, subject, canary, wallet_mode):
    assert canary and len(canary) >= 16
    if not response_identity.get('request_id'):
        raise LogSinkUnavailable('public HS response lacks an unambiguous generated request ID')
    request_id = response_identity['request_id']
    assert isinstance(subject, str) and re.fullmatch(r'[A-Za-z0-9_-]{1,128}', subject)
    correlation = [request_id.encode(), subject.encode()]
    observed = {}
    # HS may flush these asynchronously. Every read remains bounded and no raw
    # log line, credential or token is persisted or included in an exception.
    for _ in range(20):
        for name, snapshot in snapshots.items():
            actual = Path(f'/proc/{snapshot["pid"]}/fd/{snapshot["fd"]}').stat()
            selected = snapshot['path'].stat()
            identity = (snapshot['device'], snapshot['inode'])
            assert (actual.st_dev, actual.st_ino) == identity == (selected.st_dev, selected.st_ino)
            assert selected.st_size >= snapshot['offset'], 'sink shrank or rotated'
            if selected.st_size - snapshot['offset'] > 16 * 1024 * 1024:
                raise LogSinkUnavailable('actual HS sink correlation window exceeded bounded read')
            with snapshot['path'].open('rb') as stream:
                stream.seek(snapshot['offset'])
                wire = stream.read()
            assert canary not in wire, 'public inert canary leaked; content withheld'
            for line in wire.splitlines():
                if not all(value in line for value in correlation):
                    continue
                try:
                    record = json.loads(line)
                except (ValueError, UnicodeDecodeError):
                    continue
                if str(record.get('pid')) != str(snapshot['pid']):
                    continue
                extra = record.get('extra')
                if record.get('target') != 'router::events::event_logger' or not isinstance(extra, dict):
                    continue
                expected_type = 'ConnectorApiLogs' if name == 'connector_event' else 'ApiLogs'
                if extra.get('event_type') != expected_type or not masked_wallet_field(extra.get('event'), wallet_mode, expected_type):
                    continue
                observed[name] = {'device': snapshot['device'], 'inode': snapshot['inode'],
                    'offset': snapshot['offset'], 'end': selected.st_size,
                    'request_id': request_id, 'request_id_source': response_identity['source'],
                    'subject': subject, 'pid': snapshot['pid'], 'wallet_mode': wallet_mode,
                    'correlated_masked_entry_sha256': digest(line)}
        if len(observed) == 2:
            return observed
        time.sleep(.1)
    raise LogSinkUnavailable('correlated masked wallet entry missing from an actual HS sink')


def selected_active_attempt(payment, active_id, merchant_id, payment_id):
    assert payment['payment_id'] == payment_id and payment['merchant_id'] == merchant_id
    public_id = payment.get('active_attempt_id')
    if public_id is not None:
        assert isinstance(public_id, str) and public_id == active_id
    assert isinstance(active_id, str) and active_id
    rows = payment.get('attempts')
    if not isinstance(rows, list):
        raise GsmUnavailable('expanded public attempts are required for exact active-attempt selection')
    matches = [row for row in rows if row.get('attempt_id') == active_id]
    assert len(matches) == 1 and matches[0]['connector'] == 'paypal'
    return matches[0]


def owned_active_attempt(payment, operator, source):
    cfg = operator['provider']
    merchant, payment_id = cfg['merchant_id'], payment['payment_id']
    active_id = payment.get('active_attempt_id')
    receipt = None
    if active_id is None:
        # Credentials stay in the subprocess environment, never argv or artifacts.
        names = cfg.get('readonly_database_env')
        if not names or set(names) != {'PGHOST', 'PGPORT', 'PGDATABASE', 'PGUSER', 'PGPASSWORD'}:
            raise GsmUnavailable('public active-attempt pointer absent; owned read-only DB lookup is required')
        environment = {**os.environ, **{key: os.environ[name] for key, name in names.items()}}
        actual = dict(item.split(b'=', 1) for item in Path(f'/proc/{source["hs"]["pid"]}/environ').read_bytes().split(b'\0') if b'=' in item)
        for pg, hs in [('PGHOST', 'HOST'), ('PGPORT', 'PORT'), ('PGDATABASE', 'DBNAME'), ('PGUSER', 'USERNAME')]:
            assert environment[pg].encode() == actual[b'ROUTER__MASTER_DATABASE__' + hs.encode()]
        assert all(re.fullmatch(r'[A-Za-z0-9_-]{1,128}', item) for item in [merchant, payment_id])
        query = ("SELECT json_build_object('merchant_id',i.merchant_id,'payment_id',i.payment_id,"
            "'active_attempt_id',i.active_attempt_id,'merchant_connector_id',a.merchant_connector_id,"
            "'database',current_database(),'server',inet_server_addr()::text) "
            "FROM payment_intent i JOIN payment_attempt a ON a.attempt_id=i.active_attempt_id "
            "AND a.payment_id=i.payment_id AND a.merchant_id=i.merchant_id "
            "WHERE i.merchant_id='" + merchant + "' AND i.payment_id='" + payment_id + "'")
        environment['PGOPTIONS'] = '-c default_transaction_read_only=on'
        wire = subprocess.check_output(['psql', '--no-psqlrc', '-XAt', '-v', 'ON_ERROR_STOP=1', '-c', query], env=environment, stderr=subprocess.DEVNULL)
        rows = [json.loads(line) for line in wire.splitlines() if line]
        assert len(rows) == 1
        row = rows[0]
        assert row['merchant_id'] == merchant and row['payment_id'] == payment_id and row['merchant_connector_id'] == cfg['merchant_connector_id']
        active_id = row['active_attempt_id']
        receipt = {'query_sha256': digest(query.encode()), 'response_sha256': digest(wire),
            'database_identity_sha256': digest(canonical([row['database'], row['server']])),
            'observed_at_ns': time.time_ns(), 'hs_pid': source['hs']['pid'], 'source_commit': source['hs']['source_commit']}
    return selected_active_attempt(payment, active_id, merchant, payment_id), receipt


def provider_get(cfg, kind, resource_id):
    # No redirects, arbitrary paths or financial POSTs. OAuth alone uses POST.
    assert re.fullmatch(r'[A-Za-z0-9_-]{1,128}', resource_id)
    paths = {'order': '/v2/checkout/orders/', 'capture': '/v2/payments/captures/',
        'refund': '/v2/payments/refunds/', 'authorization': '/v2/payments/authorizations/'}
    assert kind in paths
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *_args, **_kwargs):
            return None
    opener = urllib.request.build_opener(NoRedirect())
    import base64
    credentials = (os.environ[cfg['client_id_env']] + ':' + os.environ[cfg['client_secret_env']]).encode()
    authorization = 'Basic ' + base64.b64encode(credentials).decode()
    auth = urllib.request.Request('https://api-m.sandbox.paypal.com/v1/oauth2/token', data=b'grant_type=client_credentials',
        headers={'Authorization': authorization, 'Content-Type': 'application/x-www-form-urlencoded'}, method='POST')
    with opener.open(auth, timeout=30) as response:
        assert response.status == 200
        token = json.load(response)['access_token']
    req = urllib.request.Request('https://api-m.sandbox.paypal.com' + paths[kind] + resource_id,
        headers={'Authorization': 'Bearer ' + token}, method='GET')
    with opener.open(req, timeout=30) as response:
        assert response.status == 200
        wire = response.read(2 * 1024 * 1024 + 1)
        assert len(wire) <= 2 * 1024 * 1024
    value = json.loads(wire)
    assert value['id'] == resource_id
    return value, digest(wire)


def provider_money(amount):
    from decimal import Decimal
    assert amount['currency_code'] == 'USD' and re.fullmatch(r'[0-9]+\.[0-9]{2}', amount['value'])
    return int(Decimal(amount['value']) * 100)


def _primary_route_observation_once(snapshots, source, identity, subject, merchant_id, flow):
    if not identity.get('request_id'):
        raise GsmUnavailable('actual generated HS request UUID is missing')
    for snapshot in snapshots.values():
        descriptor = Path(f'/proc/{snapshot["pid"]}/fd/{snapshot["fd"]}').stat()
        selected = snapshot['path'].stat()
        assert (descriptor.st_dev, descriptor.st_ino) == (selected.st_dev, selected.st_ino) == (snapshot['device'], snapshot['inode'])
        assert 0 <= selected.st_size - snapshot['offset'] <= 16 * 1024 * 1024
        with snapshot['path'].open('rb') as stream:
            stream.seek(snapshot['offset']);wire = stream.read(selected.st_size - snapshot['offset'])
        for line in wire.splitlines():
            record = json.loads(line)
            if str(record.get('pid')) != str(source['hs']['pid']) or record.get('target') != 'router::events::event_logger':
                continue
            extra = record.get('extra', {})
            if extra.get('event_type') != 'ConnectorApiLogs':
                continue
            event = extra.get('event')
            event = json.loads(event) if isinstance(event, str) and event.lstrip().startswith('{') else tracing_json_value(event)
            if event.get('request_id') != identity['request_id'] or event.get('flow') != flow:
                continue
            assert event.get('payment_id') == subject and event.get('merchant_id') == merchant_id
            assert event.get('destination') == 'unified_connector_service' and event.get('execution_mode') == 'primary'
            assert event.get('url') == 'grpc://unified-connector-service' and event.get('connector_name') == 'paypal'
            return {'request_id': identity['request_id'], 'flow': flow, 'record_sha256': digest(line),
                'hs_pid': source['hs']['pid'], 'source_sha256': digest(canonical(source))}
    raise GsmUnavailable('current subject/UUID-correlated primary UCS route is not observable')


def primary_route_observation(snapshots, source, identity, subject, merchant_id, flow):
    # Event emission is asynchronous. Retry only bounded read-only sink reads;
    # a wrong/Direct event remains an immediate failure, never a payment retry.
    deadline = time.monotonic() + 2
    while True:
        try:
            return _primary_route_observation_once(snapshots, source, identity, subject, merchant_id, flow)
        except GsmUnavailable:
            if time.monotonic() >= deadline:
                raise
            time.sleep(.05)


def provider_state_observation(operator, payment, source, snapshots, identity, flow, refund_state=None):
    cfg = operator['provider']
    route = primary_route_observation(snapshots, source, identity, payment['payment_id'], cfg['merchant_id'], flow)
    attempt, db_receipt = owned_active_attempt(payment, operator, source)
    meta = attempt.get('connector_metadata')
    assert isinstance(meta, dict) and meta.get('order_id')
    order_id = meta['order_id']
    order, order_sha = provider_get(cfg, 'order', order_id)
    units = order.get('purchase_units', [])
    assert len(units) == 1
    unit = units[0]
    expected = cfg['expected']
    payments = unit.get('payments')
    inventory_present = isinstance(payments, dict) and 'captures' in payments
    if inventory_present and not isinstance(payments['captures'], list):
        raise GsmUnavailable('provider capture inventory is null or malformed, not a complete empty list')
    inventory_known = inventory_present
    captures = payments['captures'] if inventory_known else []
    authorizations = payments.get('authorizations', []) if isinstance(payments, dict) else []
    if not inventory_known and (not expected.get('authorization_status')
            or expected['capture_amounts'] or expected['final_capture_flags']):
        raise GsmUnavailable('provider omitted capture inventory; an authorization hold cannot certify complete history')
    capture_ids = [row['id'] for row in captures]
    assert len(capture_ids) == len(set(capture_ids))
    amounts = [provider_money(row['amount']) for row in captures]
    for row in captures:
        captured, _ = provider_get(cfg, 'capture', row['id'])
        assert captured['status'] == row['status'] and captured['amount'] == row['amount']
        assert captured.get('supplementary_data', {}).get('related_ids', {}).get('order_id') == order_id
    assert provider_money(unit['amount']) == expected['order_amount']
    assert order['status'] == expected['order_status']
    assert sorted(amounts) == sorted(expected['capture_amounts'])
    assert all(row['status'] == 'COMPLETED' for row in captures)
    assert [row['final_capture'] for row in captures] == expected['final_capture_flags']
    public_rows = payment.get('captures') or []
    assert isinstance(public_rows, list)
    # Public CaptureResponse fields come from the owned payment's stored capture
    # row (types/transformers.rs CaptureResponse conversion). Only explicit
    # current-attempt, charged, exact-ID/currency/Money matches certify a row.
    # Unrelated/pending reservations are neither summed nor rejected as captures.
    matched_ids = []
    for cid, amount in zip(capture_ids, amounts):
        matches = [row for row in public_rows if row.get('connector_capture_id') == cid
            and row.get('status') == 'charged' and row.get('authorized_attempt_id') == attempt['attempt_id']
            and row.get('currency') == 'USD' and type(row.get('amount')) is int and row['amount'] == amount]
        assert len(matches) <= 1, 'duplicate local rows cannot corroborate one provider capture'
        if matches:
            matched_ids.append(cid)
    rows_corroborated = bool(capture_ids) and set(matched_ids) == set(capture_ids)
    evidence = {'route': route, 'active_attempt_id': attempt['attempt_id'], 'active_lookup': db_receipt,
        'order_id': order_id, 'order_response_sha256': order_sha,
        'capture_inventory': 'PRESENT_LIST' if inventory_known else 'ABSENT_AUTHORIZATION_HOLD_ONLY',
        'capture_ids': capture_ids if inventory_known else None,
        'capture_amounts': amounts if inventory_known else None,
        'captured_gross': sum(amounts) if inventory_known else None, 'provider_method': 'GET',
        'complete_order_history': inventory_known, 'no_pending_reservation_counted': inventory_known,
        'matched_charged_owned_capture_ids': matched_ids,
        'capture_operation_money_corroborated': rows_corroborated,
        'actual_capture_reference_owned_row': rows_corroborated,
        'two_distinct_capture_ids': inventory_known and len(capture_ids) == 2 and len(set(capture_ids)) == 2,
        'captured_amounts': amounts if inventory_known else None,
        'aggregate_gross': sum(amounts) if inventory_known else None, 'provider_state_verified': True,
        'vendor_wire_verified': False}
    if expected.get('authorization_status'):
        assert len(authorizations) == 1 and authorizations[0]['id'] == meta['authorize_id']
        auth, sha = provider_get(cfg, 'authorization', meta['authorize_id'])
        assert auth['status'] == expected['authorization_status'] and provider_money(auth['amount']) == expected['order_amount']
        evidence['authorization_response_sha256'] = sha
    if expected.get('refund'):
        refund_spec = expected['refund']
        assert refund_state and refund_state['payment_id'] == payment['payment_id']
        refund_id = refund_state.get('connector_refund_id')
        assert refund_id == refund_spec['id']
        refund, sha = provider_get(cfg, 'refund', refund_id)
        assert refund['status'] == refund_spec['status'] and provider_money(refund['amount']) == refund_spec['amount']
        parent_links = [link['href'] for link in refund.get('links', []) if link.get('rel') == 'up']
        assert parent_links == ['https://api-m.sandbox.paypal.com/v2/payments/captures/' + refund_spec['capture_id']]
        assert refund_spec['capture_id'] in capture_ids
        evidence['refund_response_sha256'] = sha
    return evidence


def first_authorize_config_preflight(operator, source, base, token, body):
    """Source/config evidence before creation; never claim an actual route trace."""
    cfg = operator['provider']
    subject = operator['bindings']['payment_id']
    assert re.fullmatch(r'[A-Za-z0-9_-]{1,128}', subject)
    assert body['payment_id'] == subject and body['merchant_id'] == cfg['merchant_id']
    assert body['profile_id'] == cfg['profile_id'] and body['currency'] == 'USD'
    assert type(body['amount']) is int and body['amount'] == cfg['expected']['order_amount']
    assert body['capture_method'] == cfg['capture_method'] and body['capture_method'] in {'automatic', 'manual', 'manual_multiple'}
    assert body['confirm'] is True and body['payment_method'] == 'card'
    assert body['routing'] == {'type': 'single', 'data': {'connector': 'paypal', 'merchant_connector_id': cfg['merchant_connector_id']}}
    merchant = cfg['merchant_id']
    status, account, _ = request(base, 'GET', '/accounts/' + merchant, None, None, cfg['admin_key_env'], token)
    assert status == 200 and account['merchant_id'] == merchant
    org = account['organization_id']
    assert re.fullmatch(r'[A-Za-z0-9_-]{1,128}', org)
    status, enabled, _ = request(base, 'GET', '/configs/ucs_enabled', None, None, cfg['admin_key_env'], token)
    assert status == 200 and enabled['key'] == 'ucs_enabled' and enabled['value'] == 'true'
    # Exact highest-to-lowest precedence from build_rollout_keys_by_precedence.
    scope = merchant + '_paypal_card_Authorize'
    keys = ['ucs_rollout_config_' + scope, 'ucs_rollout_config_' + org + '_' + scope,
        'ucs_rollout_config_' + org + '_' + merchant, 'ucs_rollout_config_' + org]
    effective = None
    for key in keys:
        status, value, _ = request(base, 'GET', '/configs/' + key, None, None, cfg['admin_key_env'], token)
        if status == 404:
            continue
        assert status == 200 and value['key'] == key
        effective = json.loads(value['value'])
        assert effective['execution_mode'] == 'primary' and effective['rollout_percent'] == 1.0
        assert effective.get('kill_switch_enabled', False) is False
        break
    if effective is None:
        raise GsmUnavailable('effective deterministic Primary Authorize config is absent')
    # Source-backed config may still differ from a live cached decision. The
    # actual post-call ConnectorApiLogs Primary event is mandatory independently.
    status, _, absence_sha = request(base, 'GET', '/payments/' + subject, None, None,
        operator.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
    assert status == 404, 'first Authorize subject already exists or absence is unproven'
    return {'phase': 'NEW_SUBJECT_CONFIG_PREFLIGHT_ONLY', 'actual_route_verified': False,
        'subject': subject, 'merchant_id': merchant, 'profile_id': cfg['profile_id'],
        'merchant_connector_id': cfg['merchant_connector_id'], 'body_sha256': digest(canonical(body)),
        'effective_config_sha256': digest(canonical(effective)), 'absence_response_sha256': absence_sha,
        'source_sha256': digest(canonical(source)), 'observed_at_ns': time.time_ns()}


def provider_preflight(operator, source, base, token, first_body=None):
    cfg = operator.get('provider')
    if not cfg:
        raise GsmUnavailable('genuine owned provider state/route configuration is required')
    environment = dict(item.split(b'=', 1) for item in Path(f'/proc/{source["ucs"]["pid"]}/environ').read_bytes().split(b'\0') if b'=' in item)
    assert environment.get(b'CS__CONNECTORS__PAYPAL__BASE_URL') == b'https://api-m.sandbox.paypal.com/'
    # The mask precondition applies to financial cases independently of GSM.
    applicable_wallet_mask_preflight(cfg['wallet_mask_receipts'], source, cfg['wallet_mask_config'])
    merchant, mca = cfg['merchant_id'], cfg['merchant_connector_id']
    assert all(re.fullmatch(r'[A-Za-z0-9_-]{1,128}', item) for item in [merchant, mca])
    status, account, _ = request(base, 'GET', '/account/' + merchant + '/connectors/' + mca,
        None, None, cfg['admin_key_env'], token)
    assert status == 200 and account['merchant_connector_id'] == mca and account['connector_name'] == 'paypal'
    assert account['test_mode'] is True and account.get('disabled') is not True
    assert account['profile_id'] == cfg['profile_id']
    credentials = account.get('connector_account_details')
    if not isinstance(credentials, dict) or credentials.get('auth_type') != 'BodyKey':
        raise GsmUnavailable('owned standard PayPal MCA credentials cannot be corroborated in memory')
    assert credentials.get('key1') == os.environ[cfg['client_id_env']]
    assert credentials.get('api_key') == os.environ[cfg['client_secret_env']]
    if first_body is not None:
        return first_authorize_config_preflight(operator, source, base, token, first_body)
    pre_path = cfg['route_preflight_path']
    query = urllib.parse.parse_qs(urllib.parse.urlsplit(pre_path).query)
    assert query.get('force_sync') == ['true'] and query.get('expand_attempts') == ['true']
    snapshots = snapshot_log_sinks(operator, source)
    identity = {}
    status, payment, _ = request(base, 'GET', pre_path, None, None,
        operator.get('api_key_env', 'HYPERSWITCH_API_KEY'), token, identity)
    assert status == 200
    assert payment['payment_id'] == cfg['route_preflight_payment_id']
    pre_operator = {**operator, 'provider': {**cfg, 'expected': cfg['pre_expected']}}
    evidence = provider_state_observation(pre_operator, payment, source, snapshots, identity, 'PSync')
    return evidence


def run_case(case, context, journal, journal_path, source, base):
    operator = context['cases'][case['id']]
    bindings = operator['bindings']
    for field in case['positive_money']:
        assert isinstance(bindings[field], int) and not isinstance(bindings[field], bool) and bindings[field] > 0
    assert operator.get('kind') in {'provider_live', 'production_fixture'}
    if case.get('log_canary'):
        assert operator['kind'] == 'production_fixture'
        assert len(os.environ['PAYPAL_HS_PUBLIC_CANARY']) >= 16
    results = []
    for index, step in enumerate(case['steps']):
        cfg = operator['steps'][index]
        if operator['kind'] == 'provider_live':
            original_provider = context['cases'][case['id']].get('provider')
            if not original_provider:
                raise GsmUnavailable('genuine owned provider configuration is absent')
            operator = {**operator, 'provider': {**original_provider,
                'expected': cfg.get('provider_expected', original_provider['expected']),
                'pre_expected': cfg.get('provider_pre_expected', original_provider.get('pre_expected'))}}
        method = step['method']
        path = cfg['path']
        body = json.loads(os.environ[cfg['body_env']]) if cfg.get('body_env') else None
        if step.get('require_force_sync'):
            assert method == 'GET' and urllib.parse.parse_qs(urllib.parse.urlsplit(path).query).get('force_sync') == ['true']
        if case.get('protected_readonly'):
            assert method == 'GET' and body is None
            assert bindings['provider_order_id'] == PROTECTED[1]
        if method != 'GET':
            # Also reject the protected resource in body, regardless of case ID.
            wire = canonical(body).decode() + path
            assert all(identifier not in wire for identifier in (*PROTECTED, *context['protected_hs_payment_ids'])), 'protected capture cannot be replayed'
        expected = resolve(step['expect'], bindings)
        identity = {'method': method, 'path': path, 'body_sha256': digest(canonical(body)),
                    'case': case['id'], 'step': index, 'source': source}
        operation = case['id'] + ':' + str(index)
        saved = journal['operations'].get(operation)
        if saved:
            assert {key: value for key, value in saved['identity'].items() if key != 'source'} == {key: value for key, value in identity.items() if key != 'source'}, 'journal request identity changed'
            # Any response-lost or completed write is irreversible from this runner.
            # Reconcile by GET below; no second financial request is allowed.
            if method != 'GET':
                read_status, state, wire_sha = request(base, 'GET', cfg['reconcile_path'], None,
                    None, cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), saved['idempotency'])
                assert read_status == 200
                try:
                    check(state, resolve(step['readback_expect'], bindings))
                except AssertionError:
                    check(state, resolve(step['resume_readback_expect'], bindings))
                reconcile_evidence = None
                if operator['kind'] == 'provider_live':
                    read_path = cfg.get('payment_reconcile_path', cfg['reconcile_path'])
                    parts = urllib.parse.urlsplit(read_path)
                    query = urllib.parse.parse_qs(parts.query)
                    query.update(force_sync=['true'], expand_attempts=['true'], expand_captures=['true'])
                    route_path = parts.path + '?' + urllib.parse.urlencode(query, doseq=True)
                    provider_cfg = {**operator['provider'], 'route_preflight_path': route_path,
                        'route_preflight_payment_id': bindings['payment_id'], 'pre_expected': operator['provider']['expected']}
                    reconcile_evidence = provider_preflight({**operator, 'provider': provider_cfg}, source, base, saved['idempotency'])
                results.append({'state': 'READ_ONLY_RECONCILED', 'operation': operation,
                                'reconciled_provider_state': reconcile_evidence,
                                'wire_sha256': wire_sha, 'cached_not_current_verification': True, 'current_mapping_verified': False,
                                'provider_verification': False, 'original_provenance': saved.get('result', {}),
                                'original_identity_sha256': digest(canonical(saved['identity']))})
                continue
        token = saved['idempotency'] if saved else 'paypal-hs-' + digest(canonical(identity))[:32]
        payment_before = None
        if case.get('separate_payment_money'):
            before_status, payment_before, _ = request(base, 'GET', cfg['payment_reconcile_path'], None, None,
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            assert before_status == 200 and payment_before['payment_id'] == bindings['payment_id']
        if case.get('retain_refund_state'):
            before_status, before, _ = request(base, 'GET', cfg['reconcile_path'], None, None,
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            assert before_status == 200
            check(before, resolve(step['readback_expect'], bindings))
        if case.get('retain_prior_money'):
            before_status, before, _ = request(base, 'GET', cfg['reconcile_path'], None, None,
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            assert before_status == 200
            assert before['amount_received'] == bindings['received']
            assert before['amount_capturable'] == bindings['capturable']
            assert bindings['retained_status'] in {before['status'], 'processing', 'pending'}
        provider_before_state = None
        if operator['kind'] == 'provider_live':
            if case['id'] == 'zero_auth':
                raise GsmUnavailable('Vault availability requires a distinct nonpayment oracle; Orders state cannot verify zero authorization')
            supported_proof = {'capture_ids', 'capture_amounts', 'no_pending_reservation_counted',
                'capture_operation_money_corroborated', 'actual_capture_reference_owned_row',
                'provider_method', 'complete_order_history', 'two_distinct_capture_ids', 'captured_amounts', 'aggregate_gross'}
            if case.get('proof') and not set(case['proof']) <= supported_proof:
                raise GsmUnavailable('this scenario requires genuine additional device/buyer/webhook/fault observations before a new payment write')
            provider_before_state = provider_preflight(operator, source, base, token,
                first_body=body if method == 'POST' and path == '/payments' else None)
        route_snapshots = snapshot_log_sinks(operator, source) if operator['kind'] == 'provider_live' else None
        log_mode = wallet_log_mode(body) if case.get('log_canary') else None
        log_snapshots = snapshot_log_sinks(operator, source) if case.get('log_canary') else None
        provider_before = provider_observation_snapshot(source) if case.get('log_canary') else None
        if step.get('precondition'):
            precondition_path = cfg['precondition_path']
            assert urllib.parse.parse_qs(urllib.parse.urlsplit(precondition_path).query).get('force_sync') != ['true']
            pre_status, pre_state, _ = request(base, 'GET', precondition_path, None, None,
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            assert pre_status == 200
            if step.get('precondition_scope') == 'selected_active_attempt':
                pre_attempt, _ = owned_active_attempt(pre_state, operator, source)
                assert provider_before_state is None or pre_attempt['attempt_id'] == provider_before_state['active_attempt_id']
                resolved_precondition = resolve(step['precondition'], bindings)
                check(pre_attempt, {key: value for key, value in resolved_precondition.items() if key.startswith('/connector_metadata/')})
                public_conditions = {key: value for key, value in resolved_precondition.items() if not key.startswith('/connector_metadata/')}
                if public_conditions:
                    check(pre_state, public_conditions)
            else:
                check(pre_state, resolve(step['precondition'], bindings))
        if provider_before_state and method != 'GET' and path != '/payments':
            assert provider_before_state['order_id'] == operator['provider']['expected_order_id']
            assert operator['provider']['route_preflight_payment_id'] == bindings['payment_id']
        journal['operations'][operation] = {'identity': identity, 'idempotency': token, 'state': 'sending',
            'provider_before': provider_before_state,
            'subject': bindings['payment_id'], 'active_attempt_id': provider_before_state.get('active_attempt_id') if provider_before_state else None}
        atomic(journal_path, journal)
        transport_proof = None
        response_identity = {}
        if case.get('transport_fault'):
            fault_control(os.environ['PAYPAL_HS_TRANSPORT_CONTROL'], {'armed': True, 'operation': token})
        try:
            status, response, wire_sha = request(base, method, path, body, cfg.get('header_env'),
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token, response_identity)
        finally:
            if case.get('transport_fault'):
                transport_proof = fault_control(os.environ['PAYPAL_HS_TRANSPORT_CONTROL'])
                fault_control(os.environ['PAYPAL_HS_TRANSPORT_CONTROL'], {'armed': False, 'operation': token})
                assert transport_proof['fault_injected'] and transport_proof['operation'] == token

        observation_unavailable = False
        journal['operations'][operation]['http_response_observed'] = {
            'status': status, 'wire_sha256': wire_sha, 'request_id': response_identity.get('request_id')}
        atomic(journal_path, journal)
        if log_snapshots is not None:
            try:
                # Only unavailable observation proof is deferred. Security/mask
                # assertions remain immediate failures; no raw error is retained.
                time.sleep(.2)
                observation = collect_wallet_preflight_observation(log_snapshots, source, log_mode,
                    bindings['payment_id'], response_identity, provider_before, 'PAYPAL_HS_PUBLIC_CANARY')
                observation_path = journal_path.with_name(journal_path.stem + '.' + digest(operation.encode())[:16] + '.wallet-observation.json')
                atomic(observation_path, observation)
                journal['operations'][operation]['wallet_preflight_observation'] = str(observation_path)
            except GsmUnavailable:
                observation_unavailable = True
                journal['operations'][operation]['wallet_preflight_observation_state'] = 'MISSING_SOURCE_PROOF'
            atomic(journal_path, journal)
        assert status in step['http_status'], 'unexpected HTTP result; body withheld'
        if step['expect'] == {'/': None}:
            assert wire_sha == digest(b''), 'expected actual empty webhook ACK body'
        check(response, expected)
        read_status, state, read_sha = request(base, 'GET', cfg['reconcile_path'], None, None,
            cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
        assert read_status == 200
        check(state, resolve(step['readback_expect'], bindings))
        if payment_before is not None:
            money_status, payment_after, _ = request(base, 'GET', cfg['payment_reconcile_path'], None, None,
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            assert money_status == 200
            for field in ['payment_id', 'amount_received', 'amount_capturable']:
                assert payment_after[field] == payment_before[field]
        if step.get('capture_rows_oracle'):
            expected_rows = step['capture_rows_oracle']
            rows = state['captures']
            assert sorted(row['amount'] for row in rows) == sorted(expected_rows['amounts'])
            ids = [row['connector_capture_id'] for row in rows]
            assert all(isinstance(value, str) and value and value != bindings['provider_order_id'] for value in ids)
            assert len(set(ids)) == len(ids)
            assert all(row['status'] == 'charged' and row['currency'] == expected_rows['currency']
                and row['authorized_attempt_id'] == bindings['attempt_id'] for row in rows)
        proof = None
        provider_evidence = None
        if operator['kind'] == 'provider_live':
            cfg_provider = operator['provider']
            provider_payment = state
            if case['group'] in {'Refund', 'RSync'}:
                payment_status, provider_payment, _ = request(base, 'GET', cfg['payment_reconcile_path'], None, None,
                    cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
                assert payment_status == 200
            flow = 'PSync' if method == 'GET' and case['group'] not in {'Refund', 'RSync'} else {'Refund': 'Execute'}.get(case['group'], case['group'])
            provider_evidence = provider_state_observation(operator, provider_payment, source, route_snapshots,
                response_identity, flow, refund_state=state if case['group'] in {'Refund', 'RSync'} else None)
        if case.get('proof') and index == case.get('proof_after_step', index):
            # Local fixture/notification collector supplies actual wire observations,
            # not an authored expected-result file. Proof endpoint is read-only.
            if operator['kind'] == 'provider_live':
                proof = provider_evidence
                needed = resolve(case['proof'], bindings)
                if needed.get('complete_order_history') is True and proof.get('complete_order_history') is not True:
                    raise GsmUnavailable('provider capture-history completeness is not established')
                if not set(needed) <= set(proof):
                    raise GsmUnavailable('this scenario needs additional genuine device/buyer/webhook or fault observations')
                check(proof, needed)
            elif transport_proof is None:
                proof_url = urllib.parse.urlsplit(os.environ['PAYPAL_HS_PROOF_BASE'])
                assert proof_url.hostname in {'127.0.0.1', 'localhost', '::1'}
            if operator['kind'] == 'provider_live':
                pass
            elif transport_proof is not None:
                proof = transport_proof
            else:
                _, proof, _ = request(os.environ['PAYPAL_HS_PROOF_BASE'], 'GET', cfg['proof_path'],
                    None, None, cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            check(proof, resolve(case['proof'], bindings))
            if operator['kind'] != 'provider_live':
                assert proof['operation'] == token
        vendor_wire_verified = False
        log_evidence = None
        if log_snapshots is not None:
            assert operator['kind'] == 'production_fixture', 'log canary uses public inert fixture only'
            log_evidence = verify_log_sinks(log_snapshots, response_identity, bindings['payment_id'],
                os.environ['PAYPAL_HS_PUBLIC_CANARY'].encode(), log_mode)
            assert proof and proof['inert_wallet_canary_received'] is True
        if observation_unavailable:
            raise GsmUnavailable('payment oracles passed, but current wallet source observation proof is unavailable')
        result = {'operation': operation, 'state': 'ORACLE_PASSED_PENDING_RUNTIME_RECHECK',
                  'http_status': status, 'wire_sha256': wire_sha, 'readback_sha256': read_sha,
                  'provider_verification': False, 'requested_evidence_kind': operator['kind'],
                  'current_mapping_verified': False, 'vendor_wire_verified': vendor_wire_verified,
                  'provider_state_verified': bool(provider_evidence), 'provider_state_observation': provider_evidence,
                  'log_sink_evidence': log_evidence,
                  'unresolved_capture_rows': [{'capture_id': row.get('capture_id'), 'preserved_status': row.get('status')}
                      for row in (state.get('captures') or []) if not row.get('connector_capture_id')
                        or row.get('connector_capture_id') not in (((state.get('connector_metadata') or {}).get('capture_context') or {}).get('capture_amounts') or {})] if isinstance(state, dict) else [],
                  'aggregate_oracle_does_not_certify_all_local_rows': True}
        journal['operations'][operation].update(state='completed', result=result)
        atomic(journal_path, journal)
        results.append(result)
    return results


class GsmUnavailable(Exception):
    """A required real-engine/owned-database precondition is absent: NOT_RUN."""


GSM_KEY_FIELDS = ('connector', 'flow', 'sub_flow', 'code', 'message')
GSM_MESSAGES = {
    '9500': 'SUSPECTED_FRAUD. Try using another card. Do not retry the same card.',
    '5180': 'INVALID_OR_RESTRICTED_CARD. Try using another card. Do not retry the same card.',
    '9520': 'LOST_OR_STOLEN. Try using another card. Do not retry the same card.',
}


def gsm_key(row):
    return {field: row[field] for field in GSM_KEY_FIELDS}


def gsm_policy():
    asset = json.loads((ROOT / 'config/gsm/paypal.json').read_text())
    rows = asset['rules']
    assert len(rows) == 6 and len({digest(canonical(gsm_key(row))) for row in rows}) == 6
    fields = set(GSM_KEY_FIELDS) | {'status', 'router_error', 'decision', 'step_up_possible',
        'unified_code', 'unified_message', 'error_category', 'clear_pan_possible', 'feature',
        'feature_data', 'standardised_code', 'description', 'user_guidance_message'}
    for row in rows:
        assert set(row) == fields and row['status'] == 'failure'
        assert all(isinstance(row[field], str) and row[field] for field in GSM_KEY_FIELDS)
        assert all(row[field] is None or isinstance(row[field], str) for field in
            ['router_error', 'unified_code', 'unified_message', 'description', 'user_guidance_message'])
        assert row['error_category'] is None and row['standardised_code'] is None
        assert row['connector'] == 'paypal' and row['flow'] == 'Payment' and row['sub_flow'] == 'Authorize'
        code = row['code'].removeprefix('Network:Visa|IssuerCode:')
        assert code in GSM_MESSAGES
        issuer = row['code'].startswith('Network:Visa|IssuerCode:')
        assert row['message'] == (row['code'] if issuer else GSM_MESSAGES[code])
        assert row['decision'] == 'do_default' and row['step_up_possible'] is False and row['clear_pan_possible'] is False
        assert row['feature'] == 'retry' and row['feature_data'] == {'retry': {
            'decision': 'do_default', 'step_up_possible': False,
            'clear_pan_possible': False, 'alternate_network_possible': False}}
    return rows


def observed_window(window, pid):
    """Re-read actual owned descriptor bytes; result labels cannot replace origin."""
    if not isinstance(window, dict) or not {'pid', 'fd', 'path', 'device', 'inode', 'offset', 'end', 'sha256'} <= set(window):
        raise GsmUnavailable('direct owned origin descriptor/window observations are unavailable')
    assert window['pid'] == pid and type(window['fd']) is int
    descriptor = Path(f'/proc/{pid}/fd/{window["fd"]}')
    path = Path(window['path']).resolve(strict=True)
    actual, selected = descriptor.stat(), path.stat()
    identity = (window['device'], window['inode'])
    assert stat.S_ISREG(actual.st_mode) and (actual.st_dev, actual.st_ino) == identity == (selected.st_dev, selected.st_ino)
    assert type(window['offset']) is int and type(window['end']) is int
    assert 0 <= window['offset'] < window['end'] <= selected.st_size
    assert window['end'] - window['offset'] <= 16 * 1024 * 1024
    with path.open('rb') as stream:
        stream.seek(window['offset'])
        wire = stream.read(window['end'] - window['offset'])
    assert digest(wire) == window['sha256']
    return wire, [json.loads(line) for line in wire.splitlines() if line]


# Exact already-inspected existing inert HTTP handler implementation. No new
# producer is introduced; another implementation needs its own source review.
INERT_PROVIDER_OBSERVER_SHA256 = 'a48a197f512a2a569ae7612c2ad864fb5689a02ad0d040dbb143648c9d6d554f'


def provider_observation_snapshot(source):
    base = urllib.parse.urlsplit(os.environ['PAYPAL_HS_PROOF_BASE'])
    assert base.scheme == 'http' and base.hostname in {'127.0.0.1', 'localhost'} and base.port
    assert base.username is None and base.password is None and not base.query and not base.fragment and base.path in {'', '/'}
    environment = Path(f'/proc/{source["ucs"]["pid"]}/environ').read_bytes().split(b'\0')
    prefix = b'CS__CONNECTORS__PAYPAL__BASE_URL='
    actual_url = next((entry[len(prefix):].decode() for entry in environment if entry.startswith(prefix)), '')
    actual = urllib.parse.urlsplit(actual_url)
    assert (actual.scheme, actual.hostname, actual.port) == (base.scheme, base.hostname, base.port)
    listeners = subprocess.check_output(['ss', '-H', '-ltnp', 'sport = :' + str(base.port)], stderr=subprocess.DEVNULL).decode()
    pids = {int(pid) for pid in re.findall(r'pid=(\d+)', listeners)}
    assert len(pids) == 1
    pid = pids.pop()
    command = Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
    scripts = [Path(arg.decode()).resolve() for arg in command if arg.endswith(b'.py')]
    trusted = [path for path in scripts if path.is_file() and digest(path.read_bytes()) == INERT_PROVIDER_OBSERVER_SHA256]
    if len(trusted) != 1:
        raise GsmUnavailable('actual provider HTTP observer source has not been reviewed for this existing format')
    # The pinned implementation's literal is an inert public canary, not a
    # credential. Compare hashes only; never persist or print its value.
    tree = ast.parse(trusted[0].read_text())
    canary_node = next(node.value for node in tree.body if isinstance(node, ast.Assign)
        and any(isinstance(target, ast.Name) and target.id == 'CANARY' for target in node.targets))
    assert isinstance(canary_node, ast.Constant) and isinstance(canary_node.value, str)
    canary_sha = digest(canary_node.value.encode())
    assert canary_sha == digest(os.environ['PAYPAL_HS_PUBLIC_CANARY'].encode())
    observer_env = json.loads((trusted[0].parent.parent / 'env/env.json').read_text())
    assert observer_env['ucs']['pid'] == source['ucs']['pid']
    binding = {'pid': pid, 'process_start_ticks': Path(f'/proc/{pid}/stat').read_text().split(') ', 1)[1].split()[19],
        'executable_sha256': digest(Path(f'/proc/{pid}/exe').read_bytes()),
        'source_path': str(trusted[0]), 'source_sha256': INERT_PROVIDER_OBSERVER_SHA256,
        'canary_sha256': canary_sha, 'observed_ucs_pid': source['ucs']['pid'],
        'endpoint': base.geturl()}
    start_wall, start_mono = time.time_ns(), time.monotonic_ns()
    with urllib.request.urlopen(base.geturl().rstrip('/') + '/proof', timeout=10) as response:
        assert response.status == 200
        wire = response.read(16 * 1024 * 1024 + 1)
        if len(wire) > 16 * 1024 * 1024:
            raise GsmUnavailable('actual provider observation response exceeded bounded read')
    finish_wall, finish_mono = time.time_ns(), time.monotonic_ns()
    value = json.loads(wire)
    assert set(value) == {'operation', 'inert_wallet_canary_received', 'actual_primary_ucs', 'observations'}
    for observation in value['observations']:
        assert set(observation) == {'path', 'body_sha256', 'inert_wallet_canary_received', 'actual_owned_ucs_peer', 'peer_port'}
        assert isinstance(observation['path'], str) and observation['path'].startswith('/')
        assert re.fullmatch(r'[0-9a-f]{64}', observation['body_sha256'])
        assert type(observation['inert_wallet_canary_received']) is bool and type(observation['actual_owned_ucs_peer']) is bool
    assert Path(f'/proc/{pid}/stat').read_text().split(') ', 1)[1].split()[19] == binding['process_start_ticks']
    assert digest(trusted[0].read_bytes()) == binding['source_sha256']
    return {'binding': binding, 'started_wall_ns': start_wall, 'completed_wall_ns': finish_wall,
        'started_monotonic_ns': start_mono, 'completed_monotonic_ns': finish_mono,
        'wire_sha256': digest(wire), 'actual_response': value}


def provider_post_delta(before, after):
    assert before['binding'] == after['binding']
    assert before['completed_monotonic_ns'] < after['started_monotonic_ns']
    assert 0 < after['completed_monotonic_ns'] - before['started_monotonic_ns'] <= 120_000_000_000
    initial, final = before['actual_response'], after['actual_response']
    assert initial['operation'] == final['operation'], 'provider observer reset during actual request'
    previous = initial['observations']
    assert len(final['observations']) >= len(previous) and final['observations'][:len(previous)] == previous
    calls = final['observations'][len(previous):]
    assert all(event['actual_owned_ucs_peer'] is True for event in calls)
    return calls, list(range(len(previous) + 1, len(final['observations']) + 1))


def collect_wallet_preflight_observation(snapshots, source, mode, subject, response_identity, before, canary_env):
    # Called immediately after the actual request, before any strict payment
    # state/readback assertion. Capturing evidence never makes those assertions pass.
    canary = os.environ[canary_env].encode()
    assert len(canary) >= 16 and response_identity.get('request_id')
    sink_windows = {}
    for name, snapshot in snapshots.items():
        actual = Path(f'/proc/{snapshot["pid"]}/fd/{snapshot["fd"]}').stat()
        selected = snapshot['path'].stat()
        assert (actual.st_dev, actual.st_ino) == (selected.st_dev, selected.st_ino) == (snapshot['device'], snapshot['inode'])
        assert selected.st_size >= snapshot['offset'] and selected.st_size - snapshot['offset'] <= 16 * 1024 * 1024
        with snapshot['path'].open('rb') as stream:
            stream.seek(snapshot['offset'])
            wire = stream.read(selected.st_size - snapshot['offset'])
        assert canary not in wire, 'public inert canary leaked; content withheld'
        sink_windows[name] = {**snapshot, 'path': str(snapshot['path']), 'end': selected.st_size, 'sha256': digest(wire)}
    after = provider_observation_snapshot(source)
    calls, indices = provider_post_delta(before, after)
    return {'source': source, 'wallet_preflight_observation': {'wallet_mode': mode,
        'subject': subject, 'request_id': response_identity['request_id'],
        'canary_env': canary_env, 'canary_sha256': digest(canary), 'sink_windows': sink_windows,
        'provider_http_window': {'before': before, 'after': after,
            'derived_POST_request_indices': indices, 'derived_POST_count': len(calls), 'GET': 'UNOBSERVED'},
        'vendor_exception': {'url': 'https://developer.paypal.com/api/orders/v2/schema.json',
            'request_type': 'google_pay_request', 'fields': ['name', 'email_address', 'phone_number', 'card',
                'decrypted_token', 'assurance_details', 'experience_context']}}}


def emission_source_proof(proof, source, ucs_repo, connector_absent=True):
    # This is reviewed control-flow evidence, not an operator's absence label.
    # Bind every quoted branch/emission excerpt to the actual running commits.
    assert set(proof) == ({'HS', 'UCS'} if connector_absent else {'UCS'})
    for name, repo in [('HS', ROOT), ('UCS', Path(ucs_repo))]:
        if name not in proof:
            continue
        item = proof[name]
        assert item['causal_explanation'].strip() and item['review_artifact_sha256']
        review = Path(item['review_artifact'])
        assert digest(review.read_bytes()) == item['review_artifact_sha256']
        review_data = json.loads(review.read_text())
        assert review_data['review_kind'] == 'google_psp_unsupported_connector_event_control_flow'
        assert review_data['repository'] == name and review_data['source_commit'] == source[name.lower()]['source_commit']
        assert review_data['causal_explanation'] == item['causal_explanation']
        assert review_data['anchors'] == item['anchors']
        if connector_absent:
            assert review_data.get('connector_event_absence_allowed') is True, 'source review does not authorize connector-event absence'
        roles = {'unsupported_return', 'provider_http_dispatch'} if name == 'UCS' else {'unsupported_return'}
        if connector_absent:
            roles.add('connector_event_emission')
        assert len(item['anchors']) >= 2 and {entry['role'] for entry in item['anchors']} >= roles
        for anchor in item['anchors']:
            path = Path(anchor['file'])
            assert not path.is_absolute() and '..' not in path.parts and path.parts[0] == 'crates' and path.suffix == '.rs'
            content = git(repo, 'show', source[name.lower()]['source_commit'] + ':' + str(path))
            assert digest(content) == anchor['file_sha256']
            lines = content.splitlines(keepends=True)
            assert 1 <= anchor['line_start'] <= anchor['line_end'] <= len(lines)
            excerpt = b''.join(lines[anchor['line_start'] - 1:anchor['line_end']])
            assert digest(excerpt) == anchor['excerpt_sha256']
    return True


def wallet_connector_event(event, mode, canary, source):
    """Only the source-bound no-wallet access-token event is auxiliary proof."""
    payload = json.loads(event) if isinstance(event, str) and event.lstrip().startswith('{') else tracing_json_value(event) if isinstance(event, str) else event
    assert isinstance(payload, dict)
    assert canary not in canonical(payload), 'public inert canary leaked; content withheld'
    flow = payload.get('flow')
    if flow == 'AccessTokenAuth':
        request_value = payload.get('request')
        request_value = json.loads(request_value) if isinstance(request_value, str) else request_value
        if not isinstance(request_value, dict) or set(request_value) != {'merchant_access_token_id', 'connector', 'test_mode'}:
            raise AssertionError('access-token auxiliary request schema differs from reviewed source')
        sensitive = {'wallet', 'apple_pay', 'google_pay', 'apple_pay_sdk', 'google_pay_sdk',
            'payment_method_data', 'payment_data', 'tokenization_data', 'cryptogram', 'online_payment_cryptogram'}
        def contains_wallet(value):
            if isinstance(value, dict):
                return bool(set(value) & sensitive) or any(contains_wallet(item) for item in value.values())
            if isinstance(value, list):
                return any(contains_wallet(item) for item in value)
            if isinstance(value, str) and value.lstrip().startswith(('{', '[')):
                try:
                    return contains_wallet(json.loads(value))
                except ValueError:
                    return False
            return False
        assert not contains_wallet(payload), 'wallet-bearing auxiliary event cannot be excluded'
        # Bind this exact request/flow exemption to the actual running HS source.
        anchors = {
            'crates/router/src/core/payments/gateway/access_token_gateway.rs': 'f3bee70e329e57fc512505bc2abf97300145b654c7adf4b00dce44b102d04c34',
            'crates/router/src/core/unified_connector_service.rs': '090df9d99d7d93ce5784411e73d234348f235641d1c89e5ebed151f93c783abc',
        }
        for path, sha in anchors.items():
            if digest(git(ROOT, 'show', source['hs']['source_commit'] + ':' + path)) != sha:
                raise GsmUnavailable('access-token auxiliary schema needs current source review')
        return False  # Never satisfies required Authorize wallet proof.
    if flow != 'Authorize':
        raise GsmUnavailable('unknown connector flow cannot certify the wallet mask')
    assert masked_wallet_field(payload, mode, 'ConnectorApiLogs'), 'observed Authorize wallet event is not correctly masked'
    return True


def applicable_wallet_mask_preflight(paths, source, config):
    states = {}
    for path in paths:
        report = json.loads(Path(path).read_text())
        if report.get('source') != source or 'wallet_preflight_observation' not in report:
            raise GsmUnavailable('current source-bound wallet origin observations are missing')
        observation = report['wallet_preflight_observation']
        if not {'wallet_mode', 'subject', 'request_id', 'canary_env', 'canary_sha256', 'sink_windows', 'provider_http_window'} <= set(observation):
            raise GsmUnavailable('current direct wallet sink/provider origin observations are unavailable')
        mode = observation['wallet_mode']
        assert mode in {'apple_psp', 'apple_hs', 'google_psp', 'google_hs'} and mode not in states
        subject, request_id = observation['subject'], observation['request_id']
        assert re.fullmatch(r'[A-Za-z0-9_-]{1,128}', subject)
        assert re.fullmatch(r'[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}', request_id)
        canary = os.environ[observation['canary_env']].encode()
        assert len(canary) >= 16 and digest(canary) == observation['canary_sha256']
        windows = observation['sink_windows']
        assert set(windows) == {'application', 'connector_event'}
        matched = {}
        correlated = []
        api_created_ns = None
        connector_seen = False
        for sink, window in windows.items():
            wire, records = observed_window(window, source['hs']['pid'])
            assert canary not in wire, 'inert token leaked in an actual observable HS sink'
            wanted = 'ApiLogs' if sink == 'application' else 'ConnectorApiLogs'
            for line, record in zip([line for line in wire.splitlines() if line], records):
                if request_id.encode() not in line:
                    continue
                if str(record.get('pid')) != str(source['hs']['pid']):
                    continue
                correlated.append(record)
                extra = record.get('extra', {})
                if record.get('target') != 'router::events::event_logger' or extra.get('event_type') != wanted:
                    continue
                if wanted == 'ConnectorApiLogs':
                    connector_seen = True
                    if not wallet_connector_event(extra.get('event'), mode, canary, source):
                        continue
                else:
                    assert masked_wallet_field(extra.get('event'), mode, wanted, require_typed_marker=(mode == 'google_psp')), 'observed wallet event is not correctly masked'
                if subject.encode() not in line:
                    raise GsmUnavailable('observed connector/API event cannot be correlated to the actual payment subject')
                if wanted == 'ApiLogs':
                    event = extra['event']
                    event = json.loads(event) if isinstance(event, str) and event.lstrip().startswith('{') else tracing_json_value(event)
                    assert type(event['created_at_timestamp']) is int
                    api_created_ns = event['created_at_timestamp'] * 1_000_000
                matched[sink] = digest(line)
        if 'application' not in matched:
            raise GsmUnavailable('current correlated exact wallet API mask is absent')
        if connector_seen and 'connector_event' not in matched:
            raise GsmUnavailable('auxiliary events cannot replace the required Authorize wallet mask')
        provider = observation['provider_http_window']
        if not isinstance(provider, dict) or not {'before', 'after'} <= set(provider):
            raise GsmUnavailable('actual BEFORE/AFTER owned provider proof observations are unavailable')
        before, after = provider['before'], provider['after']
        assert before['binding'] == after['binding']
        origin = after['binding']
        assert origin['pid'] == config['fixture_pid']
        assert origin['source_sha256'] == INERT_PROVIDER_OBSERVER_SHA256
        assert origin['canary_sha256'] == digest(canary) and origin['observed_ucs_pid'] == source['ucs']['pid']
        assert digest(Path(f'/proc/{origin["pid"]}/exe').read_bytes()) == origin['executable_sha256']
        assert Path(f'/proc/{origin["pid"]}/stat').read_text().split(') ', 1)[1].split()[19] == origin['process_start_ticks']
        producer_source = Path(origin['source_path']).resolve(strict=True)
        assert str(producer_source).encode() in Path(f'/proc/{origin["pid"]}/cmdline').read_bytes().split(b'\0')
        assert digest(producer_source.read_bytes()) == origin['source_sha256']
        calls, indices = provider_post_delta(before, after)
        assert provider['derived_POST_request_indices'] == indices and provider['derived_POST_count'] == len(calls)
        assert provider['GET'] == 'UNOBSERVED'
        assert before['started_wall_ns'] // 1_000_000 <= api_created_ns // 1_000_000 <= after['completed_wall_ns'] // 1_000_000
        canary_calls = [event for event in calls if event['inert_wallet_canary_received'] is True]
        if mode != 'google_psp':
            if set(matched) != {'application', 'connector_event'} or not any(
                    event['inert_wallet_canary_received'] is True for event in calls):
                raise GsmUnavailable('supported wallet needs both current masked sinks and actual inert provider wire observation')
            states[mode] = {'state': 'OBSERVED_BOTH_MASKED_WITH_INERT_WIRE', 'provider_verification': False}
            continue
        # HS→UCS gRPC token transport is legitimate. This counter measures only
        # actual owned UCS→provider HTTP handlers, never inferred gRPC absence.
        assert not canary_calls
        oauth_calls = [event for event in calls if event['path'] == '/v1/oauth2/token' and not event['inert_wallet_canary_received']]
        assert len(oauth_calls) == len(calls), 'non-OAuth provider POST observed on unsupported wallet path'
        boundary = False
        api_error = False
        for record in correlated:
            extra = record.get('extra', {})
            if record.get('target') == 'external_services::grpc_client::unified_connector_service' and extra.get('method') == 'payment_authorize':
                boundary |= 'PayPal REST does not accept an encrypted Google Pay token' in str(extra.get('grpc_error', ''))
            if extra.get('event_type') == 'ApiLogs':
                payload = extra.get('event')
                payload = json.loads(payload) if isinstance(payload, str) and payload.lstrip().startswith('{') else tracing_json_value(payload)
                candidates = [payload.get('error')]
                if payload.get('response'):
                    candidates.append(json.loads(payload['response']))
                api_error |= payload.get('status_code') == 400 and any(isinstance(item, dict) and
                    (item.get('error', item)).get('code') == 'IR_19' for item in candidates)
        if not boundary or not api_error:
            raise GsmUnavailable('actual HS→UCS unsupported boundary and typed IR_19 API error are not corroborated')
        source_proof = config.get('google_psp_emission_source_proof')
        if not source_proof:
            raise GsmUnavailable('current source-proven preHTTP refusal/emission proof is unavailable')
        absent = 'connector_event' not in matched
        if absent:
            assert not connector_seen
        emission_source_proof(source_proof, source,
            Path(f'/proc/{source["ucs"]["pid"]}/cwd').resolve(strict=True), connector_absent=absent)
        connector_state = 'NOT_APPLICABLE_API_CARRIER_UNAVAILABLE' if absent else 'OBSERVED_MASKED'
        exception = observation['vendor_exception']
        assert exception == {'url': 'https://developer.paypal.com/api/orders/v2/schema.json',
            'request_type': 'google_pay_request', 'fields': ['name', 'email_address', 'phone_number', 'card',
                'decrypted_token', 'assurance_details', 'experience_context']}
        states[mode] = {'state': 'APPLICABLE_MASK_PREFLIGHT_ONLY', 'connector_state': connector_state,
            'HS_to_UCS_gRPC_token_transport': 'PERMITTED', 'provider_HTTP_POSTs_observed': len(calls), 'provider_OAuth_POSTs_observed': len(oauth_calls),
            'provider_wallet_TOKEN_POSTs_observed': 0, 'provider_GETs': 'UNOBSERVED',
            'provider_verification': False, 'wallet_implementation_verified': False}
    if set(states) != {'apple_psp', 'apple_hs', 'google_psp', 'google_hs'}:
        raise GsmUnavailable('applicable wallet preflight observations are incomplete')
    return states


def gsm_preflight(operator, source, require_wallet_masks=True):
    cfg = operator.get('gsm', {})
    if not {'wallet_mask_receipts', 'owned_database', 'admin_key_env', 'engine_preflight_path'} <= set(cfg):
        raise GsmUnavailable('current GSM context lacks isolated-DB, wallet-mask or actual-engine preconditions')
    if operator['kind'] != 'production_fixture':
        raise GsmUnavailable('GSM controls require an owned inert production fixture')
    wallet_states = applicable_wallet_mask_preflight(cfg['wallet_mask_receipts'], source, cfg) if require_wallet_masks else {}
    if require_wallet_masks:
        ucs_environment = dict(item.split(b'=', 1) for item in Path(f'/proc/{source["ucs"]["pid"]}/environ').read_bytes().split(b'\0') if b'=' in item)
        endpoint = urllib.parse.urlsplit(ucs_environment.get(b'CS__CONNECTORS__PAYPAL__BASE_URL', b'').decode())
        if endpoint.scheme != 'http' or endpoint.hostname not in {'127.0.0.1', 'localhost'} or not endpoint.port or endpoint.username:
            raise GsmUnavailable('actual UCS PayPal outbound endpoint is not the isolated inert loopback fixture')
        fixture_pid = cfg.get('fixture_pid')
        if not isinstance(fixture_pid, int) or isinstance(fixture_pid, bool) or fixture_pid <= 0:
            raise GsmUnavailable('actual owned inert fixture PID is required before payment write')
        listeners = subprocess.check_output(['ss', '-H', '-ltnp', 'sport = :' + str(endpoint.port)], stderr=subprocess.DEVNULL).decode()
        assert {int(pid) for pid in re.findall(r'pid=(\d+)', listeners)} == {fixture_pid}
    database = cfg['owned_database']
    assert database['fresh_isolated'] is True
    marker = database['run_id'].rsplit('-', 1)[-1]
    assert len(marker) >= 6 and marker in database['container_name']
    template = '{"id":"{{.Id}}","name":"{{.Name}}","created":"{{.Created}}","running":{{.State.Running}},"ports":{{json .NetworkSettings.Ports}}}'
    actual = json.loads(subprocess.check_output(['docker', 'inspect', '--format', template,
        database['container_id']], stderr=subprocess.DEVNULL))
    assert actual['id'] == database['container_id'] and actual['name'] == '/' + database['container_name'] and actual['running'] is True
    assert datetime.datetime.fromisoformat(actual['created'].replace('Z', '+00:00')) >= datetime.datetime.fromisoformat(database['created_after'].replace('Z', '+00:00'))
    ports = actual['ports']['5432/tcp']
    assert len(ports) == 1 and ports[0]['HostIp'] == '127.0.0.1'
    # Read only the bound HS process environment in memory. Never retain the
    # environment, database password, admin key or request body in evidence.
    environment = dict(item.split(b'=', 1) for item in Path(f'/proc/{source["hs"]["pid"]}/environ').read_bytes().split(b'\0') if b'=' in item)
    prefix = b'ROUTER__MASTER_DATABASE__'
    assert environment[prefix + b'HOST'] in {b'127.0.0.1', b'localhost'}
    assert environment[prefix + b'PORT'].decode() == ports[0]['HostPort']
    assert environment[prefix + b'DBNAME'].decode() == database['database_name']
    return {'container_id': actual['id'], 'container_name': database['container_name'],
        'hs_pid': source['hs']['pid'], 'applicable_wallet_mask_states': wallet_states}


def run_gsm_case(case, context, journal, journal_path, source, base):
    operator = context['cases'][case['id']]
    guard = gsm_preflight(operator, source, require_wallet_masks=False)
    config = operator['gsm']
    assert urllib.parse.urlsplit(base).hostname in {'127.0.0.1', 'localhost'}
    admin_env = config['admin_key_env']
    token = 'paypal-gsm-' + digest(canonical({'case': case['id'], 'source': source}))[:32]

    def retrieve(key):
        status, value, _ = request(base, 'POST', '/gsm/get', key, None, admin_env, token)
        if status == 404:
            return None
        assert status == 200 and gsm_key(value) == key
        return value

    def mutate(phase, action, key, before, after):
        identity = phase + ':' + digest(canonical(key))
        writes = lifecycle.setdefault('writes', {})
        current = retrieve(key)
        if identity in writes and writes[identity]['state'] == 'sending':
            assert writes[identity]['before_sha256'] == digest(canonical(before)) and writes[identity]['after_sha256'] == digest(canonical(after))
            if current == after:
                writes[identity]['state'] = 'read_only_reconciled_applied'
                atomic(journal_path, journal)
                return
            assert current == before, 'GSM response-lost mutation differs from both known states'
        elif identity in writes:
            assert current == after, 'completed GSM mutation unexpectedly changed'
            return
        assert current == before, 'GSM rule changed after its exact snapshot'
        writes[identity] = {'state': 'sending', 'action': action, 'key': key,
            'before_sha256': digest(canonical(before)), 'after_sha256': digest(canonical(after))}
        atomic(journal_path, journal)
        endpoint = {'create': '/gsm', 'update': '/gsm/update', 'delete': '/gsm/delete'}[action]
        status, _, _ = request(base, 'POST', endpoint, key if action == 'delete' else after, None, admin_env, token)
        assert status == 200
        assert retrieve(key) == after, 'actual keyed readback differs from desired GSM record'
        writes[identity]['state'] = 'readback_verified'
        atomic(journal_path, journal)

    rows = gsm_policy()
    spec = case['gsm']
    code = spec['processor_code']
    message = spec['processor_message']
    issuer_key = {'connector': 'paypal', 'flow': 'Payment', 'sub_flow': 'Authorize',
        'code': 'Network:Visa|IssuerCode:' + code, 'message': 'Network:Visa|IssuerCode:' + code}
    connector_key = {**issuer_key, 'code': code, 'message': message}
    selected = [row for row in rows if row['code'] in {code, issuer_key['code']}]
    mode = spec['lookup']
    if mode in {'wrong_flow', 'wrong_subflow', 'unmatched_message'}:
        row = next(row.copy() for row in selected if row['code'] == code)
        field, replacement = {'wrong_flow': ('flow', 'Authorize'),
            'wrong_subflow': ('sub_flow', 'Capture'), 'unmatched_message': ('message', message + ' (unmatched control)')}[mode]
        row[field] = replacement
        row['unified_code'] += '_' + mode.upper()
        selected = [row]
    if mode == 'unknown_no_rule':
        selected = []
    lifecycles = journal.setdefault('gsm_lifecycles', {})
    lifecycle = lifecycles.get(case['id'])
    if lifecycle and lifecycle.get('state') in {'cleaned', 'aborted_cleaned'}:
        for snapshot in lifecycle['snapshots']:
            assert retrieve(snapshot['key']) == snapshot['value']
        return [{'state': 'READ_ONLY_RECONCILED', 'cached_not_current_verification': True,
            'provider_verification': False, 'current_mapping_verified': False}]
    if lifecycle is None:
        snapshots = [{'key': gsm_key(row), 'value': retrieve(gsm_key(row))} for row in selected]
        for item, desired in zip(snapshots, selected):
            existing = item['value']
            if existing is not None and (existing['unified_code'] != desired['unified_code'] or existing['decision'] != 'do_default'
                    or existing['feature'] != 'retry' or existing['feature_data'] != desired['feature_data']
                    or existing['clear_pan_possible'] is not False or existing['step_up_possible'] is not False):
                raise GsmUnavailable('an unrelated existing global rule would be overwritten')
        if mode in {'unknown_no_rule', 'wrong_flow', 'wrong_subflow', 'unmatched_message'} and any(retrieve(key) is not None for key in [issuer_key, connector_key]):
            raise GsmUnavailable('negative/no-rule control already has a matching issuer or connector rule')
        lifecycle = {'source_sha256': digest(canonical(source)), 'state': 'snapshotted', 'snapshots': snapshots, 'writes': {}}
        lifecycles[case['id']] = lifecycle
        atomic(journal_path, journal)
    assert lifecycle['source_sha256'] == digest(canonical(source)), 'GSM lifecycle source changed; reconcile with original committed source'
    result = None
    succeeded = False
    try:
        for snapshot, row in zip(lifecycle['snapshots'], selected):
            mutate('install', 'create' if snapshot['value'] is None else 'update', snapshot['key'], snapshot['value'], row)
            mutate('update_readback', 'update', snapshot['key'], row, row)
        lifecycle['admin_requests_readback_graded'] = True
        atomic(journal_path, journal)
        # Admin CRUD is nonfinancial. All wallet mask guards are mandatory before
        # the NEW payment write; a blocked engine still cleans up every rule.
        guard = gsm_preflight(operator, source)
        # This is a real producer endpoint, never an authored expected-result
        # file. It must prove live HS retry configuration and routing eligibility.
        assert urllib.parse.urlsplit(os.environ['PAYPAL_HS_PROOF_BASE']).hostname in {'127.0.0.1', 'localhost', '::1'}
        preflight_status, preflight, _ = request(os.environ['PAYPAL_HS_PROOF_BASE'], 'GET', config['engine_preflight_path'], None, None,
            operator.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
        assert preflight_status == 200
        check(preflight, {'actual_engine_observed': True, 'fixture_only': True,
            'hs_pid': source['hs']['pid'], 'hs_binary_sha256': source['hs']['binary_sha256'],
            'ucs_binary_sha256': source['ucs']['binary_sha256'], 'automatic_retries_enabled': True,
            'eligible_alternate_owned_fixture': True, 'external_provider_calls': 0,
            'eligible_connector_names': ['paypal', 'paypal'], 'fixture_pid': config['fixture_pid']})
        assert isinstance(preflight['retry_budget'], int) and not isinstance(preflight['retry_budget'], bool) and preflight['retry_budget'] > 0
        assert len(set(preflight['eligible_mca_ids'])) >= 2
        positive = mode in {'issuer', 'issuer_first'}
        matched = issuer_key if positive else None
        row = next((row for row in selected if gsm_key(row) == matched), None)
        expected = {'/status': 'failed', '/payment_id': '@payment_id', '/error_code': code, '/error_message': message}
        if row:
            expected['/unified_code'] = row['unified_code']
        engine_case = {**case, 'steps': [{'method': 'POST', 'http_status': [200],
            'expect': expected, 'readback_expect': expected}], 'proof': {
                'actual_engine_observed': True, 'fixture_only': True, 'matched_key': matched,
                'decision': 'do_default', 'initial_fixture_calls': 1, 'alternate_fixture_calls': 0,
                'external_provider_calls': 0, 'retry_budget_before': preflight['retry_budget'],
                'eligible_mca_ids': preflight['eligible_mca_ids'],
                'hs_pid': source['hs']['pid'], 'hs_binary_sha256': source['hs']['binary_sha256'],
                'ucs_binary_sha256': source['ucs']['binary_sha256']}}
        result = run_case(engine_case, context, journal, journal_path, source, base)
        succeeded = True
        return result
    finally:
        failures = []
        for snapshot in reversed(lifecycle['snapshots']):
            try:
                current = retrieve(snapshot['key'])
                if current != snapshot['value']:
                    installed = next(row for row in selected if gsm_key(row) == snapshot['key'])
                    assert current is None or current == installed, 'unrelated concurrent GSM change; cleanup must not clobber it'
                    if current is not None:
                        mutate('restore_delete', 'delete', snapshot['key'], current, None)
                    if snapshot['value'] is not None:
                        mutate('restore_create', 'create', snapshot['key'], None, snapshot['value'])
                assert retrieve(snapshot['key']) == snapshot['value']
            except Exception:
                failures.append(digest(canonical(snapshot['key'])))
        lifecycle['state'] = ('cleaned' if succeeded else 'aborted_cleaned') if not failures else 'cleanup_failed'
        lifecycle['cleanup_failures'] = failures
        lifecycle['guard'] = guard
        atomic(journal_path, journal)
        assert not failures, 'GSM cleanup failed; keyed reconciliation required before further execution'


def fault_control(base, value=None):
    endpoint = urllib.parse.urlsplit(base)
    assert endpoint.hostname in {'127.0.0.1', 'localhost', '::1'}
    req = urllib.request.Request(base.rstrip('/') + ('/arm' if value else '/proof'),
        data=canonical(value) if value else None,
        headers={'Content-Type': 'application/json'}, method='POST' if value else 'GET')
    with urllib.request.urlopen(req, timeout=10) as response:
        return json.load(response)


def serve_fault_proxy(listen_port, upstream_port, control_port):
    """An inert loopback TCP proxy: drop real gRPC connections, never synthesize HTTP.

    Provision HS's UCS endpoint to this loopback listener for fault rows only.
    The upstream remains loopback. No payload/header bytes are logged or stored.
    """
    lock = threading.Lock()
    state = {'armed': False, 'operation': None, 'drops': 0}
    launched_runner_sha256 = digest(Path(__file__).read_bytes())
    sockets = set()

    def drop(connection):
        try:
            connection.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        connection.close()

    class Proxy(socketserver.BaseRequestHandler):
        def handle(self):
            with lock:
                if state['armed']:
                    state['drops'] += 1
                    drop(self.request)
                    return
            try:
                upstream = socket.create_connection(('127.0.0.1', upstream_port), timeout=5)
            except OSError:
                drop(self.request)
                return
            with lock:
                sockets.update((self.request, upstream))
            try:
                while True:
                    readable, _, _ = select.select([self.request, upstream], [], [], 1)
                    for incoming in readable:
                        wire = incoming.recv(65536)
                        if not wire:
                            return
                        (upstream if incoming is self.request else self.request).sendall(wire)
            except (OSError, ValueError):
                pass
            finally:
                with lock:
                    sockets.discard(self.request)
                    sockets.discard(upstream)
                drop(self.request)
                drop(upstream)

    class Control(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            if self.path != '/arm':
                self.send_error(404)
                return
            value = json.loads(self.rfile.read(int(self.headers.get('Content-Length', '0'))))
            assert set(value) == {'armed', 'operation'} and isinstance(value['armed'], bool)
            with lock:
                state.update(armed=value['armed'], operation=value['operation'], drops=0)
                if value['armed']:
                    for connection in tuple(sockets):
                        drop(connection)
                        state['drops'] += 1
            self.do_GET()

        def do_GET(self):
            with lock:
                value = {'fault_kind': 'transport_connection_closed', 'http_response_generated': False,
                    'operation': state['operation'], 'fault_injected': state['drops'] > 0,
                    'runner_sha256': launched_runner_sha256,
                    'drop_count': state['drops'], 'armed': state['armed']}
            wire = canonical(value)
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(wire)))
            self.end_headers()
            self.wfile.write(wire)

    class Server(socketserver.ThreadingTCPServer):
        allow_reuse_address = True
        daemon_threads = True
    proxy = Server(('127.0.0.1', listen_port), Proxy)
    threading.Thread(target=proxy.serve_forever, daemon=True).start()
    ThreadingHTTPServer(('127.0.0.1', control_port), Control).serve_forever()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--serve-fault-proxy', nargs=3, type=int, metavar=('LISTEN', 'UPSTREAM', 'CONTROL'))
    parser.add_argument('--context', type=Path)
    parser.add_argument('--journal', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--case', action='append')
    parser.add_argument('--require-committed', action='store_true')
    parser.add_argument('--hs-build-receipt', type=Path)
    parser.add_argument('--ucs-build-receipt', type=Path)
    parser.add_argument('--ucs-repo', type=Path)
    parser.add_argument('--ucs-endpoint', default='127.0.0.1:8001')
    args = parser.parse_args()
    if args.serve_fault_proxy:
        serve_fault_proxy(*args.serve_fault_proxy)
        return 0
    assert args.context and args.journal and args.out
    # Whole invocation lock precedes initialization, source reads and operation IDs.
    args.journal.parent.mkdir(parents=True, exist_ok=True)
    with args.journal.with_suffix(args.journal.suffix + '.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        fixtures = json.loads(FIXTURES.read_text())
        context = json.loads(args.context.read_text())
        hs_source = manifest(ROOT)
        source = {'hs': {'commit': hs_source['commit'], 'manifest_sha256': hs_source['sha256']},
                  'runner_sha256': digest(Path(__file__).read_bytes()), 'fixtures_sha256': digest(FIXTURES.read_bytes())}
        executing = any(case['id'] in context.get('cases', {}) for case in fixtures['cases']
            if (not args.case or case['id'] in args.case) and case.get('execution_policy') != 'private_boundary_not_publicly_exercisable')
        assert not executing or args.require_committed, 'all runtime execution requires --require-committed'
        def bound_runtime():
            assert args.hs_build_receipt and args.ucs_build_receipt and args.ucs_repo
            binding = {
                'hs': runtime(ROOT, args.hs_build_receipt, 'router', os.environ['HYPERSWITCH_BASE_URL']),
                'ucs': runtime(args.ucs_repo, args.ucs_build_receipt, 'grpc-server', args.ucs_endpoint),
                'runner_sha256': digest(Path(__file__).read_bytes()),
                'fixtures_sha256': digest(FIXTURES.read_bytes()),
            }
            actual_route(binding, args.ucs_endpoint)
            return binding
        if executing:
            source = bound_runtime()
        journal = json.loads(args.journal.read_text()) if args.journal.exists() else {'operations': {}}
        results = []
        wanted = set(args.case or [case['id'] for case in fixtures['cases']])
        assert wanted <= {case['id'] for case in fixtures['cases']}
        for case in fixtures['cases']:
            if case['id'] not in wanted:
                continue
            try:
                if case.get('execution_policy') == 'private_boundary_not_publicly_exercisable':
                    results.append({'id': case['id'], 'group': case['group'], 'state': 'NOT_RUN',
                        'requires': ['This private server-minted capture-reference negative cannot be injected through the production public route; no test-only production hook permitted'],
                        'evidence_state': 'IMPLEMENTED', 'provider_verification': False})
                    continue
                if case['id'] not in context.get('cases', {}):
                    results.append({'id': case['id'], 'group': case['group'], 'state': 'NOT_RUN',
                                    'requires': case['requires']})
                    continue
                actual = (run_gsm_case if case.get('gsm') else run_case)(
                    case, context, journal, args.journal, source, os.environ['HYPERSWITCH_BASE_URL'])
                results.append({'id': case['id'], 'group': case['group'], 'state': 'COMPLETED', 'steps': actual})
            except GsmUnavailable as error:
                results.append({'id': case['id'], 'group': case['group'], 'state': 'NOT_RUN',
                    'requires': [str(error)], 'evidence_state': 'IMPLEMENTED', 'provider_verification': False,
                    'gsm_lifecycle': journal.get('gsm_lifecycles', {}).get(case['id'])})
            except LogSinkUnavailable:
                results.append({'id': case['id'], 'group': case['group'], 'state': 'NOT_RUN',
                    'requires': ['Both actual bound HS regular-file JSON sinks with correlated masked wallet entries; Kafka/pipe-only/filtered collector cannot prove this oracle']})
            except Exception as error:
                # Never print request/response bodies, header values or exception URLs.
                results.append({'id': case['id'], 'group': case['group'], 'state': 'FAILED',
                                'error_type': type(error).__name__, 'detail': 'Request or oracle failed; inspect safe hashes and controlled runtime evidence'})
        binding_valid = not executing or source == bound_runtime()
        for result in results:
            if result['state'] == 'COMPLETED':
                fresh = binding_valid and all(step.get('state') == 'ORACLE_PASSED_PENDING_RUNTIME_RECHECK'
                    for step in result['steps'])
                result['current_mapping_verified'] = fresh
                result['provider_verification'] = fresh and all(step.get('provider_state_verified') is True
                    for step in result['steps'])
                if not fresh:
                    result['state'] = 'READ_ONLY_RECONCILED' if binding_valid else 'RUNTIME_BINDING_INVALIDATED'
                for step in result['steps']:
                    if step.get('state') == 'ORACLE_PASSED_PENDING_RUNTIME_RECHECK':
                        step['state'] = ('VERIFIED' if step.get('provider_state_verified') is True else 'GRADED') if binding_valid else 'RUNTIME_BINDING_INVALIDATED'
                        step['current_mapping_verified'] = bool(binding_valid)
                        step['provider_verification'] = bool(binding_valid and step.get('provider_state_verified') is True)
        atomic(args.out, {'source': source, 'cases': results, 'created_at_epoch': int(time.time())})
        print(json.dumps({'cases': len(results), 'failed': sum(r['state'] == 'FAILED' for r in results),
                          'not_run': sum(r['state'] == 'NOT_RUN' for r in results)}))
        return 1 if any(r['state'] != 'COMPLETED' for r in results) else 0


if __name__ == '__main__':
    raise SystemExit(main())
