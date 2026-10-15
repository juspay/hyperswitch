#!/usr/bin/env python3
"""Drive actual HS APIs and readback oracles; fixture evidence is never vendor verification.

Operator context contains resource IDs and environment variable NAMES only. Request
bodies, API keys, provider headers and credentials are resolved in memory. A completed
journal operation is reconciled by GET, never replayed. Final mode requires both real
committed runtimes, not merely binaries left in a target directory.
"""
import argparse
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
PROTECTED = ('2K728570VG9010042', '0SG65781FC8888712')
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


def request(base, method, path, body, header_env, key_env, idempotency):
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
            status, wire = response.status, response.read()
    except urllib.error.HTTPError as error:
        status, wire = error.code, error.read()
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


def masked_wallet_field(value):
    if isinstance(value, str):
        try:
            return masked_wallet_field(json.loads(value))
        except (ValueError, TypeError):
            # tracing's serde_json::Value Debug representation nests JSON in
            # String("...") fields. Decode those actual string literals in
            # memory; an unrelated masked API key is never a wallet oracle.
            for match in re.finditer(r'String\(("(?:[^"\\]|\\.)*")\)', value):
                try:
                    if masked_wallet_field(json.loads(match.group(1))):
                        return True
                except (ValueError, TypeError):
                    pass
            return bool(re.search(r'"(?:encrypted_data|encryptedData|payment_data|paymentData|cryptogram|online_payment_cryptogram)"\s*:\s*String\("\*\*\*"\)', value))
    if isinstance(value, list):
        return any(masked_wallet_field(item) for item in value)
    if isinstance(value, dict):
        return any((''.join(c.lower() for c in key if c.isalnum()) in
            {'encrypteddata', 'paymentdata', 'cryptogram', 'onlinepaymentcryptogram'} and item == '***')
            or masked_wallet_field(item) for key, item in value.items())
    return False


