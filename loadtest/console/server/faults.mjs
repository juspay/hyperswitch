import {randomUUID} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile,open,rename,mkdir} from 'node:fs/promises';
import path from 'node:path';
import {dataRoot,readJson,readJsonSync,redact,controlDb,localSettings} from './state.mjs';
import {faultType,faultEnvironment} from '../src/fault-types.mjs';
import {localAdapter,processComposeController} from './local-faults.mjs';
import {atomicJson} from './state.mjs';
import {HttpError} from './errors.mjs';
import {queryWorker} from './workers.mjs';
import {watchDatabaseAvailability} from './database-availability.mjs';
import {databaseRecoveryGate} from './database-recovery-gate.mjs';
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
  if(t.adapter==='kubernetes'&&t.recoveryReplicas!==undefined&&(!Number.isInteger(t.recoveryReplicas)||t.recoveryReplicas<1||t.recoveryReplicas>100))throw new Error('Recovery replicas must be 1–100');
  if(['local-vault','process-compose'].includes(t.adapter)){if(!Number.isInteger(t.port)||t.port<1||t.port>65535)throw new Error('Invalid service port');if(t.adapter==='local-vault'){if(typeof t.repository!=='string'||!path.isAbsolute(t.repository))throw new Error('Use an absolute vault repository path');}else{word(t.process);if(typeof t.binary!=='string'||!(t.binary==='process-compose'||(path.isAbsolute(t.binary)&&path.basename(t.binary)==='process-compose')))throw new Error('Use process-compose or its absolute binary path');if(!Number.isInteger(t.controllerPort)||t.controllerPort<1||t.controllerPort>65535)throw new Error('Invalid controller port');}}
  const serviceType=faultType(t);if((t.adapter==='gcp-alloydb'&&serviceType!=='postgres')||(t.adapter==='gcp-redis'&&serviceType!=='redis'))throw new Error('Cloud adapter does not match service type');if(serviceType){if(!['card-vault','postgres','redis'].includes(serviceType)||(t.adapter==='process-compose'&&serviceType==='card-vault')||(t.adapter==='local-vault'&&t.serviceType&&t.serviceType!=='card-vault'))throw new Error('Invalid fault service type');if(types.has(serviceType)&&!allowLegacyDuplicates)throw new Error(`Only one ${serviceType} fault target is allowed`);types.add(serviceType);}
  return {...t,...(serviceType?{serviceType}:{}),label:String(t.label||t.id).slice(0,100),actions:t.adapter.startsWith('gcp-')?['failover']:['outage']};
 });
}
const targetsPath=path.join(dataRoot,'local-fault-targets.json');
export function localFaultTargets(){const saved=controlDb().prepare("SELECT value FROM console_preferences WHERE key='fault-targets'").get();return validateTargets(saved?JSON.parse(saved.value):readJsonSync(targetsPath,[]),{allowLegacyDuplicates:true});}
export function saveLocalFaultTargets(targets){if(state.active||state.recovery||state.blocked)throw new HttpError(409,'Restore the active fault before changing targets');const valid=validateTargets(targets);validateTargets([...serverTargets(),...valid]);controlDb().prepare("INSERT INTO console_preferences VALUES('fault-targets',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").run(JSON.stringify(valid));return valid;}
function serverTargets(){return validateTargets(process.env.LOADTEST_FAULT_TARGETS?readJsonSync(path.resolve(process.env.LOADTEST_FAULT_TARGETS),[]):[{id:'demo',label:'Simulation · no infrastructure changes',adapter:'simulated'}]);}
export function faultTargets(){const seen=new Set();return validateTargets([...serverTargets(),...localFaultTargets()].filter(t=>{const type=faultType(t);if(!type)return true;if(seen.has(type))return false;seen.add(type);return true;}));}
export async function checkLocalFaultTarget(input){
 const [target]=validateTargets([input]);
 try{await makeFaultAdapter(target,runCommandWithTimeout(12000)).probe();}
 catch(error){
  if(error.code==='COMMAND_TIMEOUT'){
   const service=target.adapter==='kubernetes'?'Kubernetes':target.adapter.startsWith('gcp-')?'Google Cloud':'Target';
   throw new Error(`${service} did not respond within 12 seconds. Check VPN and cloud access.`);
  }
  throw error;
 }
 return {ok:true,message:'Running and reachable · no changes made'};
}

