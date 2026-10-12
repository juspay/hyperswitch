import concurrent.futures, json, pathlib, subprocess, sys, time

ROOT = pathlib.Path(__file__).parent
PROJECT = 'hyperswitch-dev'
REGION = 'asia-south1'
CLUSTER = 'hyperswitch-dev-gke-v1'
CONTEXT = 'gke_hyperswitch-dev_asia-south1_hyperswitch-dev-gke-v1'

def run(args):
    result = subprocess.run(args, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(' '.join(args) + '\n' + result.stderr)
    return result.stdout

def cloud(*args):
    return run(['gcloud', *args, '--project', PROJECT, '--quiet'])

def kube(*args):
    return run(['kubectl', '--context', CONTEXT, *args])

def main():
    gke = json.loads(cloud('container', 'clusters', 'describe', CLUSTER, '--region', REGION, '--format=json'))
    workloads = json.loads(kube('get', 'deployments,statefulsets,cronjobs', '-A', '-o', 'json'))['items']
    workloads = [w for w in workloads if w['metadata']['namespace'] in ['hyperswitch', 'monitoring', 'istio-system']]
    databases = []
    for cluster in json.loads(cloud('alloydb', 'clusters', 'list', '--region', REGION, '--format=json')):
        name = cluster['name'].split('/')[-1]
        instances = json.loads(cloud('alloydb', 'instances', 'list', '--cluster', name, '--region', REGION, '--format=json'))
        databases.append({'cluster': name, 'instances': [{'name': i['name'].split('/')[-1], 'instanceType': i['instanceType'], 'activationPolicy': i.get('activationPolicy', 'ALWAYS'), 'state': i['state']} for i in instances]})
    vms = json.loads(cloud('compute', 'instances', 'list', '--format=json'))
    vms = [{'name': v['name'], 'zone': v['zone'].split('/')[-1], 'status': v['status']} for v in vms if v['name'].startswith(('dev-hyperswitch-envoy-', 'dev-hyperswitch-squid-', 'dev-hyperswitch-bastion-'))]
    snapshot = {'project': PROJECT, 'region': REGION, 'cluster': CLUSTER,
                'nodePools': [{'name': p['name'], 'autoscaling': p.get('autoscaling', {}), 'instanceGroupUrls': p.get('instanceGroupUrls', [])} for p in gke['nodePools']],
                'workloads': [{'namespace': w['metadata']['namespace'], 'name': w['metadata']['name'], 'kind': w['kind'], 'replicas': w['spec'].get('replicas'), 'suspend': w['spec'].get('suspend')} for w in workloads],
                'databases': databases, 'vms': vms}
    # Preserve the original restore point even if a later retry is needed.
    path = ROOT / 'before-shutdown.json'
    if '--use-snapshot' in sys.argv:
        original = json.loads(path.read_text())
        if original['project'] != PROJECT or original['cluster'] != CLUSTER:
            raise RuntimeError('Restore snapshot does not match the shutdown target')
        print('Preserving existing restore snapshot:', path, flush=True)
    else:
        with path.open('x') as out:
            json.dump(snapshot, out, indent=2)
        print('Saved restore snapshot:', path, flush=True)
    if '--snapshot-only' in sys.argv:
        print('Read-only inventory complete; no cloud resources changed.', flush=True)
        return
    for namespace in sorted({w['metadata']['namespace'] for w in workloads}):
        print(kube('scale', 'deployment,statefulset', '--all', '-n', namespace, '--replicas=0'), flush=True)
    for w in workloads:
        if w['kind'] == 'CronJob':
            print(kube('patch', 'cronjob', w['metadata']['name'], '-n', w['metadata']['namespace'], '--type=merge', '-p', '{"spec":{"suspend":true}}'), flush=True)

    def stop_database(db):
        # Stop read pools first, then the primary within each cluster.
        for instance in sorted(db['instances'], key=lambda i: i['instanceType'] == 'PRIMARY'):
            details = json.loads(cloud('alloydb', 'instances', 'describe', instance['name'], '--cluster', db['cluster'], '--region', REGION, '--format=json'))
            if details['state'] == 'STOPPED':
                print('DB already stopped:', instance['name'], flush=True)
                continue
            print('Stopping DB:', db['cluster'], instance['name'], flush=True)
            if details['state'] != 'STOPPING':
                cloud('alloydb', 'instances', 'update', instance['name'], '--cluster', db['cluster'], '--region', REGION, '--activation-policy=NEVER', '--async')
            for attempt in range(180):
                details = json.loads(cloud('alloydb', 'instances', 'describe', instance['name'], '--cluster', db['cluster'], '--region', REGION, '--format=json'))
                if details['state'] == 'STOPPED':
                    break
                time.sleep(5)
            else:
                raise RuntimeError('Database stop did not complete: ' + instance['name'])
            print('DB stopped:', instance['name'], flush=True)

    errors = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        futures = [pool.submit(stop_database, db) for db in databases]
        # GKE operations must be sequential to avoid overlapping updates.
        for node_pool in snapshot['nodePools']:
            try:
                name = node_pool['name']
                print('Disabling node autoscaling:', name, flush=True)
                cloud('container', 'clusters', 'update', CLUSTER, '--region', REGION, '--node-pool', name, '--no-enable-autoscaling')
                print('Scaling nodes to zero:', name, flush=True)
                cloud('container', 'clusters', 'resize', CLUSTER, '--region', REGION, '--node-pool', name, '--num-nodes=0')
                print('Node pool at zero:', name, flush=True)
            except Exception as exc:
                errors.append(str(exc))
                print('ERROR:', exc, flush=True)
        for f in futures:
            try:
                f.result()
            except Exception as exc:
                errors.append(str(exc))
                print('ERROR:', exc, flush=True)
    # Keep network access available until Kubernetes and database work finishes.
    for vm in sorted(vms, key=lambda v: 'bastion' in v['name']):
        if vm['status'] == 'TERMINATED':
            continue
        try:
            print('Stopping VM:', vm['name'], flush=True)
            cloud('compute', 'instances', 'stop', vm['name'], '--zone', vm['zone'])
            print('VM stopped:', vm['name'], flush=True)
        except Exception as exc:
            errors.append(str(exc))
            print('ERROR:', exc, flush=True)
    if errors:
        raise RuntimeError('\n'.join(errors))
    print('Shutdown requests completed.', flush=True)

if __name__ == '__main__':
    main()
