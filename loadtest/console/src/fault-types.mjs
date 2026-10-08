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

export function faultEnvironment(target){return target.environment|| (target.adapter.startsWith('gcp-')?'cloud':['local-vault','process-compose'].includes(target.adapter)?'local':'custom');}
export function configureFaultTarget(target,environment){
 const type=faultType(target),spec=faultTypes.find(t=>t.type===type);
 const cloudVault=environment==='cloud'&&type==='card-vault';
 const adapter=environment==='local'?spec.adapter:environment==='cloud'?(type==='postgres'?'gcp-alloydb':type==='redis'?'gcp-redis':'kubernetes'):'docker';
 return {...target,environment,adapter,port:target.port??spec.port,repository:target.repository??'',binary:target.binary??'process-compose',controllerPort:target.controllerPort??8090,process:target.process??(type==='redis'?'r1':'p1'),project:target.project||'hyperswitch-dev',region:target.region||'asia-south1',cluster:target.cluster||'dev-hyperswitch-alloydb',instance:target.instance||(type==='postgres'?'dev-hyperswitch-alloydb-primary':''),context:target.context??'',namespace:target.namespace||(cloudVault?'hyperswitch':''),deployment:target.deployment||(cloudVault?'hyperswitch-card-vault-hyperswitch-card-vault':''),container:target.container??''};
}

export function faultCommandPreview(target){
 if(target.adapter==='gcp-alloydb')return `gcloud alloydb instances failover ${target.instance} --cluster=${target.cluster} --region=${target.region} --project=${target.project} --quiet`;
 if(target.adapter==='kubernetes'){
  const prefix=`kubectl${target.context?` --context=${target.context}`:''} -n ${target.namespace}`;
  return `Outage: ${prefix} scale deploy ${target.deployment} --replicas=0\nRestart: ${prefix} rollout restart deploy ${target.deployment}`;
 }
 return '';
}
