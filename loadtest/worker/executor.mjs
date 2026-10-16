import {createHash,randomUUID} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createReadStream} from 'node:fs';
import {mkdir,readFile,writeFile,readdir} from 'node:fs/promises';
import path from 'node:path';

const digest=value=>createHash('sha256').update(value).digest('hex');
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
export function shardLoadConfig(input,{offeredTps,totalTps}){
  const config=structuredClone(input),fraction=Number(offeredTps)/Number(totalTps);
  if(!(fraction>0&&fraction<=1))throw new Error('Worker shard fraction is invalid');
  if(Array.isArray(config.load?.stages)){
    config.load.stages=config.load.stages.map(stage=>({...stage,target_rps:Math.max(.001,Number(stage.target_rps)*fraction)}));
    config.load.start_rps=Math.max(0,Number(config.load.start_rps||0)*fraction);
  }else if(config.load?.phases){
    config.load.phases={...config.load.phases,starting_rps:Number(config.load.phases.starting_rps)*fraction,target_rps:Number(config.load.phases.target_rps)*fraction};
  }else config.load.total_rps=Number(offeredTps);
  config.load.pre_allocated_vus_multiplier=Number(config.load.pre_allocated_vus_multiplier||3);config.load.max_vus_multiplier=Number(config.load.max_vus_multiplier||10);
  return config;
}
export class WorkerExecutor{
  constructor({server,identity,request,dataDir,loadtestRoot=path.resolve(process.env.LOADTEST_ROOT||path.join(import.meta.dirname,'..'))}){this.server=server;this.identity=identity;this.request=request;this.dataDir=dataDir;this.loadtestRoot=loadtestRoot;this.runs=new Map();}
  async persist(runId,run){await writeFile(path.join(run.directory,'worker-state.json'),JSON.stringify({runId,status:run.status,exitCode:run.exitCode,bundleId:run.bundleId})+'\n',{mode:0o600});}
  async recover(){for(const entry of await readdir(path.join(this.dataDir,'runs'),{withFileTypes:true}).catch(e=>e.code==='ENOENT'?[]:Promise.reject(e))){if(!entry.isDirectory())continue;const directory=path.join(this.dataDir,'runs',entry.name);try{const state=JSON.parse(await readFile(path.join(directory,'worker-state.json'),'utf8')),status=['ready','complete'].includes(state.status)?state.status:'failed',run={directory,fixtures:path.join(directory,'fixtures'),output:path.join(directory,'output'),status,exitCode:state.exitCode,bundleId:state.bundleId,child:null,log:status==='failed'?'Worker agent restarted during execution':''};this.runs.set(state.runId,run);await this.persist(state.runId,run);}catch{}}
  }
  async prepare(command){
    const existing=this.runs.get(command.runId);if(existing&&!['failed'].includes(existing.status)){if(existing.bundleId&&existing.bundleId!==command.payload.bundleId)throw new Error('Run was already prepared from a different bundle');return {runId:command.runId,status:existing.status};}
    const {bundleId,sha256}=command.payload,body=await this.request(`/api/worker/v1/workers/${this.identity.id}/bundles/${bundleId}`,{raw:true});
    if(digest(body)!==sha256)throw new Error('Worker bundle checksum mismatch');const bundle=JSON.parse(body),directory=path.join(this.dataDir,'runs',bundle.runId),fixtures=path.join(directory,'fixtures'),output=path.join(directory,'output');
    await mkdir(fixtures,{recursive:true,mode:0o700});await mkdir(output,{recursive:true,mode:0o700});
    const config=shardLoadConfig(bundle.config,command.payload);
    await Promise.all([writeFile(path.join(directory,'config.json'),JSON.stringify(config,null,2),{mode:0o600}),writeFile(path.join(fixtures,'manifest.json'),JSON.stringify(bundle.manifest),{mode:0o600}),writeFile(path.join(fixtures,'merchants.json'),JSON.stringify(bundle.merchants),{mode:0o600})]);
    const run={directory,fixtures,output,status:'ready',child:null,bundleId};this.runs.set(bundle.runId,run);await this.persist(bundle.runId,run);return {runId:bundle.runId,status:'ready'};
  }
  async start(command){
    const run=this.runs.get(command.runId);if(!run)throw new Error('Run is not prepared');if(run.status!=='ready'){if(['running','pausing','paused','draining','recorded','uploading','complete'].includes(run.status))return {runId:command.runId,status:run.status};throw new Error('Run is not prepared');}const delay=Number(command.payload.startAt||0)-Date.now();if(delay>0)await wait(delay);
    const binary=process.env.K6_BINARY||path.join(this.loadtestRoot,'k6/scenario-mix/bin/k6-sqlite'),entry=path.join(this.loadtestRoot,'k6/scenario-mix/run.mjs');
    run.status='running';run.controlToken=randomUUID();await this.persist(command.runId,run);run.child=spawn(process.execPath,[entry,'load','--config',path.join(run.directory,'config.json'),'--fixtures',run.fixtures,'--output',run.output,'--k6',binary],{stdio:['ignore','pipe','pipe'],env:{...process.env,LOADTEST_CONSOLE_JOB:'1',SQLITE_RECORDER_CONTROL_TOKEN:run.controlToken,SQLITE_RECORDER_CONTROL_FILE:path.join(run.output,'control.json'),SQLITE_RECORDER_PROGRESS_FILE:path.join(run.output,'progress.json')}});
    run.log='';for(const stream of [run.child.stdout,run.child.stderr])stream.on('data',chunk=>run.log=(run.log+chunk).slice(-100000));
    run.done=new Promise(resolve=>run.child.once('close',async code=>{run.exitCode=code??1;run.status=[0,99].includes(run.exitCode)?'recorded':'failed';await this.persist(command.runId,run);resolve(run);}));return {runId:command.runId,status:'running'};
  }
  async control(command){
    const run=this.runs.get(command.runId);if(!run)throw new Error('Run does not exist');
    const delay=Number(command.payload?.effectiveAt||0)-Date.now();if(delay>0)await wait(delay);
    if(command.kind==='stop'){if(!['complete','failed','recorded','uploading'].includes(run.status)){run.child?.kill('SIGTERM');run.status='draining';}return {status:run.status};}
    if((command.kind==='pause'&&run.status==='paused')||(command.kind==='resume'&&run.status==='running'))return {state:run.status};
    const control=JSON.parse(await readFile(path.join(run.output,'control.json'),'utf8')),response=await fetch(control.endpoint,{method:'POST',headers:{authorization:`Bearer ${run.controlToken}`,'content-type':'application/json'},body:JSON.stringify({action:command.kind,...command.payload}),signal:AbortSignal.timeout(5000)});
    if(!response.ok)throw new Error(await response.text());const state=await response.json();run.status=state.state==='paused'?'paused':state.state==='pausing'?'pausing':'running';return state;
  }
  async state(){const active=[...this.runs.entries()].find(([,run])=>!['complete','failed'].includes(run.status));if(active){let progress={};try{progress=JSON.parse(await readFile(path.join(active[1].output,'progress.json'),'utf8'));}catch{}return {runId:active[0],runStatus:active[1].status,status:['recorded'].includes(active[1].status)?'uploading':active[1].status,progress:{...progress,exitCode:active[1].exitCode}};}const finished=[...this.runs.entries()].at(-1);return finished?{runId:finished[0],runStatus:finished[1].status,status:'idle',progress:{exitCode:finished[1].exitCode},error:finished[1].status==='failed'?finished[1].log:null}:{status:'idle'};}
  async upload(runId,headers){
    const run=this.runs.get(runId);if(!run)throw new Error('Run does not exist');for(const name of ['results.sqlite','summary.json','progress.json']){const file=path.join(run.output,name);try{await readFile(file);}catch{continue;}const response=await fetch(`${this.server}/api/worker/v1/workers/${this.identity.id}/runs/${runId}/artifacts/${name}`,{method:'PUT',headers,body:createReadStream(file),duplex:'half'});if(!response.ok)throw new Error(`Artifact upload failed (${response.status})`);}return {uploaded:true};
  }
}
