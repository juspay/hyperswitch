import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { controlDb, initializeConfig, readJsonSync, readJson, atomicJson, dataRoot, configPath, localSettings, consoleRoot, loadtestRoot, redact, runDirectory } from './state.mjs';
import { queryWorker } from './workers.mjs';
import { HttpError } from './errors.mjs';
import { appendTelemetry,telemetryPoint } from './telemetry.mjs';

// Share lifecycle state with adapter-node and Vite's server-module reloads.
const key=Symbol.for('hyperswitch.loadtest.jobs');
const manager=globalThis[key] ||= {running:new Map(),initialized:false,shuttingDown:false,recovering:false};
const running=manager.running;
export function initializeJobs() {
  if(manager.initialized)return;
  initializeConfig();const db=controlDb();
  const interrupted=db.prepare("SELECT id FROM jobs WHERE status IN ('starting','running','cancelling')").all();
  db.prepare("UPDATE jobs SET status='interrupted',ended_at=?,error='Server restarted before job finished' WHERE status IN ('starting','running','cancelling')").run(Date.now());
  manager.initialized=true;
  if(interrupted.length){
    manager.recovering=true;
    manager.recovery=Promise.all(interrupted.map(stopInterrupted)).finally(()=>{manager.recovering=false;});
  }
  process.once('SIGTERM',()=>{shutdownJobs().catch(console.error);});
  process.once('SIGINT',()=>{shutdownJobs().catch(console.error);});
}
async function stopInterrupted({id}) {
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
}
const parsed=value=>JSON.parse(value || '{}');
export function getJob(id) {
  const row=controlDb().prepare('SELECT * FROM jobs WHERE id=?').get(id);
  if(!row) throw new HttpError(404,'Job not found');
  const config=row.config_path?readJsonSync(row.config_path,{}):{};
  return {id:row.id,command:row.kind,mode:row.mode,status:row.status,startedAt:row.started_at,endedAt:row.ended_at,
    exitCode:row.exit_code,runId:row.kind==='load'&&row.run_path?`${row.mode}:${path.basename(row.run_path)}`:undefined,
    profile:config.load?{totalRps:config.load.total_rps,durationSeconds:config.load.duration_seconds,scenarios:config.scenarios}:null,
    output:row.output,progress:parsed(row.progress),result:row.result?parsed(row.result):null,error:row.error};
}
export function activeJob() {
  initializeJobs();const row=controlDb().prepare("SELECT id FROM jobs WHERE status IN ('starting','running','cancelling') ORDER BY started_at DESC LIMIT 1").get();
  return row?getJob(row.id):null;
}
function reserve(kind,mode,runPath=null) {
  initializeJobs();if(manager.shuttingDown) throw new Error('Server is shutting down');
  if(manager.recovering)throw new HttpError(409,'Previous workers are stopping after restart; retry shortly');
  const db=controlDb(),id=randomUUID();
  if(kind==='reconcile' && db.prepare("SELECT id FROM jobs WHERE kind='reconcile' AND status IN ('starting','running','cancelling')").get()) throw new HttpError(409,'Reconciliation is already running');
  try {db.prepare("INSERT INTO jobs(id,kind,mode,status,started_at,run_path) VALUES(?,?,?,'starting',?,?)").run(id,kind,mode,Date.now(),runPath);}
  catch(e) {if(e.code==='ERR_SQLITE_ERROR')throw new HttpError(409,'A fixture or load job is already running');throw e;}
  return id;
}
function update(id,values) {
  const columns=Object.keys(values);controlDb().prepare(`UPDATE jobs SET ${columns.map(k=>`${k}=?`).join(',')} WHERE id=?`).run(...Object.values(values),id);
}
export async function startGenerator(kind,mode) {
  if(!['fixtures','load'].includes(kind) || !['single','tenancy'].includes(mode)) throw new Error('Invalid job');
  const id=reserve(kind,mode);
  try {
    const jobDir=path.join(dataRoot,'jobs',id);await mkdir(jobDir,{recursive:true,mode:0o700});
    const config=structuredClone(readJsonSync(configPath));
    config.mode=mode;
    const runPath=kind==='load'?path.join(config.output,mode,'runs',`${new Date().toISOString().replace(/[:.]/g,'-')}-${mode}-${config.load.total_rps}tps-${id.slice(0,8)}`):null;
    const snapshot=path.join(jobDir,'config.json');await atomicJson(snapshot,config);
    const spec={id,kind,mode,configPath:snapshot,runPath,loadtestRoot,parentPid:process.pid};
    await atomicJson(path.join(jobDir,'job.json'),spec);update(id,{config_path:snapshot,run_path:runPath,status:'running'});
    let output='',lastWrite=0,lastElapsed=-1,lastPoint=null,forceTimer;
    const remember=async progress=>{
      if(runPath && Number.isFinite(progress.elapsed_seconds) && progress.elapsed_seconds>lastElapsed){
        lastElapsed=progress.elapsed_seconds;lastPoint=telemetryPoint(progress,lastPoint);await appendTelemetry(runPath,lastPoint);
      }
    };
    const child=spawn(process.execPath,[path.join(consoleRoot,'scripts','execute.mjs'),path.join(jobDir,'job.json')],{
      cwd:consoleRoot,stdio:['ignore','pipe','pipe'],env:{...process.env,LOADTEST_JOB_DIR:jobDir,SQLITE_RECORDER_PROGRESS_FILE:runPath?path.join(runPath,'progress.json'):''}
    });
    const log=(data)=>{output=(output+redact(data)).slice(-50000);};
    child.stdout.on('data',log);child.stderr.on('data',log);
    const tick=setInterval(async()=>{
      if(Date.now()-lastWrite<900)return;lastWrite=Date.now();
      try {const progress=runPath?await readJson(path.join(runPath,'progress.json'),{}):{};await remember(progress);progress.trend=lastPoint;update(id,{output,progress:JSON.stringify(progress)});}
      catch(e) {log(`\nProgress unavailable: ${e.message}`);}
    },1000);
    const done=new Promise(resolve=>{
      child.once('error',e=>{log(e.message);update(id,{status:'failed',error:redact(e.message),ended_at:Date.now(),output});});
      child.once('close',async(code,signal)=>{
        clearInterval(tick);clearTimeout(forceTimer);
        if(runPath)try{await remember(await readJson(path.join(runPath,'progress.json'),{}));}catch(e){log(`\nTrend unavailable: ${e.message}`);}
        const previous=getJob(id);
        let result=await readJson(path.join(jobDir,'result.json'),null).catch(()=>null);
        update(id,{status:previous.status==='cancelling'?'cancelled':code===0?'complete':'failed',exit_code:code ?? 1,ended_at:Date.now(),output,
          result:result?JSON.stringify(result):null,error:code===0?null:redact(result?.error || `Worker exited ${code ?? signal}`)});
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
    running.set(id,{cancel,done});
    try {await atomicJson(path.join(jobDir,'worker-pid.json'),{pid:child.pid});}
    catch(e){cancel();await done;throw e;}
    return getJob(id);
  } catch(e) {update(id,{status:'failed',error:redact(e.message),ended_at:Date.now()});throw e;}
}

export async function startReconciliation(id,allowIncomplete=false) {
  const directory=runDirectory(id),jobId=reserve('reconcile',directory.mode,directory.path);
  update(jobId,{status:'running'});
  let worker;
  const done=queryWorker('reconcile',{id,database:localSettings().database,allowIncomplete},
    progress=>update(jobId,{progress:JSON.stringify(progress)}),value=>worker=value)
    .then(result=>update(jobId,{status:'complete',result:JSON.stringify(result),ended_at:Date.now(),exit_code:result.mismatch||result.missing?2:0}))
    .catch(e=>{if(getJob(jobId).status!=='cancelled')update(jobId,{status:'failed',error:redact(e.message),ended_at:Date.now(),exit_code:1});})
    .finally(()=>running.delete(jobId));
  running.set(jobId,{done,cancel:()=>{worker?.terminate();update(jobId,{status:'cancelled',ended_at:Date.now()});}});
  return getJob(jobId);
}
export function cancelJob(id) {
  const job=getJob(id);
  if(!['starting','running','cancelling'].includes(job.status))return job;
  if(job.status!=='cancelling'){update(id,{status:'cancelling'});running.get(id)?.cancel();}
  return getJob(id);
}
export async function shutdownJobs() {
  manager.shuttingDown=true;
  const values=[...running.entries()];for(const [id] of values)cancelJob(id);
  await Promise.all(values.map(([,v])=>v.done));
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
          if(!['starting','running','cancelling'].includes(job.status))finish();
        }catch{finish();}
      };
      timer=setInterval(emit,1000);emit();request.signal.addEventListener('abort',finish,{once:true});
    },cancel(){clearInterval(timer);}
  });
  return new Response(stream,{headers:{'content-type':'text/event-stream','cache-control':'no-cache, no-transform','x-accel-buffering':'no'}});
}