export function faultStatus(){return {targets:faultTargets().map(t=>({id:t.id,label:t.label,adapter:t.adapter,actions:t.actions,environment:faultEnvironment(t),target:t})),active:state.active?{id:state.active.id,jobId:state.active.jobId,target:state.active.target.label,targetId:state.active.target.id,databaseAvailability:state.active.databaseAvailability,loadReleased:Boolean(state.active.loadReleased),recoveryAt:state.active.downtimeStartedAt?state.active.downtimeStartedAt+state.active.seconds*1000:null,occurred:Boolean(state.active.downtimeStartedAt),action:state.active.action,status:state.active.status,simulated:state.active.target.adapter==='simulated'}:null,blocked:state.blocked,recovering:Boolean(state.recovery)};}
async function durable(value){await mkdir(dataRoot,{recursive:true});const file=await open(ledgerPath+'.tmp','w',0o600);try{await file.writeFile(JSON.stringify(value));await file.sync();}finally{await file.close();}await rename(ledgerPath+'.tmp',ledgerPath);const directory=await open(dataRoot,'r');try{await directory.sync();}finally{await directory.close();}}
export const runCommandWithTimeout=timeout=>async(binary,args)=>{
 try{const {stdout}=await exec(binary,args,{timeout,maxBuffer:1024*1024,windowsHide:true});return stdout;}
 catch(error){if(error.killed&&error.signal==='SIGTERM')throw Object.assign(new Error(`Command timed out after ${timeout} ms`),{code:'COMMAND_TIMEOUT'});throw error;}
};
const runCommand=runCommandWithTimeout(120000);
export function makeFaultAdapter(target,command=runCommand,{databaseCheck=()=>queryWorker('database-test',{database:localSettings().database})}={}){
 const t=target;
 if(['local-vault','process-compose'].includes(t.adapter))return localAdapter(t,command,t.adapter==='process-compose'&&command===runCommand?{controller:processComposeController(t)}:{});
 if(t.adapter==='simulated')return {probe:async()=>({ready:true}),apply:async()=>{},restore:async()=>{},verify:async()=>true};
 if(t.adapter==='docker'){
  const inspect=async()=>JSON.parse(await command('docker',['inspect',t.container]))[0];
  return {probe:async()=>{const info=await inspect();if(!info.State.Running||(info.State.Health && info.State.Health.Status!=='healthy'))throw new Error('Container is not healthy');return {running:true,id:info.Id};},isDown:async before=>{const info=await inspect();return info.Id===before.id&&!info.State.Running;},apply:()=>command('docker',['stop','--time','30',t.container]),restore:async before=>{if((await inspect()).Id!==before.id)throw new Error('Container identity changed');await command('docker',['start',t.container]);},verify:async before=>{const info=await inspect();return info.Id===before.id && info.State.Running && (!info.State.Health || info.State.Health.Status==='healthy');}};
 }
 if(t.adapter==='kubernetes'){
  const flags=[...(t.context?['--context',t.context]:[]),'--namespace',t.namespace];
  const inspect=async()=>JSON.parse(await command('kubectl',[...flags,'get','deployment',t.deployment,'-o','json']));
  const recoveryReplicas=before=>t.recoveryReplicas ?? before.replicas;
  return {probe:async()=>{const info=await inspect();const replicas=info.spec.replicas;if(!(replicas>0)||info.status.availableReplicas!==replicas||info.status.observedGeneration<info.metadata.generation)throw new Error('Deployment is not ready');return {replicas,uid:info.metadata.uid};},isDown:async before=>{const info=await inspect();return info.metadata.uid===before.uid&&info.spec.replicas===0&&(info.status.replicas || 0)===0&&(info.status.availableReplicas || 0)===0&&info.status.observedGeneration>=info.metadata.generation;},apply:()=>command('kubectl',[...flags,'scale','deployment',t.deployment,'--replicas=0']),restore:async before=>{if((await inspect()).metadata.uid!==before.uid)throw new Error('Deployment identity changed');await command('kubectl',[...flags,'scale','deployment',t.deployment,`--replicas=${recoveryReplicas(before)}`]);},verify:async before=>{const info=await inspect(),replicas=recoveryReplicas(before);return info.metadata.uid===before.uid&&info.spec.replicas===replicas&&(info.status.availableReplicas||0)>=replicas&&(info.status.readyReplicas??info.status.availableReplicas??0)>=replicas&&info.status.observedGeneration>=info.metadata.generation;}};
 }
 const group=t.adapter==='gcp-redis'?'redis':'alloydb';const flags=['--project',t.project,'--region',t.region,...(group==='alloydb'?['--cluster',t.cluster]:[])];
 const inspect=async()=>JSON.parse(await command('gcloud',[group,'instances','describe',t.instance,...flags,'--format=json']));
 if(group==='alloydb'){
  const instanceName=`projects/${t.project}/locations/${t.region}/clusters/${t.cluster}/instances/${t.instance}`;
  const operationPrefix=`projects/${t.project}/locations/${t.region}/operations/`;
  return {
   recoveryTimeoutMs:600000,
   checkAvailability:()=>queryWorker('database-test',{database:localSettings().database,timeoutMillis:1000}),
   probe:async()=>{const info=await inspect();if(!info.createTime||info.state!=='READY'||info.availabilityType!=='REGIONAL'||info.instanceType!=='PRIMARY')throw new Error('Failover target must be a ready HA primary');return {ready:true,created:info.createTime};},
   apply:async(action,before,persist=async()=>{})=>{
    // Persist intent first: a lost CLI response must never turn READY into proof of failover.
    before.failoverRequested=true;await persist();
    let response;
    try{response=await command('gcloud',[group,'instances','failover',t.instance,...flags,'--quiet','--async','--format=json']);}
    catch(error){
     // Explicit API rejection proves no operation was accepted. A timeout or
     // lost response remains ambiguous and still requires recovery verification.
     if(/ERROR:.*\b(?:ABORTED|FAILED_PRECONDITION|INVALID_ARGUMENT|NOT_FOUND|PERMISSION_DENIED|UNAUTHENTICATED):/.test(error.message)){
      before.failoverRejected=redact(error.message);await persist();
     }
     throw error;
    }
    const operation=JSON.parse(response);
    if(!operation.name?.startsWith(operationPrefix)||!/^operation-[A-Za-z0-9-]+$/.test(operation.name.slice(operationPrefix.length)))throw new Error('Failover did not return a valid operation ID; completion is unverified');
    before.operation=operation.name;await persist();
   },
   restore:async()=>{},
   verify:async before=>{
    if(!before.operation)throw new Error('Failover operation ID is missing; cannot verify completion from instance readiness');
    if(!before.operation.startsWith(operationPrefix))throw new Error('Failover operation identity changed');
    const operation=JSON.parse(await command('gcloud',['alloydb','operations','describe',before.operation.slice(operationPrefix.length),'--project',t.project,'--region',t.region,'--format=json']));
    if(operation.name!==before.operation||operation.metadata?.target!==instanceName||operation.metadata?.verb!=='failover')throw new Error('Failover operation identity changed');
    if(!operation.done)return false;
    if(operation.error){before.failoverFailed=`AlloyDB failover failed: ${operation.error.message||operation.error.code||'unknown error'}`;return true;}
    const info=await inspect();
    if(info.createTime!==before.created||info.state!=='READY'||info.instanceType!=='PRIMARY'||info.availabilityType!=='REGIONAL')return false;
    try{return (await databaseCheck()).ok===true;}catch{return false;}
   }
  };
 }
 return {probe:async()=>{const info=await inspect();if(!info.createTime||info.state!=='READY'||(group==='redis'?info.tier!=='STANDARD_HA':info.availabilityType!=='REGIONAL'||info.instanceType!=='PRIMARY'))throw new Error('Failover target must be a ready HA primary');return {ready:true,created:info.createTime};},apply:()=>command('gcloud',[group,'instances','failover',t.instance,...flags,'--quiet',...(group==='redis'?['--data-protection-mode=limited-data-loss']:[])]),restore:async()=>{},verify:async before=>{const info=await inspect();return info.createTime===before.created&&info.state==='READY'&&(group==='redis'?info.tier==='STANDARD_HA':info.instanceType==='PRIMARY'&&info.availabilityType==='REGIONAL');}};
}
async function verify(adapter,before){const end=Date.now()+(adapter.recoveryTimeoutMs||120000);do{if(await adapter.verify(before))return;await new Promise(r=>setTimeout(r,adapter.recoveryTimeoutMs?5000:1000));}while(Date.now()<end);throw new Error('Recovery could not be verified before the deadline; the cloud operation may still be running');}
function observeDatabase(entry,emit,lifecycle={}){
 if(entry.target.adapter!=='gcp-alloydb')return;
 if(state.databaseObserver&&state.databaseObserver.id===entry.id)return;
 const last=controlDb().prepare("SELECT kind FROM job_events WHERE job_id=? AND kind IN ('database_available','database_unavailable') ORDER BY event_id DESC LIMIT 1").get(entry.jobId);
 const releaseLoad=databaseRecoveryGate(entry,async()=>{if(!lifecycle.onRecovered)return false;const released=await lifecycleStep(lifecycle.onRecovered,entry,emit,'Could not release the load clock after SQL recovery');if(released)emitSafely(emit,entry.jobId,'database_recovered','DB · available · load resumed; cloud operation verification continues');return released;});
 const observer=watchDatabaseAvailability({check:makeFaultAdapter(entry.target).checkAvailability,initial:last?.kind==='database_unavailable'?'unavailable':'available',onSample:async current=>{entry.databaseAvailability=current;if(state.active?.id===entry.id)state.active.databaseAvailability=current;await releaseLoad(current);},onChange:current=>emitSafely(emit,entry.jobId,`database_${current}`,`DB · ${current}`)});
 state.databaseObserver={id:entry.id,...observer};
}
async function lifecycleStep(callback,entry,emit,label){if(!callback)return true;let error;for(let attempt=0;attempt<3;attempt++){try{await callback();return true;}catch(e){error=e;if(attempt<2)await new Promise(resolve=>setTimeout(resolve,200));}}emitSafely(emit,entry.jobId,'fault_error',`${label}: ${redact(error.message)}`);return false;}
async function restore(entry,emit,lifecycle={}){
 if(entry.before?.failoverRejected&&!entry.before.operation){
  if(!await lifecycleStep(lifecycle.onAborted,entry,emit,'Could not release the load clock after rejected failover'))throw new Error('Could not release the load clock after rejected failover');
  if(state.databaseObserver&&state.databaseObserver.id===entry.id){await state.databaseObserver.stop();state.databaseObserver=null;}
  await durable(null);state.active=null;state.blocked=false;
  emitSafely(emit,entry.jobId,'fault_skipped',`${entry.target.label} · failover rejected · continuing`);return;
 }
 observeDatabase(entry,emit,lifecycle);await lifecycleStep(lifecycle.onRestore,entry,emit,'Could not hold the load clock for recovery');entry.status='restoring';state.active=entry;await durable(entry).catch(()=>{});emitSafely(emit,entry.jobId,'fault_restore',`${entry.target.label} · restoring`);const adapter=makeFaultAdapter(entry.target);await adapter.restore(entry.before);await verify(adapter,entry.before);if(state.databaseObserver&&state.databaseObserver.id===entry.id){await state.databaseObserver.stop();state.databaseObserver=null;}if(!entry.loadReleased&&!await lifecycleStep(entry.before?.failoverFailed?lifecycle.onAborted:lifecycle.onRecovered,entry,emit,'Could not release the load clock after recovery'))throw new Error('Infrastructure recovered, but the load clock could not be released');await durable(null);state.active=null;state.blocked=false;if(entry.before?.failoverFailed){emitSafely(emit,entry.jobId,'fault_error',entry.before.failoverFailed);emitSafely(emit,entry.jobId,'fault_skipped',`${entry.target.label} · failover failed · continuing`);}else emitSafely(emit,entry.jobId,'fault_restored',`${entry.target.label} · recovery verified`);}
