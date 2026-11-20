import {targetEnvironment,faultCapabilities,environmentLabel} from './environments.mjs';
export const faultTypes=[{type:'card-vault',label:'Card vault',adapter:'local-vault',port:3001},{type:'postgres',label:'PostgreSQL',adapter:'process-compose',port:5432},{type:'redis',label:'Redis',adapter:'process-compose',port:6379}];
// Older targets did not store a service type. Keep their IDs and connection settings.
export function faultType(target){
 if(target.serviceType)return target.serviceType;
 if(target.adapter==='gcp-alloydb')return 'postgres';
 if(target.adapter==='gcp-redis')return 'redis';
 if(target.adapter==='local-vault')return 'card-vault';
 if(target.adapter!=='process-compose')return null;
 return /redis/i.test(target.label||'')||[6378,6379].includes(target.port)||/^r\d+$/.test(target.process||'')?'redis':'postgres';
}

export function faultEnvironment(target){return targetEnvironment(target)||'unassigned';}
export function configureFaultTarget(target,environment){
 environment=environment==='cloud'?'gcp':environment;
 const type=faultType(target),spec=faultTypes.find(t=>t.type===type),capability=faultCapabilities[environment]?.[type];
 return {...target,environment,adapter:capability?.adapter||'placeholder',actions:capability?.actions||[],port:target.port??spec?.port,repository:target.repository??'',binary:target.binary??'process-compose',controllerPort:target.controllerPort??8090,process:target.process??(type==='redis'?'r1':'p1'),project:target.project||'',region:target.region||'',cluster:target.cluster||'',instance:target.instance||'',context:target.context??'',namespace:target.namespace||'',deployment:target.deployment||'',recoveryReplicas:target.recoveryReplicas??1,container:target.container??''};
}
export const faultTargetLabel=target=>`${target.label} · ${environmentLabel(faultEnvironment(target))}`;

// Describe the actual execution adapter rather than assuming every provider uses a CLI.
export function faultAccessNote(target){
 const location='on the console supervisor host';
 switch(target.adapter){
  case 'gcp-alloydb':
  case 'gcp-redis':return `Requires gcloud installed and authenticated ${location}, with access to the configured project and permissions to inspect the target and trigger failover. Check target is read-only; it does not prove failover permission.`;
  case 'kubernetes':return `Requires kubectl and any cluster credential plugin ${location}, the correct kubeconfig context, and permissions to inspect, scale and delete pods for the configured workload.`;
  case 'docker':return `Requires the Docker CLI ${location}, access to the Docker daemon, and permissions to inspect, stop and start the configured container.`;
  case 'process-compose':return `Requires access to the configured process-compose controller and process ${location}. The process-compose CLI must also be available for target checks. Stop/start permissions are required.`;
  case 'local-vault':return `Requires lsof and ps ${location}, access to the vault repository, and permission to stop and restart its process. Nix/Cargo must be available for a Nix-based repository, or provide a prebuilt locker binary.`;
  case 'command':return `Custom commands run through /bin/sh as the console supervisor's OS user. Install and authenticate any CLI they invoke (such as gcloud, aws or az) on that host, with permissions for the intended resources. Verification commands must be read-only.`;
  case 'placeholder':return `${environmentLabel(faultEnvironment(target))} fault execution is not supported yet.`;
  default:return '';
 }
}

export function faultCommandPreview(target){
 if(target.adapter==='command')return `Scheduled trigger: ${target.triggerCommand||'Not configured'}\nRecovery: ${target.recoveryMode==='self-recovering'?'Self-recovering':target.restoreCommand||'Not configured'}`;
 if(target.adapter==='gcp-alloydb')return `gcloud alloydb instances failover ${target.instance} --cluster=${target.cluster} --region=${target.region} --project=${target.project} --quiet`;
 if(target.adapter==='kubernetes'){
  const prefix=`kubectl${target.context?` --context=${target.context}`:''} -n ${target.namespace}`;
  const hard=(target.outageMode??(faultType(target)==='card-vault'?'hard':'graceful'))==='hard';
  return `Interruption: ${prefix} scale deploy ${target.deployment} --replicas=0${hard?`\n${prefix} delete pod <captured vault pod> --grace-period=0 --force --wait=false\nVerify: captured vault pods absent from kubelet runtime before starting outage duration`:''}\nRecovery: ${prefix} scale deploy ${target.deployment} --replicas=${target.recoveryReplicas ?? 'original'}`;
 }
 return '';
}

export function switchFaultEnvironment(target,environment){
 const {presets={},...current}=target;
 environment=environment==='cloud'?'gcp':environment;
 const saved={...presets,...(presets.cloud?{gcp:presets.cloud}:{}),[faultEnvironment(target)]:current};
 const next=configureFaultTarget({...saved[environment],id:target.id,label:target.label,serviceType:faultType(target)},environment);
 return {...next,presets:saved};
}

export function faultTargetResource(target){
 if(!target)return '';
 if(target.adapter==='local-vault')return `${target.repository} · port ${target.port}`;
 if(target.adapter==='process-compose')return `${target.process} · controller 127.0.0.1:${target.controllerPort} · service port ${target.port}`;
 if(target.adapter==='kubernetes')return [target.context||'Current Kubernetes context',`${target.namespace}/${target.deployment}`].join(' · ');
 if(target.adapter?.startsWith('gcp-'))return [target.project,target.region,target.cluster,target.instance].filter(Boolean).join(' · ');
 return target.container||target.resourceId||'';
}
