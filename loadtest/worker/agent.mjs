#!/usr/bin/env node
import os from 'node:os';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import path from 'node:path';
import {WorkerExecutor} from './executor.mjs';

const server=String(process.env.LOADTEST_CONTROL_PLANE_URL||'').replace(/\/$/,'');
const enrollmentToken=process.env.LOADTEST_WORKER_ENROLLMENT_TOKEN||'';
const stateDir=path.resolve(process.env.LOADTEST_WORKER_DATA_DIR||'.worker-data'),stateFile=path.join(stateDir,'identity.json');
if(!server)throw new Error('LOADTEST_CONTROL_PLANE_URL is required');
function networkError(error){
  const cause=error?.cause,detail=[cause?.code,cause?.message].filter(Boolean).join(' · ');
  return detail?`${error.message} (${detail})`:error.message;
}
const request=async(url,options={})=>{let response;const {raw=false,...fetchOptions}=options;try{response=await fetch(server+url,{...fetchOptions,headers:{'content-type':'application/json',...(fetchOptions.headers||{})},signal:AbortSignal.timeout(15000)});}catch(error){throw new Error(`Cannot reach control plane at ${server}: ${networkError(error)}`);}const text=await response.text(),body=raw?text:(text?JSON.parse(text):{});if(!response.ok)throw new Error(body.error||`Control plane returned ${response.status}`);return body;};
let identity;try{identity=JSON.parse(await readFile(stateFile,'utf8'));}catch(e){if(e.code!=='ENOENT')throw e;}
if(!identity){
  for(;;)try{identity=await request('/api/worker/v1/enroll',{method:'POST',headers:{authorization:`Bearer ${enrollmentToken}`},body:JSON.stringify({name:process.env.LOADTEST_WORKER_NAME||os.hostname(),capacityTps:Number(process.env.LOADTEST_WORKER_TPS||250),capacityVus:Number(process.env.LOADTEST_WORKER_VUS||2500),version:'1',labels:{platform:process.platform,arch:process.arch}})});break;}catch(e){console.error(`Enrollment pending: ${e.message}`);await new Promise(resolve=>setTimeout(resolve,3000));}
  await mkdir(stateDir,{recursive:true,mode:0o700});await writeFile(stateFile,JSON.stringify(identity,null,2)+'\n',{mode:0o600});
}
let revision=Number(identity.revision||0),stopping=false,pollMs=Number(process.env.LOADTEST_WORKER_POLL_MS||1000);if(!Number.isInteger(pollMs)||pollMs<250||pollMs>10000)throw new Error('LOADTEST_WORKER_POLL_MS must be 250–10000 milliseconds');
const headers={authorization:`Bearer ${identity.token}`};
const executor=new WorkerExecutor({server,identity,request:(url,options={})=>request(url,{...options,headers:{...headers,...(options.headers||{})}}),dataDir:stateDir});
await executor.recover();
async function cycle(){
  await request(`/api/worker/v1/workers/${identity.id}/heartbeat`,{method:'POST',headers,body:JSON.stringify({...await executor.state(),version:'1'})});
  const commands=await request(`/api/worker/v1/workers/${identity.id}/commands?after=${revision}`,{headers});
  for(const command of commands){
    let result,error;try{
      if(command.kind==='prepare')result=await executor.prepare(command);
      else if(command.kind==='start'){result=await executor.start(command);const run=executor.runs.get(command.runId);run.done.then(async()=>{if(run.status==='recorded'){run.status='uploading';await executor.persist(command.runId,run);await executor.upload(command.runId,headers);run.status='complete';await executor.persist(command.runId,run);}}).catch(async e=>{run.status='failed';run.log=(run.log||'')+'\n'+e.message;await executor.persist(command.runId,run);});}
      else if(['pause','resume','gate','release','recover','stop'].includes(command.kind))result=await executor.control(command);
      else if(command.kind==='upload')result=await executor.upload(command.runId,headers);
      else throw new Error('Unsupported command');
    }catch(e){error=e.message;}
    await request(`/api/worker/v1/workers/${identity.id}/commands/${command.id}/ack`,{method:'POST',headers,body:JSON.stringify({ok:!error,result:error?{error}:result})});
    revision=Math.max(revision,command.revision);
    identity.revision=revision;await writeFile(stateFile,JSON.stringify(identity,null,2)+'\n',{mode:0o600});
  }
}
process.once('SIGINT',()=>stopping=true);process.once('SIGTERM',()=>stopping=true);
while(!stopping){try{await cycle();}catch(e){console.error(e.message);}await new Promise(resolve=>setTimeout(resolve,pollMs));}
