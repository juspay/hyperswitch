import {getProfile,fixtureSignature,validateFixtureSet,listFixtureSets} from './profiles.mjs';
import {DatabaseSync} from 'node:sqlite';
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir,appendFile } from 'node:fs/promises';
import path from 'node:path';
import { controlDb, initializeConfig, readJsonSync, readJson, atomicJson, dataRoot, configPath, localSettings, consoleRoot, loadtestRoot, redact, runDirectory } from './state.mjs';
import { queryWorker } from './workers.mjs';
import { HttpError } from './errors.mjs';
import { appendTelemetry,telemetryPoint } from './telemetry.mjs';
import {loadShape,loadTimeline} from '../src/load-shape.mjs';
import {initializeFaults,restoreFaults,faultStatus,faultTargets,fireFault,testFault} from './faults.mjs';
import {snapshotFaultPlan,faultScheduler} from './fault-plan.mjs';
import {workerFailure} from './worker-result.mjs';

// Share lifecycle state with adapter-node and Vite's server-module reloads.
const key=Symbol.for('hyperswitch.loadtest.jobs');
const manager=globalThis[key] ||= {running:new Map(),initialized:false,shuttingDown:false,recovering:false};
const running=manager.running;
export function initializeJobs() {
  if(manager.initialized)return;
  initializeConfig();const db=controlDb();
  const interrupted=db.prepare("SELECT id,run_path FROM jobs WHERE status IN ('starting','running','pausing','paused','cancelling')").all();
  db.prepare("UPDATE jobs SET status='interrupted',ended_at=?,error='Server restarted before job finished' WHERE status IN ('starting','running','pausing','paused','cancelling')").run(Date.now());
  db.prepare("UPDATE fixture_sets SET status='interrupted' WHERE status='creating'").run();
  manager.initialized=true;
  manager.recovering=true;
  manager.recovery=Promise.all([initializeFaults(recordActivity),...interrupted.map(stopInterrupted)]).catch(e=>{console.error('Recovery pending:',redact(e.message));}).finally(()=>{manager.recovering=false;});
  process.once('SIGTERM',()=>{shutdownJobs().catch(console.error);});
  process.once('SIGINT',()=>{shutdownJobs().catch(console.error);});
}
async function stopInterrupted({id,run_path}) {
  const directory=path.join(dataRoot,'jobs',id),processes=[];
  for(const {file,group} of [{file:'worker-pid.json',group:false},{file:'k6-pid.json',group:true}]) {
    const saved=await readJson(path.join(directory,file),null).catch(()=>null);
    if(Number.isSafeInteger(saved?.pid)&&saved.pid>1)processes.push(group?-saved.pid:saved.pid);
  }
  const signal=(pid,name)=>{try{process.kill(pid,name);return true;}catch(e){return e.code!=='ESRCH';}};
  for(const pid of processes)signal(pid,'SIGTERM');
  const deadline=Date.now()+60000;
  while(processes.some(pid=>signal(pid,0))) {
    if(Date.now()>=deadline){for(const pid of processes)signal(pid,'SIGKILL');break;}
    await new Promise(resolve=>setTimeout(resolve,250));
  }
  if(run_path){
    let db;try{db=new DatabaseSync(path.join(run_path,'results.sqlite'));db.exec('PRAGMA busy_timeout=5000');db.prepare("UPDATE runs SET recording_state='incomplete',error='Worker interrupted before job finished',ended_at=COALESCE(ended_at,?)").run(Date.now());}catch{/* A worker may have crashed before creating results. */}finally{db?.close();}
  }
}
const parsed=value=>JSON.parse(value || '{}');
export function getJob(id) {
  const row=controlDb().prepare('SELECT * FROM jobs WHERE id=?').get(id);
  if(!row) throw new HttpError(404,'Job not found');
  const config=row.config_path?readJsonSync(row.config_path,{}):{};
  return {id:row.id,command:row.kind,mode:row.mode,status:row.status,startedAt:row.started_at,endedAt:row.ended_at,
    exitCode:row.exit_code,runId:row.run_path?`${row.mode}:${path.basename(row.run_path)}`:undefined,
    fixtureSetId:config.fixtureSet?.id,
    profile:config.load?{id:config.profile?.id,version:config.profile?.version,name:config.profile?.name,totalRps:loadShape(config.load).peak,durationSeconds:loadShape(config.load).duration,timeline:loadTimeline(config.load,config.faultPlan),scenarios:config.scenarios}:null,
    loadState: row.kind==='load' ? (['complete','failed','cancelled','interrupted'].includes(row.status)?'completed':row.status==='cancelling'?'pausing':parsed(row.progress).control?.state==='draining'?'pausing':parsed(row.progress).control?.state || (['starting','running'].includes(row.status)?'idle':row.status)) : null,
    events:controlDb().prepare('SELECT event_id AS id,at,kind,message FROM job_events WHERE job_id=? ORDER BY event_id DESC LIMIT 200').all(id).reverse(),
    faults:faultStatus(),output:row.output,progress:parsed(row.progress),result:row.result?parsed(row.result):null,error:row.error==='Worker exited 99'?workerFailure(99):row.error};
}
export function latestReconciliation(runPath) {
  const row=controlDb().prepare("SELECT id FROM jobs WHERE kind='reconcile' AND run_path=? ORDER BY started_at DESC LIMIT 1").get(runPath);return row?getJob(row.id):null;
}
export function activeJob(kind=null) {
  initializeJobs();const row=controlDb().prepare("SELECT id FROM jobs WHERE status IN ('starting','running','pausing','paused','cancelling') AND (? IS NULL OR kind=? OR (?='generator' AND kind IN ('fixtures','load'))) ORDER BY started_at DESC LIMIT 1").get(kind,kind,kind);
  return row?getJob(row.id):null;
}
function reserve(kind,mode,runPath=null) {
  initializeJobs();if(manager.shuttingDown) throw new Error('Server is shutting down');
  if(kind!=='reconcile'&&(faultStatus().blocked||faultStatus().active||faultStatus().recovering))throw new HttpError(409,'Fault recovery must be verified before starting another run');
  if(manager.recovering)throw new HttpError(409,'Previous workers are stopping after restart; retry shortly');
  if(kind==='fault-test'&&activeJob('generator'))throw new HttpError(409,'Stop the fixture or load job before testing a fault');
  if(['fixtures','load','fault-test'].includes(kind)&&activeJob('fault-test'))throw new HttpError(409,'A fault test is already running');
  const db=controlDb(),id=randomUUID();
  if(kind==='reconcile' && db.prepare("SELECT id FROM jobs WHERE kind='reconcile' AND status IN ('starting','running','pausing','paused','cancelling')").get()) throw new HttpError(409,'Reconciliation is already running');
  try {db.prepare("INSERT INTO jobs(id,kind,mode,status,started_at,run_path) VALUES(?,?,?,'starting',?,?)").run(id,kind,mode,Date.now(),runPath);}
  catch(e) {if(e.code==='ERR_SQLITE_ERROR')throw new HttpError(409,'A fixture or load job is already running');throw e;}
  return id;
}
function update(id,values) {
  const columns=Object.keys(values);controlDb().prepare(`UPDATE jobs SET ${columns.map(k=>`${k}=?`).join(',')} WHERE id=?`).run(...Object.values(values),id);
}
export function recordActivity(id,kind,message) {
  const at=Date.now(),row=controlDb().prepare('SELECT run_path FROM jobs WHERE id=?').get(id);
  const event={id:Number(controlDb().prepare('INSERT INTO job_events(job_id,at,kind,message) VALUES(?,?,?,?)').run(id,at,kind,redact(message)).lastInsertRowid),at,kind,message:redact(message)};
  if(row?.run_path)appendFile(path.join(row.run_path,'activity.jsonl'),JSON.stringify(event)+'\n',{mode:0o600}).catch(()=>{});
  return event;
}
export async function controlLoad(id,action) {
  const job=getJob(id),worker=running.get(id);
  if(job.command!=='load'||!worker)throw new HttpError(409,'No active load worker');
  if(action==='pause'&&faultStatus().active)throw new HttpError(409,'Wait for fault recovery before pausing');
  if((action==='pause'&&job.loadState!=='running')||(action==='resume'&&job.loadState!=='paused'))throw new HttpError(409,'Action is not available in this state');
  const endpoint=(await readJson(path.join(worker.runPath,'control.json'),{})).endpoint;
  if(!endpoint)throw new HttpError(409,'Load worker is still initializing');
  const u=new URL(endpoint);if(u.hostname!=='127.0.0.1'||u.pathname!=='/control')throw new Error('Invalid worker control endpoint');
  const response=await fetch(u,{method:'POST',headers:{authorization:`Bearer ${worker.controlToken}`,'content-type':'application/json'},body:JSON.stringify({action}),signal:AbortSignal.timeout(5000)});
  if(!response.ok)throw new HttpError(409,await response.text());
  const control=await response.json();worker.revision=(worker.revision || 0)+1;update(id,{status:control.state,progress:JSON.stringify({...job.progress,control})});
  recordActivity(id,action,action==='pause'?'Pause requested · no new flows':'Load resumed');
  if(action==='pause'&&control.state==='paused')recordActivity(id,'paused','Load paused · active flows drained');
  return getJob(id);
}
export async function startGenerator(kind,mode,selection={}) {
  if(!['fixtures','load'].includes(kind) || !['single','tenancy'].includes(mode)) throw new Error('Invalid job');
  const config=structuredClone(readJsonSync(configPath));
  const profile=selection.profileId?getProfile(selection.profileId,selection.version):null;
  if(profile){mode=profile.mode;config.load=profile.load;config.scenarios=profile.scenarios;config.acceptance=profile.acceptance;config.profile={id:profile.id,version:profile.version,name:profile.name};}
  const count=profile?.merchantCount ?? (mode==='single'?1:1500);
  let fixtureSet,fixtureSetId;
  if(kind==='load') {
    const available=listFixtureSets();
    const chosen=selection.fixtureSetId || available.find(s=>s.compatible&&s.mode===mode&&s.merchant_count===count&&s.status==='ready')?.id;
    if(!chosen)throw new HttpError(409,'Create a ready, compatible fixture set first');
    fixtureSet=validateFixtureSet(chosen,{mode,merchantCount:count},config);fixtureSetId=fixtureSet.id;
  }
  const faultPlan=kind==='load'?snapshotFaultPlan(config.load,faultTargets()):[];
  config.faultPlan=faultPlan;
  const id=reserve(kind,mode);
  try {
    const jobDir=path.join(dataRoot,'jobs',id);await mkdir(jobDir,{recursive:true,mode:0o700});
    config.mode=mode;
    if(kind==='fixtures') {
      const signature=fixtureSignature(config,mode,count);
      fixtureSet=controlDb().prepare("SELECT * FROM fixture_sets WHERE signature=? AND status IN ('failed','interrupted') ORDER BY created_at DESC LIMIT 1").get(signature);
      fixtureSetId=fixtureSet?.id || randomUUID();
      const directory=fixtureSet?.path || path.join(config.output,mode,'fixture-sets',fixtureSetId);
      if(fixtureSet)controlDb().prepare("UPDATE fixture_sets SET status='creating',job_id=? WHERE id=?").run(id,fixtureSetId);
      else controlDb().prepare('INSERT INTO fixture_sets VALUES(?,?,?,?,?,?,?,?)').run(fixtureSetId,mode,count,signature,directory,'creating',id,Date.now());
      fixtureSet={id:fixtureSetId,path:directory};
    }
    config.fixtureSet={id:fixtureSetId,merchantCount:count};
    const runPath=kind==='load'?path.join(config.output,mode,'runs',`${new Date().toISOString().replace(/[:.]/g,'-')}-${mode}-${loadShape(config.load).peak}tps-${id.slice(0,8)}`):null;
    if(runPath)await mkdir(runPath,{recursive:true,mode:0o700});
    const snapshot=path.join(jobDir,'config.json');await atomicJson(snapshot,config);
    const spec={id,kind,mode,configPath:snapshot,runPath,loadtestRoot,parentPid:process.pid,fixturePath:fixtureSet.path,merchantCount:count,fixtureSetId};
    await atomicJson(path.join(jobDir,'job.json'),spec);update(id,{config_path:snapshot,run_path:runPath,status:'running'});
    const controlToken=randomUUID();
    let output='',lastWrite=0,lastElapsed=-1,lastPoint=null,forceTimer;
    const remember=async progress=>{
      if(runPath && Number.isFinite(progress.elapsed_seconds) && progress.elapsed_seconds>lastElapsed){
        lastElapsed=progress.elapsed_seconds;lastPoint=telemetryPoint(progress,lastPoint);await appendTelemetry(runPath,lastPoint);
      }
    };
    const fixtureProgress=async()=>({fixtures:{completed:(await readJson(path.join(fixtureSet.path,'merchants.json'),[])).length,total:count},setup:await readJson(path.join(jobDir,'fixture-progress.json'),{stage:'merchants',superposition:{status:config.fixtures.superposition?'pending':'skipped'}})});
    const child=spawn(process.execPath,[path.join(consoleRoot,'scripts','execute.mjs'),path.join(jobDir,'job.json')],{
      cwd:consoleRoot,stdio:['ignore','pipe','pipe'],env:{...process.env,LOADTEST_JOB_DIR:jobDir,SQLITE_RECORDER_CONTROL_TOKEN:controlToken,SQLITE_RECORDER_PROGRESS_FILE:runPath?path.join(runPath,'progress.json'):''}
    });
    recordActivity(id,'started',`${kind==='load'?'Load':'Fixture setup'} started · ${mode} · ${loadShape(config.load).peak} TPS`);
    const log=(data)=>{output=(output+redact(data)).slice(-50000);};
    child.stdout.on('data',log);child.stderr.on('data',log);
    const schedule=faultScheduler(faultPlan,{claim:index=>{const key=`scheduled step ${index+1}`;if(controlDb().prepare("SELECT 1 FROM job_events WHERE job_id=? AND kind='scheduled_fault' AND message=?").get(id,key))return false;recordActivity(id,'scheduled_fault',key);return true;},fire:(job,input,target)=>fireFault(job,input,recordActivity,()=>getJob(id),target),event:(kind,message)=>recordActivity(id,kind,message)});
    const tick=setInterval(async()=>{
      if(Date.now()-lastWrite<(faultPlan.length?200:900))return;lastWrite=Date.now();
      try {
        const worker=running.get(id),revision=worker?.revision || 0;
        const progress=runPath?await readJson(path.join(runPath,'progress.json'),{}):await fixtureProgress();
        const endpoint=runPath?(await readJson(path.join(runPath,'control.json'),{})).endpoint:null;
        if(endpoint && worker){
          const u=new URL(endpoint);if(u.hostname!=='127.0.0.1'||u.pathname!=='/control')throw new Error('Invalid control endpoint');
          try{const r=await fetch(u,{headers:{authorization:`Bearer ${controlToken}`},signal:AbortSignal.timeout(750)});if(r.ok)progress.control=await r.json();}catch{/* Worker may be exiting. */}
        }
        if((worker?.revision || 0)!==revision)return; // A newer pause/resume response wins.
        await remember(progress);progress.trend=lastPoint;
        const before=getJob(id),state=progress.control?.state,values={output,progress:JSON.stringify(progress)};
        if(['running','pausing','paused'].includes(state)&&before.status!=='cancelling')values.status=state;
        if(state&&state!==before.progress.control?.state&&['pausing','paused','completed'].includes(state))recordActivity(id,state,state==='paused'?'Load paused · active flows drained':state==='pausing'?'Pausing · draining active flows':'Active load duration reached');
        update(id,values);
        if(kind==='load')await schedule(getJob(id),progress.control?.active_seconds);
      }
      catch(e) {log(`\nProgress unavailable: ${e.message}`);}
    },faultPlan.length?250:1000);
    const done=new Promise(resolve=>{
      child.once('error',e=>{log(e.message);update(id,{status:'failed',error:redact(e.message),ended_at:Date.now(),output});});
      child.once('close',async(code,signal)=>{
        clearInterval(tick);clearTimeout(forceTimer);
        if(faultStatus().active?.jobId===id)await restoreFaults(recordActivity).catch(e=>log(`\nRecovery pending: ${e.message}`));
        if(runPath)try{await remember(await readJson(path.join(runPath,'progress.json'),{}));}catch(e){log(`\nTrend unavailable: ${e.message}`);}
        if(kind==='fixtures')try{update(id,{progress:JSON.stringify(await fixtureProgress())});}catch{}
        const previous=getJob(id);
        let result=await readJson(path.join(jobDir,'result.json'),null).catch(()=>null);
        const summary=code===99 && runPath?await readJson(path.join(runPath,'summary.json'),null).catch(()=>null):null;
        update(id,{status:previous.status==='cancelling'?'cancelled':code===0?'complete':'failed',exit_code:code ?? 1,ended_at:Date.now(),output,
          result:result?JSON.stringify(result):null,error:code===0?null:redact(workerFailure(code,result,summary,signal))});
        if(kind==='fixtures')controlDb().prepare('UPDATE fixture_sets SET status=? WHERE id=?').run(code===0?'ready':'failed',fixtureSetId);
        recordActivity(id,'finished',`${kind} job ${getJob(id).status}`);
        running.delete(id);resolve();
      });
    });
    const cancel=()=>{
      child.kill('SIGTERM');
      forceTimer=setTimeout(async()=>{
        const k6=await readJson(path.join(jobDir,'k6-pid.json'),null).catch(()=>null);
        if(k6?.pid)try {process.kill(-k6.pid,'SIGKILL');}catch{}
        child.kill('SIGKILL');
      },60000);
    };
    running.set(id,{cancel,done,controlToken,runPath});
    try {await atomicJson(path.join(jobDir,'worker-pid.json'),{pid:child.pid});}
    catch(e){cancel();await done;throw e;}
    return getJob(id);
  } catch(e) {update(id,{status:'failed',error:redact(e.message),ended_at:Date.now()});if(kind==='fixtures'&&fixtureSetId)controlDb().prepare("UPDATE fixture_sets SET status='failed' WHERE id=?").run(fixtureSetId);throw e;}
}

