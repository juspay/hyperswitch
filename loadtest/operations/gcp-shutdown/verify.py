import concurrent.futures, json, pathlib, subprocess

root = pathlib.Path(__file__).parent
snapshot = json.loads((root / 'before-shutdown.json').read_text())

def cloud(*args):
    return json.loads(subprocess.check_output(['gcloud', *args, '--project=hyperswitch-dev', '--format=json'], text=True))

def db_status(db):
    instances = cloud('alloydb', 'instances', 'list', '--cluster=' + db['cluster'], '--region=asia-south1')
    return [{'name': i['name'].split('/')[-1], 'state': i['state'], 'activationPolicy': i.get('activationPolicy')} for i in instances]

with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
    databases = list(executor.map(db_status, snapshot['databases']))
instances = cloud('compute', 'instances', 'list')
vm_names = {v['name'] for v in snapshot['vms']}
vms = [{'name': v['name'], 'state': v['status']} for v in instances if v['name'] in vm_names]
nodes = [{'name': v['name'], 'state': v['status']} for v in instances if v.get('labels', {}).get('goog-k8s-cluster-name') == snapshot['cluster']]
cluster = cloud('container', 'clusters', 'describe', snapshot['cluster'], '--region=asia-south1')
groups = cloud('compute', 'instance-groups', 'managed', 'list')
node_group_names = {url.rsplit('/', 1)[-1] for p in snapshot['nodePools'] for url in p['instanceGroupUrls']}
node_groups = [{'name': g['name'], 'targetSize': g.get('targetSize', 0)} for g in groups if g['name'] in node_group_names]
pools = [{'name': p['name'], 'autoscaling': p.get('autoscaling', {}).get('enabled', False)} for p in cluster['nodePools']]
result = {'databases': [i for db in databases for i in db], 'vms': vms, 'nodes': nodes, 'nodeGroups': node_groups, 'nodePools': pools}
result['verifiedDown'] = all(i['state'] == 'STOPPED' and i['activationPolicy'] == 'NEVER' for i in result['databases']) and len(vms) == len(vm_names) and all(v['state'] == 'TERMINATED' for v in vms) and not nodes and all(g['targetSize'] == 0 for g in node_groups) and all(not p['autoscaling'] for p in pools)
(root / 'verification.json').write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))
