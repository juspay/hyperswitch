export const environments=[
 {id:'local',label:'Local',supported:true},
 {id:'gcp',label:'GCP',supported:true},
 {id:'aws',label:'AWS',supported:true},
 {id:'azure',label:'Azure',supported:false}
];
export const environmentLabel=id=>environments.find(e=>e.id===id)?.label||'Unassigned';
export function validateEnvironment(id){if(!environments.some(e=>e.id===id))throw new Error('Choose Local, GCP, AWS or Azure');return id;}
export function requireSupportedEnvironment(id){validateEnvironment(id);if(!environments.find(e=>e.id===id).supported)throw new Error(`${environmentLabel(id)} execution is not supported; select Local, GCP or AWS to execute tests`);}
// Kubernetes alone does not identify a cloud provider. Explicit legacy Cloud
// targets were GCP in the previous editor; unclassified custom targets stay inert.
export function targetEnvironment(target){
 if(environments.some(e=>e.id===target.environment))return target.environment;
 if(target.environment==='cloud'||target.adapter?.startsWith('gcp-'))return 'gcp';
 if(target.adapter?.startsWith('aws-'))return 'aws';
 if(['local-vault','process-compose'].includes(target.adapter))return 'local';
 return null;
}
export const faultCapabilities={
 local:{'card-vault':{adapter:'local-vault',actions:['outage']},postgres:{adapter:'process-compose',actions:['outage']},redis:{adapter:'process-compose',actions:['outage']}},
 gcp:{'card-vault':{adapter:'kubernetes',actions:['outage']},postgres:{adapter:'gcp-alloydb',actions:['failover']},redis:{adapter:'gcp-redis',actions:['failover']}},
 aws:{postgres:{adapter:'aws-rds',actions:['failover']},redis:{adapter:'aws-elasticache',actions:['failover']}},azure:{}
};
export function validateTargetProvider(target,type){
 const environment=targetEnvironment(target);
 if(target.adapter==='simulated')return environment;
 if(target.adapter==='command'){requireSupportedEnvironment(environment);if(!['card-vault','postgres','redis'].includes(type))throw new Error('Choose a custom fault service type');return environment;}
 if(!environment)throw new Error('Assign a provider to this legacy fault target before using it');
 requireSupportedEnvironment(environment);
 const capability=faultCapabilities[environment][type];
 if(!capability||capability.adapter!==target.adapter)throw new Error(`${environmentLabel(environment)} does not support this fault adapter for ${type}`);
 return environment;
}

export function environmentAccessNote(environment){
 if(environment==='gcp')return 'GCP actions run on the console supervisor. Authenticate gcloud there and grant access to inspect and fail over the selected resources. Kubernetes actions also need the correct context and workload permissions. A successful read-only check does not prove permission to trigger a failover.';
 if(environment==='aws')return 'AWS actions run on the console supervisor. Install AWS CLI and authenticate the configured account, region and optional profile there. RDS requires a Multi-AZ PostgreSQL DB instance; ElastiCache requires automatic failover and a replica. Read-only target checks do not prove failover permission.';
 if(environment==='local')return 'Local commands run on the console supervisor, not in your browser. Service paths and process controllers must be accessible from that machine.';
 return 'Execution is not supported for this environment yet.';
}