export async function startFaultTest(input) {
  const id=reserve('fault-test','local');
  update(id,{status:'running'});
  recordActivity(id,'fault_test','Standalone outage test · automatic recovery');
  // Start preflight synchronously so the target is reserved before returning.
  const done=testFault(getJob(id),input,recordActivity).then(result=>{
    const cancelled=getJob(id).status==='cancelling';
    update(id,{status:result?.error?'failed':cancelled?'cancelled':'complete',error:result?.error||null,result:JSON.stringify(result||{}),ended_at:Date.now(),exit_code:result?.error?1:0});
  }).catch(e=>update(id,{status:'failed',error:redact(e.message),ended_at:Date.now(),exit_code:1})).finally(()=>running.delete(id));
  running.set(id,{done,cancel:()=>{restoreFaults(recordActivity).catch(()=>{});}});
  return getJob(id);
}

export async function startReconciliation(id,allowIncomplete=false) {
  const directory=runDirectory(id),jobId=reserve('reconcile',directory.mode,directory.path);
  update(jobId,{status:'running'});
  let worker;
  const done=queryWorker('reconcile',{id,database:localSettings().database,allowIncomplete},
    progress=>update(jobId,{progress:JSON.stringify(progress)}),value=>worker=value)
    .then(result=>update(jobId,{status:'complete',result:JSON.stringify(result),ended_at:Date.now(),exit_code:result.mismatch||result.missing||result.transport?.missing||result.transport?.unresolved?2:0}))
    .catch(e=>{if(getJob(jobId).status!=='cancelled')update(jobId,{status:'failed',error:redact(e.message),ended_at:Date.now(),exit_code:1});})
    .finally(()=>running.delete(jobId));
  running.set(jobId,{done,cancel:()=>{worker?.terminate();update(jobId,{status:'cancelled',ended_at:Date.now()});}});
  return getJob(jobId);
}
export function cancelJob(id,force=false) {
  const job=getJob(id);
  if(!force&&job.command==='load'&&job.loadState==='pausing')throw new HttpError(409,'Wait for active flows to drain');
  if(!['starting','running','pausing','paused','cancelling'].includes(job.status))return job;
  if(job.status!=='cancelling'){update(id,{status:'cancelling'});running.get(id)?.cancel();}
  return getJob(id);
}
export async function shutdownJobs() {
  manager.shuttingDown=true;
  const values=[...running.entries()];for(const [id] of values)cancelJob(id,true);
  await Promise.all(values.map(([,v])=>v.done));
  await restoreFaults(recordActivity);
  await manager.recovery;
}

export function jobEvents(id,request) {
  getJob(id);
  const encoder=new TextEncoder();let timer;
  const stream=new ReadableStream({
    start(controller) {
      let revision=0;
      const finish=()=>{clearInterval(timer);try{controller.close();}catch{}};
      const emit=()=>{
        try {
          const job=getJob(id);controller.enqueue(encoder.encode(`id: ${++revision}\nevent: progress\ndata: ${JSON.stringify(job)}\n\n`));
          if(!['starting','running','pausing','paused','cancelling'].includes(job.status))finish();
        }catch{finish();}
      };
      timer=setInterval(emit,1000);emit();request.signal.addEventListener('abort',finish,{once:true});
    },cancel(){clearInterval(timer);}
  });
  return new Response(stream,{headers:{'content-type':'text/event-stream','cache-control':'no-cache, no-transform','x-accel-buffering':'no'}});
}
