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

export function faultCommandPreview(target){
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
