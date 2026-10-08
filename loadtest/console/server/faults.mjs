import {randomUUID} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile,open,rename,mkdir} from 'node:fs/promises';
import path from 'node:path';
import {dataRoot,readJson,readJsonSync,redact} from './state.mjs';
import {faultType} from '../src/fault-types.mjs';
import {localAdapter} from './local-faults.mjs';
import {atomicJson} from './state.mjs';
import {HttpError} from './errors.mjs';
const exec=promisify(execFile),key=Symbol.for('hyperswitch.loadtest.faults');
const state=globalThis[key] ||= {active:null,recovery:null,blocked:false};
function emitSafely(emit,...args){try{emit?.(...args);}catch{console.error('Fault activity storage unavailable; recovery continues');}}
const ledgerPath=path.join(dataRoot,'fault-ledger.json');
const word=(value)=>{if(typeof value!=='string'||!value.match(/^[A-Za-z0-9][A-Za-z0-9_.:-]{0,180}$/))throw new Error('Invalid fault target name');return value;};
export function validateTargets(targets,{allowLegacyDuplicates=false}={}){
 if(!Array.isArray(targets)||targets.length>20)throw new Error('Choose at most 20 fault targets');
 const ids=new Set(),types=new Set();return targets.map(t=>{
  if(t.environment&&!['local','cloud','custom'].includes(t.environment))throw new Error('Choose Local, Cloud or Custom');
  word(t.id);if(ids.has(t.id))throw new Error('Duplicate fault target');ids.add(t.id);
  if(!['simulated','docker','kubernetes','gcp-redis','gcp-alloydb','local-vault','process-compose'].includes(t.adapter))throw new Error('Unsupported fault adapter');
  const fields=t.adapter==='docker'?['container']:t.adapter==='kubernetes'?['namespace','deployment',...(t.context?['context']:[])]:t.adapter.startsWith('gcp-')?['project','region','instance',...(t.adapter==='gcp-alloydb'?['cluster']:[])]:[];
  for(const field of fields)word(t[field]);
  if(['local-vault','process-compose'].includes(t.adapter)){if(!Number.isInteger(t.port)||t.port<1||t.port>65535)throw new Error('Invalid service port');if(t.adapter==='local-vault'){if(typeof t.repository!=='string'||!path.isAbsolute(t.repository))throw new Error('Use an absolute vault repository path');}else{word(t.process);if(typeof t.binary!=='string'||!(t.binary==='process-compose'||(path.isAbsolute(t.binary)&&path.basename(t.binary)==='process-compose')))throw new Error('Use process-compose or its absolute binary path');if(!Number.isInteger(t.controllerPort)||t.controllerPort<1||t.controllerPort>65535)throw new Error('Invalid controller port');}}
  const serviceType=faultType(t);if((t.adapter==='gcp-alloydb'&&serviceType!=='postgres')||(t.adapter==='gcp-redis'&&serviceType!=='redis'))throw new Error('Cloud adapter does not match service type');if(serviceType){if(!['card-vault','postgres','redis'].includes(serviceType)||(t.adapter==='process-compose'&&serviceType==='card-vault')||(t.adapter==='local-vault'&&t.serviceType&&t.serviceType!=='card-vault'))throw new Error('Invalid fault service type');if(types.has(serviceType)&&!allowLegacyDuplicates)throw new Error(`Only one ${serviceType} fault target is allowed`);types.add(serviceType);}
  return {...t,...(serviceType?{serviceType}:{}),label:String(t.label||t.id).slice(0,100),actions:t.adapter.startsWith('gcp-')?['failover']:['outage','restart']};
 });
}
const targetsPath=path.join(dataRoot,'local-fault-targets.json');
export function localFaultTargets(){return validateTargets(readJsonSync(targetsPath,[]),{allowLegacyDuplicates:true});}
export async function saveLocalFaultTargets(targets){if(state.active||state.recovery||state.blocked)throw new HttpError(409,'Restore the active fault before changing targets');const valid=validateTargets(targets);validateTargets([...serverTargets(),...valid]);await atomicJson(targetsPath,valid);return valid;}
function serverTargets(){return validateTargets(process.env.LOADTEST_FAULT_TARGETS?readJsonSync(path.resolve(process.env.LOADTEST_FAULT_TARGETS),[]):[{id:'demo',label:'Simulation · no infrastructure changes',adapter:'simulated'}]);}
export function faultTargets(){const seen=new Set();return validateTargets([...serverTargets(),...localFaultTargets()].filter(t=>{const type=faultType(t);if(!type)return true;if(seen.has(type))return false;seen.add(type);return true;}));}
export async function checkLocalFaultTarget(input){const [target]=validateTargets([input]);await makeFaultAdapter(target).probe();return {ok:true,message:'Running and reachable · no changes made'};}

