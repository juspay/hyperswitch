// SSR is only a client of this worker service. Browser and web-server lifetimes
// have no ownership of generators, pause state, fault recovery or reconciliation.
import {openSync,closeSync} from 'node:fs';
import {spawn} from 'node:child_process';
import {createServer} from 'node:http';
import {mkdir,readFile,writeFile,rm} from 'node:fs/promises';
import {randomUUID,timingSafeEqual} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {dataRoot,consoleRoot,redact} from './state.mjs';
import {HttpError} from './errors.mjs';
const descriptor=path.join(dataRoot,'worker-service.json'),lock=path.join(dataRoot,'worker-service.lock');
const protocolVersion=2;
const alive=pid=>{try{process.kill(pid,0);return true;}catch{return false;}};
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let starting;
async function readService(){try{return JSON.parse(await readFile(descriptor,'utf8'));}catch{return null;}}
async function serviceRequest(service,operation,args={},timeout=15000){
  const response=await fetch(`${service.url}/rpc`,{method:'POST',headers:{authorization:`Bearer ${service.token}`,'content-type':'application/json'},body:JSON.stringify({operation,args}),signal:AbortSignal.timeout(timeout)});
  const result=await response.json();if(!response.ok)throw new HttpError(response.status,result.error);return result;
}
async function replaceOutdatedService(service,actual){
  let active=null;try{active=await serviceRequest(service,'active',{},2000);}catch(error){if(error.status!==404)throw error;}
  if(active)throw new HttpError(409,`Worker service protocol ${actual||'legacy'} is outdated. Stop the active ${active.command} job, then restart the console.`);
  try{process.kill(service.pid,'SIGTERM');}catch(error){if(error.code!=='ESRCH')throw error;}
  for(let i=0;i<100&&alive(service.pid);i++)await wait(100);
  if(alive(service.pid))throw new HttpError(503,'Outdated worker service did not stop; run npm run worker -- stop and retry');
  await rm(descriptor,{force:true});
}
export async function initializeSupervisor() {
  if(starting)return starting;
  starting=(async()=>{
    await mkdir(dataRoot,{recursive:true,mode:0o700});
    let launched=false;
    for(let i=0;i<150;i++){
      const service=await readService();
      if(service&&alive(service.pid)){
        try{
          const response=await fetch(`${service.url}/health`,{headers:{authorization:`Bearer ${service.token}`},signal:AbortSignal.timeout(500)});
          if(response.ok){const health=await response.json();if(health.protocol===protocolVersion)return service;await replaceOutdatedService(service,health.protocol);}
        }catch(error){if(error instanceof HttpError)throw error;}
      }
      if(!launched){
        const log=openSync(path.join(dataRoot,'worker.log'),'a',0o600);
        try{const child=spawn(process.execPath,[path.join(consoleRoot,'server','supervisor.mjs'),'serve'],{cwd:consoleRoot,detached:true,stdio:['ignore',log,log],env:process.env});child.on('error',()=>{});child.unref();launched=true;}finally{closeSync(log);}
      }
      await wait(100);
    }
    throw new HttpError(503,'Worker service unavailable; check the worker log and restart it');
  })().finally(()=>{starting=null;});
  return starting;
}
export async function workerCommand(operation,args={}) {
  const service=await initializeSupervisor();
  return serviceRequest(service,operation,args);
}
export function jobEvents(id,request){
  let timer,busy=false,closed=false;
  const encoder=new TextEncoder();
  const stream=new ReadableStream({start(controller){
    const close=()=>{if(closed)return;closed=true;clearInterval(timer);try{controller.close();}catch{}};
    const emit=async()=>{if(busy||closed)return;busy=true;try{const job=await workerCommand('job',{id});if(!closed){controller.enqueue(encoder.encode(`event: progress\ndata: ${JSON.stringify(job)}\n\n`));if(!['starting','running','pausing','paused','cancelling'].includes(job.status))close();}}catch(e){if(!closed)controller.enqueue(encoder.encode(`event: unavailable\ndata: ${JSON.stringify({error:e.message})}\n\n`));}finally{busy=false;}};
    timer=setInterval(emit,1000);request.signal.addEventListener('abort',close,{once:true});emit();
  },cancel(){closed=true;clearInterval(timer);}});
  return new Response(stream,{headers:{'content-type':'text/event-stream','cache-control':'no-cache, no-transform','x-accel-buffering':'no'}});
}
async function serve(){
  await mkdir(dataRoot,{recursive:true,mode:0o700});
  try{await mkdir(lock,{mode:0o700});}catch(e){
    if(e.code!=='EEXIST')throw e;
    const owner=JSON.parse(await readFile(path.join(lock,'owner.json'),'utf8').catch(()=>'null'));
    // Never steal a live worker's lock. Missing owner can mean another process
    // has just acquired it; retry on a subsequent startup rather than racing.
    if(owner&&alive(owner.pid))return;
    if(!owner){const {stat}=await import('node:fs/promises');if(Date.now()-(await stat(lock)).mtimeMs<15000)return;}
    await rm(lock,{recursive:true,force:true});return serve();
  }
  await writeFile(path.join(lock,'owner.json'),JSON.stringify({pid:process.pid}),{mode:0o600});
  const jobs=await import('./jobs.mjs'),faults=await import('./faults.mjs'),registry=await import('./worker-registry.mjs');jobs.initializeJobs();registry.registerLocalWorker();
  const localHeartbeat=setInterval(()=>registry.registerLocalWorker(),5000);localHeartbeat.unref();
  const token=randomUUID();let shuttingDown=false;
  const server=createServer(async(req,res)=>{
    const provided=Buffer.from(req.headers.authorization || ''),expected=Buffer.from(`Bearer ${token}`);
    if(provided.length!==expected.length||!timingSafeEqual(provided,expected)){res.writeHead(401);res.end();return;}
    res.setHeader('content-type','application/json');
    try{
      if(req.url==='/health'&&req.method==='GET'){res.end(JSON.stringify({ready:true,protocol:protocolVersion}));return;}
      if(req.url!=='/rpc'||req.method!=='POST')throw new HttpError(404,'Not found');
      if(shuttingDown)throw new HttpError(503,'Worker is shutting down');
      let text='';for await(const chunk of req){text+=chunk;if(text.length>1024*1024)throw new HttpError(413,'Request too large');}
      const {operation,args:a={}}=JSON.parse(text);let result;
      switch(operation){
        case 'save-scenario':result=(await import('./profiles.mjs')).saveScenario(a);break;
        case 'active':result=jobs.activeJob(a.kind);break;
        case 'job':result=jobs.getJob(a.id);break;
        case 'latest-reconciliation':result=jobs.latestReconciliation(a.path);break;
        case 'start':result=await jobs.startGenerator(a.kind,a.mode,a);break;
        case 'cancel':result=jobs.cancelJob(a.id);break;
        case 'control':result=await jobs.controlLoad(a.id,a.action);break;
        case 'reconcile':result=await jobs.startReconciliation(a.id,a.allowIncomplete);break;
        case 'fault-targets':result=faults.localFaultTargets();break;
        case 'save-fault-targets':result=await faults.saveLocalFaultTargets(a.targets);break;
        case 'check-fault-target':result=await faults.checkLocalFaultTarget(a);break;
        case 'test-fault':result=await jobs.startFaultTest(a);break;
        case 'faults':result=faults.faultStatus();break;
        case 'fault':result=await faults.fireFault(jobs.getJob(a.jobId),a,jobs.recordActivity,()=>jobs.getJob(a.jobId));break;
        case 'restore':result=faults.requestRestore(jobs.recordActivity);break;
        default:throw new HttpError(404,'Unknown operation');
      }
      res.end(JSON.stringify(result));
    }catch(e){res.statusCode=e.status || 400;res.end(JSON.stringify({error:redact(e.message)}));}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',()=>resolve(undefined)));
  await writeFile(descriptor,JSON.stringify({pid:process.pid,url:`http://127.0.0.1:${/** @type {import('node:net').AddressInfo} */(server.address()).port}`,token,protocol:protocolVersion}),{mode:0o600});
  const stop=async()=>{if(shuttingDown)return;shuttingDown=true;clearInterval(localHeartbeat);await jobs.shutdownJobs();server.close();await rm(descriptor,{force:true});await rm(lock,{recursive:true,force:true});process.exit(0);};
  process.once('SIGTERM',stop);process.once('SIGINT',stop);
}
if(process.argv[1]===fileURLToPath(import.meta.url)&&process.argv[2]==='serve')serve().catch(async e=>{await writeFile(path.join(dataRoot,'worker-error.log'),redact(e.stack)+'\n',{mode:0o600}).catch(()=>{});process.exitCode=1;});