def verify_log_sinks(snapshots, operation, canary):
    assert canary and len(canary) >= 16
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
            with snapshot['path'].open('rb') as stream:
                stream.seek(snapshot['offset'])
                wire = stream.read()
            assert canary not in wire, 'public inert canary leaked; content withheld'
            for line in wire.splitlines():
                if operation.encode() not in line:
                    continue
                try:
                    record = json.loads(line)
                except (ValueError, UnicodeDecodeError):
                    continue
                event = b'ConnectorApiLogs' in line
                if (name == 'connector_event') != event or not masked_wallet_field(record):
                    continue
                observed[name] = {'device': snapshot['device'], 'inode': snapshot['inode'],
                    'offset': snapshot['offset'], 'end': selected.st_size, 'correlated_masked_entry_sha256': digest(line)}
        if len(observed) == 2:
            return observed
        time.sleep(.1)
    raise LogSinkUnavailable('correlated masked wallet entry missing from an actual HS sink')


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
            assert saved['identity'] == identity, 'journal request or source identity changed'
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
                results.append({'state': 'READ_ONLY_RECONCILED', 'operation': operation,
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
        log_snapshots = snapshot_log_sinks(operator, source) if case.get('log_canary') else None
        if step.get('precondition'):
            precondition_path = cfg['precondition_path']
            assert urllib.parse.parse_qs(urllib.parse.urlsplit(precondition_path).query).get('force_sync') != ['true']
            pre_status, pre_state, _ = request(base, 'GET', precondition_path, None, None,
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            assert pre_status == 200
            check(pre_state, resolve(step['precondition'], bindings))
        journal['operations'][operation] = {'identity': identity, 'idempotency': token, 'state': 'sending'}
        atomic(journal_path, journal)
        transport_proof = None
        if case.get('transport_fault'):
            fault_control(os.environ['PAYPAL_HS_TRANSPORT_CONTROL'], {'armed': True, 'operation': token})
        try:
            status, response, wire_sha = request(base, method, path, body, cfg.get('header_env'),
                cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
        finally:
            if case.get('transport_fault'):
                transport_proof = fault_control(os.environ['PAYPAL_HS_TRANSPORT_CONTROL'])
                fault_control(os.environ['PAYPAL_HS_TRANSPORT_CONTROL'], {'armed': False, 'operation': token})
                assert transport_proof['fault_injected'] and transport_proof['operation'] == token

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
        if case.get('proof') and index == case.get('proof_after_step', index):
            # Local fixture/notification collector supplies actual wire observations,
            # not an authored expected-result file. Proof endpoint is read-only.
            if transport_proof is None:
                proof_url = urllib.parse.urlsplit(os.environ['PAYPAL_HS_PROOF_BASE'])
                assert proof_url.hostname in {'127.0.0.1', 'localhost', '::1'}
            if transport_proof is not None:
                proof = transport_proof
            else:
                _, proof, _ = request(os.environ['PAYPAL_HS_PROOF_BASE'], 'GET', cfg['proof_path'],
                    None, None, cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            check(proof, resolve(case['proof'], bindings))
            assert proof['operation'] == token
        vendor_wire_verified = False
        if operator['kind'] == 'provider_live':
            proof_url = urllib.parse.urlsplit(os.environ['PAYPAL_HS_PROOF_BASE'])
            assert proof_url.hostname in {'127.0.0.1', 'localhost', '::1'}
            _, wire_proof, _ = request(os.environ['PAYPAL_HS_PROOF_BASE'], 'GET', cfg['vendor_proof_path'],
                None, None, cfg.get('api_key_env', 'HYPERSWITCH_API_KEY'), token)
            check(wire_proof, {'operation': token, 'provider_hostname': 'api-m.sandbox.paypal.com',
                'actual_wire_observed': True, 'fixture_response': False,
                'hs_binary_sha256': source['hs']['binary_sha256'],
                'ucs_binary_sha256': source['ucs']['binary_sha256']})
            vendor_wire_verified = True
        log_evidence = None
        if log_snapshots is not None:
            assert operator['kind'] == 'production_fixture', 'log canary uses public inert fixture only'
            log_evidence = verify_log_sinks(log_snapshots, token, os.environ['PAYPAL_HS_PUBLIC_CANARY'].encode())
            assert proof and proof['inert_wallet_canary_received'] is True
        result = {'operation': operation, 'state': 'ORACLE_PASSED_PENDING_RUNTIME_RECHECK',
                  'http_status': status, 'wire_sha256': wire_sha, 'readback_sha256': read_sha,
                  'provider_verification': False, 'requested_evidence_kind': operator['kind'],
                  'current_mapping_verified': False, 'vendor_wire_verified': vendor_wire_verified,
                  'log_sink_evidence': log_evidence,
                  'unresolved_capture_rows': [{'capture_id': row.get('capture_id'), 'preserved_status': row.get('status')}
                      for row in (state.get('captures') or []) if not row.get('connector_capture_id')
                        or row.get('connector_capture_id') not in (((state.get('connector_metadata') or {}).get('capture_context') or {}).get('capture_amounts') or {})] if isinstance(state, dict) else [],
                  'aggregate_oracle_does_not_certify_all_local_rows': True}
        journal['operations'][operation].update(state='completed', result=result)
        atomic(journal_path, journal)
        results.append(result)
    return results


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
                actual = run_case(case, context, journal, args.journal, source, os.environ['HYPERSWITCH_BASE_URL'])
                results.append({'id': case['id'], 'group': case['group'], 'state': 'COMPLETED', 'steps': actual})
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
                result['provider_verification'] = fresh and all(step.get('vendor_wire_verified') is True
                    for step in result['steps'])
                if not fresh:
                    result['state'] = 'READ_ONLY_RECONCILED' if binding_valid else 'RUNTIME_BINDING_INVALIDATED'
                for step in result['steps']:
                    if step.get('state') == 'ORACLE_PASSED_PENDING_RUNTIME_RECHECK':
                        step['state'] = ('VERIFIED' if step.get('vendor_wire_verified') is True else 'GRADED') if binding_valid else 'RUNTIME_BINDING_INVALIDATED'
                        step['current_mapping_verified'] = bool(binding_valid)
                        step['provider_verification'] = bool(binding_valid and step.get('vendor_wire_verified') is True)
        atomic(args.out, {'source': source, 'cases': results, 'created_at_epoch': int(time.time())})
        print(json.dumps({'cases': len(results), 'failed': sum(r['state'] == 'FAILED' for r in results),
                          'not_run': sum(r['state'] == 'NOT_RUN' for r in results)}))
        return 1 if any(r['state'] != 'COMPLETED' for r in results) else 0


if __name__ == '__main__':
    raise SystemExit(main())