export function faultStatus(){return {targets:faultTargets().map(({id,label,adapter,actions})=>({id,label,adapter,actions})),active:state.active?{id:state.active.id,jobId:state.active.jobId,target:state.active.target.label,action:state.active.action,status:state.active.status,simulated:state.active.target.adapter==='simulated'}:null,blocked:state.blocked,recovering:Boolean(state.recovery)};}
async function durable(value){await mkdir(dataRoot,{recursive:true});const file=await open(ledgerPath+'.tmp','w',0o600);try{await file.writeFile(JSON.stringify(value));await file.sync();}finally{await file.close();}await rename(ledgerPath+'.tmp',ledgerPath);const directory=await open(dataRoot,'r');try{await directory.sync();}finally{await directory.close();}}
export function makeFaultAdapter(target,command=async(binary,args)=>{const {stdout}=await exec(binary,args,{timeout:120000,maxBuffer:1024*1024,windowsHide:true});return stdout;}){
 const t=target;
 if(['local-vault','process-compose'].includes(t.adapter))return localAdapter(t,command);
 if(t.adapter==='simulated')return {probe:async()=>({ready:true}),apply:async()=>{},restore:async()=>{},verify:async()=>true};
 if(t.adapter==='docker'){
  const inspect=async()=>JSON.parse(await command('docker',['inspect',t.container]))[0];
  return {probe:async()=>{const info=await inspect();if(!info.State.Running||(info.State.Health && info.State.Health.Status!=='healthy'))throw new Error('Container is not healthy');return {running:true,id:info.Id};},isDown:async before=>{const info=await inspect();return info.Id===before.id&&!info.State.Running;},apply:action=>command('docker',[action==='outage'?'stop':'restart','--time','30',t.container]),restore:async before=>{if((await inspect()).Id!==before.id)throw new Error('Container identity changed');await command('docker',['start',t.container]);},verify:async before=>{const info=await inspect();return info.Id===before.id && info.State.Running && (!info.State.Health || info.State.Health.Status==='healthy');}};
 }
 if(t.adapter==='kubernetes'){
  const flags=[...(t.context?['--context',t.context]:[]),'--namespace',t.namespace];
  const inspect=async()=>JSON.parse(await command('kubectl',[...flags,'get','deployment',t.deployment,'-o','json']));
  return {probe:async()=>{const info=await inspect();const replicas=info.spec.replicas;if(!(replicas>0)||info.status.availableReplicas!==replicas||info.status.observedGeneration<info.metadata.generation)throw new Error('Deployment is not ready');return {replicas,uid:info.metadata.uid};},isDown:async before=>{const info=await inspect();return info.metadata.uid===before.uid&&info.spec.replicas===0&&(info.status.replicas || 0)===0&&(info.status.availableReplicas || 0)===0&&info.status.observedGeneration>=info.metadata.generation;},apply:action=>command('kubectl',[...flags,...(action==='outage'?['scale','deployment',t.deployment,'--replicas=0']:['rollout','restart','deployment',t.deployment])]),restore:async before=>{if((await inspect()).metadata.uid!==before.uid)throw new Error('Deployment identity changed');await command('kubectl',[...flags,'scale','deployment',t.deployment,`--replicas=${before.replicas}`]);},verify:async before=>{const info=await inspect();return info.metadata.uid===before.uid&&info.spec.replicas===before.replicas&&info.status.availableReplicas===before.replicas&&info.status.updatedReplicas===before.replicas&&(info.status.replicas ?? before.replicas)===before.replicas&&info.status.observedGeneration>=info.metadata.generation;}};
 }
 const group=t.adapter==='gcp-redis'?'redis':'alloydb';const flags=['--project',t.project,'--region',t.region,...(group==='alloydb'?['--cluster',t.cluster]:[])];
 const inspect=async()=>JSON.parse(await command('gcloud',[group,'instances','describe',t.instance,...flags,'--format=json']));
 return {probe:async()=>{const info=await inspect();if(!info.createTime||info.state!=='READY'||(group==='redis'?info.tier!=='STANDARD_HA':info.availabilityType!=='REGIONAL'||info.instanceType!=='PRIMARY'))throw new Error('Failover target must be a ready HA primary');return {ready:true,created:info.createTime};},apply:()=>command('gcloud',[group,'instances','failover',t.instance,...flags,'--quiet',...(group==='redis'?['--data-protection-mode=limited-data-loss']:[])]),restore:async()=>{},verify:async before=>{const info=await inspect();return info.createTime===before.created&&info.state==='READY'&&(group==='redis'?info.tier==='STANDARD_HA':info.instanceType==='PRIMARY'&&info.availabilityType==='REGIONAL');}};
}
async function verify(adapter,before){const end=Date.now()+120000;do{if(await adapter.verify(before))return;await new Promise(r=>setTimeout(r,1000));}while(Date.now()<end);throw new Error('Recovery could not be verified');}
async function restore(entry,emit){entry.status='restoring';state.active=entry;await durable(entry).catch(()=>{});emitSafely(emit,entry.jobId,'fault_restore',`${entry.target.label} · restoring`);const adapter=makeFaultAdapter(entry.target);await adapter.restore(entry.before);await verify(adapter,entry.before);await durable(null);state.active=null;state.blocked=false;emitSafely(emit,entry.jobId,'fault_restored',`${entry.target.label} · recovery verified`);}
export async function initializeFaults(emit){if(state.recovery)return state.recovery;let saved;try{saved=await readJson(ledgerPath,null);}catch(e){state.blocked=true;throw e;}if(!saved)return;state.blocked=true;state.active=saved;state.recovery=restore(saved,emit).catch(e=>{state.blocked=true;throw e;}).finally(()=>{state.recovery=null;});return state.recovery;}
export async function fireFault(job,input,emit,currentJob=()=>job,targetSnapshot=null){
 if(job.command!=='load'||job.loadState!=='running')throw new HttpError(409,'Faults require a running load');
 return (await executeFault(job,input,emit,()=>currentJob()?.loadState==='running',targetSnapshot)).status;
}
export async function testFault(job,input,emit){
 if(job.command!=='fault-test'||job.status!=='running')throw new HttpError(409,'Fault test is not running');
 const {completion}=await executeFault(job,input,emit,()=>true);
 return completion;
}
async function executeFault(job,input,emit,isRunning,targetSnapshot=null){
 if(state.active||state.blocked||state.recovery)throw new HttpError(409,'A fault is active or recovery is pending');
 const target=targetSnapshot || faultTargets().find(t=>t.id===input.targetId);if(!target||!target.actions.includes(input.action))throw new Error('Choose a configured fault target and action');
 const phrase=`${input.action} ${target.id}`;if(input.confirm!==phrase)throw new Error(`Type ${phrase} to confirm`);
 const seconds=Number(input.durationSeconds??30);if(!Number.isInteger(seconds)||seconds<1||seconds>300)throw new Error('Outage duration must be 1–300 seconds');
 const entry={id:randomUUID(),jobId:job.id,target,action:input.action,status:'preflight',before:null,seconds,at:Date.now()};state.active=entry;
 const adapter=makeFaultAdapter(target),abort=new AbortController();state.abort=abort;
 state.done=(async()=>{
  let armed=false,error=null;
  try{
    entry.before=await adapter.probe();
    if(abort.signal.aborted||!isRunning())throw new Error('Load stopped or paused during fault preflight');
    entry.status='armed';await durable(entry);armed=true;
    if(abort.signal.aborted)throw new Error('Fault cancelled before activation');
    entry.status='active';await durable(entry);
    emitSafely(emit,job.id,'fault_requested',`${target.label} · ${input.action} requested`);
    await adapter.apply(input.action,entry.before);
    if(input.action==='outage'&&adapter.isDown){
      const deadline=Date.now()+60000;while(!await adapter.isDown(entry.before)){if(abort.signal.aborted)throw new Error('Outage cancelled before taking effect');if(Date.now()>=deadline)throw new Error('Outage did not take effect');await new Promise(r=>setTimeout(r,500));}
    }
    emitSafely(emit,job.id,'fault_started',`${target.adapter==='simulated'?'SIMULATED · ':''}${target.label} · ${input.action}`);
    if(input.action==='outage')await new Promise(r=>{const timer=setTimeout(r,seconds*1000);abort.signal.addEventListener('abort',()=>{clearTimeout(timer);r();},{once:true});if(abort.signal.aborted){clearTimeout(timer);r();}});
  }catch(e){error=redact(e.message);emitSafely(emit,job.id,'fault_error',error);}
  finally{
    if(armed){try{await restore(entry,emit);}catch(e){state.active={...entry,status:'recovery_failed'};state.blocked=true;error=`Recovery failed: ${redact(e.message)}`;emitSafely(emit,job.id,'fault_error',error);}}
    else state.active=null;
    state.abort=null;
    state.done=null;
  }
  return {error,recovered:!state.blocked};
 })();return {status:faultStatus(),completion:state.done};
}
export async function restoreFaults(emit){
 if(state.done){const done=state.done;state.abort?.abort();await done;if(state.done===done)state.done=null;return faultStatus();}
 if(state.recovery){await state.recovery;return faultStatus();}
 if(state.active){state.blocked=true;state.recovery=restore(state.active,emit).finally(()=>{state.recovery=null;});await state.recovery;}
 return faultStatus();
}

export function requestRestore(emit){
 restoreFaults(emit).catch(e=>{state.blocked=true;if(state.active){state.active.status='recovery_failed';emitSafely(emit,state.active.jobId,'fault_error',`Recovery failed: ${redact(e.message)}`);}});
 return faultStatus();
}
