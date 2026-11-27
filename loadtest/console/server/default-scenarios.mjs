import {defaultCriteria} from './acceptance.mjs';
import {loadShape} from '../src/load-shape.mjs';

const definitions=[
 {key:'card-vault-outage',name:'Card vault outage',service:'card-vault',action:'outage'},
 {key:'db-failover',name:'DB failover',service:'postgres',action:'failover',environment:'gcp'},
 {key:'steady-load',name:'Steady load'},
 {key:'ramped-load',name:'Ramped load',ramped:true},
 {key:'tenancy-steady-load',name:'Tenancy steady load',mode:'tenancy',merchantCount:100}
];
export const isDefaultScenario=id=>definitions.some(d=>id===`default:${d.key}`);

// Built-ins are versioned definitions, never user-owned database rows. Target
// identities are resolved from the selected environment and snapshotted at launch.
export function defaultScenarios(environment,targets=[]){
 return definitions.map(definition=>{
  const target=targets.find(t=>t.serviceType===definition.service&&t.environment===environment&&t.adapter!=='simulated'&&t.actions?.includes(definition.action));
  const unavailableReason=definition.environment&&environment!==definition.environment?'Requires GCP database failover.':!['local','gcp'].includes(environment)?'Provider execution is not available.':definition.service&&!target?`Configure a ${definition.service==='postgres'?'PostgreSQL':'card vault'} ${definition.action} fault target for this environment.`:null;
  const stages=definition.service?[
   {target_rps:5,duration_seconds:60},
   {target_rps:5,duration_seconds:30,fault:{targetId:target?.id||definition.service,action:definition.action}},
   {target_rps:5,duration_seconds:210}
  ]:definition.ramped?[
   {target_rps:5,duration_seconds:60},{target_rps:10,duration_seconds:60},
   {target_rps:10,duration_seconds:60},{target_rps:5,duration_seconds:60},{target_rps:5,duration_seconds:60}
  ]:null;
  const load=stages?{start_rps:5,stages}:{total_rps:5,duration_seconds:300};
  const shape=loadShape(load);
  return {id:`default:${definition.key}`,version:definition.ramped?3:2,name:definition.name,isDefault:true,archived:false,unavailableReason,mode:definition.mode||'single',merchantCount:definition.merchantCount||1,totalRps:shape.peak,durationSeconds:shape.duration,loadShape:stages?'staged':'flat',startRps:5,stagesJson:JSON.stringify(stages||[]),load,acceptance:{...defaultCriteria},preAllocatedVusMultiplier:3,maxVusMultiplier:10,scenarios:[
   {name:'cit_off_session',scenario:'cit_off_session',merchant_path:'modular',weight:50},
   {name:'sdk_checkout',scenario:'sdk_checkout',merchant_path:'modular',weight:30},
   {name:'guest',scenario:'guest',merchant_path:'modular',weight:20}
  ]};
 });
}