export async function initializeFaults(emit){if(state.recovery)return state.recovery;let saved;try{saved=await readJson(ledgerPath,null);}catch(e){state.blocked=true;throw e;}if(!saved)return;state.blocked=true;state.active=saved;state.recovery=restore(saved,emit).catch(e=>{state.blocked=true;throw e;}).finally(()=>{state.recovery=null;});return state.recovery;}
export async function fireFault(job,input,emit,currentJob=()=>job,targetSnapshot=null,lifecycle={}){
 if(job.command!=='load'||job.loadState!=='running')throw new HttpError(409,'Faults require a running load');
 return (await executeFault(job,input,emit,()=>currentJob()?.loadState==='running',targetSnapshot,lifecycle)).status;
}
export async function testFault(job,input,emit){
 if(job.command!=='fault-test'||job.status!=='running')throw new HttpError(409,'Fault test is not running');
 const {completion}=await executeFault(job,input,emit,()=>true);
 return completion;
}
async function executeFault(job,input,emit,isRunning,targetSnapshot=null,lifecycle={}){
 if(state.active||state.blocked||state.recovery)throw new HttpError(409,'A fault is active or recovery is pending');
 const target=targetSnapshot || faultTargets().find(t=>t.id===input.targetId);if(!target||!target.actions.includes(input.action))throw new Error('Choose a configured fault target and action');
 const phrase=`${input.action} ${target.id}`;if(input.confirm!==phrase)throw new Error(`Type ${phrase} to confirm`);
 const seconds=Number(input.durationSeconds??30);if(!Number.isInteger(seconds)||seconds<1||seconds>300)throw new Error('Outage duration must be 1–300 seconds');
 const entry={id:randomUUID(),jobId:job.id,target,action:input.action,status:'preflight',before:null,seconds,at:Date.now()};state.active=entry;
 state.lifecycle=lifecycle;
 const adapter=makeFaultAdapter(target),abort=new AbortController();state.abort=abort;
 state.done=(async()=>{
  let armed=false,error=null;
  try{
    entry.before=await adapter.probe();
    if(abort.signal.aborted||!isRunning())throw new Error('Load stopped or paused during fault preflight');
    entry.status='armed';await durable(entry);armed=true;
    if(abort.signal.aborted)throw new Error('Fault cancelled before activation');
    entry.status='active';await durable(entry);
    observeDatabase(entry,emit,lifecycle);
    emitSafely(emit,job.id,'fault_requested',`${target.label} · ${input.action} requested`);
    await adapter.apply(input.action,entry.before,()=>durable(entry));
    if(input.action==='outage'&&adapter.isDown){
      const deadline=Date.now()+60000;while(!await adapter.isDown(entry.before)){if(abort.signal.aborted)throw new Error('Outage cancelled before taking effect');if(Date.now()>=deadline)throw new Error('Outage did not take effect');await new Promise(r=>setTimeout(r,500));}
    }
    entry.downtimeStartedAt=Date.now();await durable(entry);
    emitSafely(emit,job.id,'fault_started',`${target.adapter==='simulated'?'SIMULATED · ':''}${target.label} · ${input.action}`);
    if(!await lifecycleStep(lifecycle.onStarted,entry,emit,'Could not start the configured fault duration'))throw new Error('Load clock did not enter the fault duration');
    await new Promise(r=>{const timer=setTimeout(r,seconds*1000);abort.signal.addEventListener('abort',()=>{clearTimeout(timer);r();},{once:true});if(abort.signal.aborted){clearTimeout(timer);r();}});
  }catch(e){error=redact(e.message);emitSafely(emit,job.id,'fault_error',error);}
  finally{
    if(armed){try{await restore(entry,emit,lifecycle);}catch(e){state.active={...entry,status:'recovery_failed'};state.blocked=true;error=`Recovery failed: ${redact(e.message)}`;emitSafely(emit,job.id,'fault_error',error);}}
    else {state.active=null;await lifecycleStep(lifecycle.onAborted,entry,emit,'Could not release the load clock after fault cancellation');}
    state.abort=null;
    state.done=null;
    if(!state.blocked)state.lifecycle=null;
  }
  return {error,recovered:!state.blocked};
 })();return {status:faultStatus(),completion:state.done};
}
export async function restoreFaults(emit){
 if(state.done){const done=state.done;state.abort?.abort();await done;if(state.done===done)state.done=null;return faultStatus();}
 if(state.recovery){await state.recovery;return faultStatus();}
 if(state.active){state.blocked=true;state.recovery=restore(state.active,emit,state.lifecycle||{}).finally(()=>{state.recovery=null;if(!state.blocked)state.lifecycle=null;});await state.recovery;}
 return faultStatus();
}

export function requestRestore(emit){
 restoreFaults(emit).catch(e=>{state.blocked=true;if(state.active){state.active.status='recovery_failed';emitSafely(emit,state.active.jobId,'fault_error',`Recovery failed: ${redact(e.message)}`);}});
 return faultStatus();
}
