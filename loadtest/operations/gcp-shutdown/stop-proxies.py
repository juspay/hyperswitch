import json, pathlib, subprocess, urllib.request, urllib.error, time

root = pathlib.Path(__file__).parent
token = subprocess.check_output(['gcloud', 'auth', 'print-access-token'], text=True).strip()
base = 'https://compute.googleapis.com/compute/v1/projects/hyperswitch-dev/regions/asia-south1'

def request(url, method='GET', body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method, headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        raise RuntimeError(error.read().decode()) from None

snapshots = []
for name in ['dev-hyperswitch-envoy-stable', 'dev-hyperswitch-squid']:
    snapshots.append(request(base + '/autoscalers/' + name + '-autoscaler'))
snapshot_path = root / 'proxy-autoscalers.json'
if not snapshot_path.exists() or not snapshot_path.read_text().strip():
    snapshot_path.write_text(json.dumps(snapshots, indent=2))
for auto in snapshots:
    print('Pausing autoscaler:', auto['name'], flush=True)
    policy = dict(auto['autoscalingPolicy'], mode='OFF')
    operation = request(base + '/autoscalers?autoscaler=' + auto['name'], 'PATCH', {'name': auto['name'], 'autoscalingPolicy': policy})
    while operation.get('status') != 'DONE':
        time.sleep(3)
        operation = request(operation['selfLink'])
    if operation.get('error'):
        raise RuntimeError(operation['error'])
    print('Autoscaler paused:', auto['name'], flush=True)

snapshot = json.loads((root / 'before-shutdown.json').read_text())
groups = [request(base + '/instanceGroupManagers/' + name) for name in ['dev-hyperswitch-envoy-stable', 'dev-hyperswitch-squid']]
group_snapshot = root / 'proxy-managed-groups.json'
if not group_snapshot.exists():
    group_snapshot.write_text(json.dumps(groups, indent=2))
for name in ['dev-hyperswitch-envoy-stable', 'dev-hyperswitch-squid']:
    subprocess.run(['gcloud', 'compute', 'instance-groups', 'managed', 'update', name,
                    '--instance-redistribution-type=none', '--region=asia-south1', '--project=hyperswitch-dev', '--quiet'], check=True)
    instances = [v['name'] for v in snapshot['vms'] if v['name'].startswith(name if 'envoy' in name else 'dev-hyperswitch-squid-')]
    print('Stopping managed instances:', ', '.join(instances), flush=True)
    subprocess.run(['gcloud', 'compute', 'instance-groups', 'managed', 'stop-instances', name,
                    '--instances=' + ','.join(instances), '--region=asia-south1', '--project=hyperswitch-dev', '--quiet'], check=True)
print('Managed proxy shutdown configured.', flush=True)
