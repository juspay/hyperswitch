import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile} from 'node:fs/promises';
import os from 'node:os';import path from 'node:path';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {Readable} from 'node:stream';
import {WorkerExecutor} from '../../worker/executor.mjs';
const temporary=await mkdtemp(path.join(os.tmpdir(),'worker-upload-http-'));
process.env.LOADTEST_DATA_DIR=temporary;
process.env.LOADTEST_CONSOLE_ROOT=path.resolve('.');process.env.LOADTEST_ROOT=path.resolve('..');
const state=await import('../server/state.mjs');state.initializeConfig();
const {passwordHash}=await import('../server/auth.mjs');
test('real worker streams recordings above 512KB through the production HTTP adapter',async()=>{
 const port=42000+Math.floor(Math.random()*1000),origin=`http://127.0.0.1:${port}`;
 const env={...process.env,HOST:'127.0.0.1',PORT:String(port),ORIGIN:origin,GRAFANA_PROXY_PORT:String(port+1000),GRAFANA_EMBED_ORIGIN:`http://127.0.0.1:${port+1000}`,ADMIN_PASSWORD_HASH:await passwordHash('fake-test-password'),LOADTEST_WORKER_ENROLLMENT_TOKEN:'fake-enrollment'};
 delete env.BODY_SIZE_LIMIT;
 const child=spawn(process.execPath,['start.mjs'],{env,stdio:['ignore','pipe','pipe']});let output='';child.stdout.on('data',d=>output+=d);child.stderr.on('data',d=>output+=d);
 try{
  let ready=false;for(let n=0;n<150;n++){try{ready=(await fetch(origin+'/health')).ok;}catch{}if(ready)break;await new Promise(r=>setTimeout(r,100));}assert.ok(ready,output);
  const enrollment=await fetch(origin+'/api/worker/v1/enroll',{method:'POST',headers:{authorization:'Bearer fake-enrollment','content-type':'application/json'},body:JSON.stringify({name:'http-worker',capacityTps:1,capacityVus:10})});assert.equal(enrollment.status,201);
  const identity=await enrollment.json();state.controlDb().prepare("INSERT INTO run_shards(id,run_id,worker_id,status,offered_tps,segment_start,segment_end,updated_at) VALUES('probe','probe',?,'uploading',1,0,1,?)").run(identity.id,Date.now());
  const outputDir=path.join(temporary,'output');await mkdir(outputDir);const artifact=Buffer.alloc(600000,65);await writeFile(path.join(outputDir,'results.sqlite'),artifact);await writeFile(path.join(outputDir,'summary.json'),'{}');
  const executor=new WorkerExecutor({server:origin,identity,request:()=>{},dataDir:temporary});executor.runs.set('probe',{output:outputDir});
  await executor.upload('probe',{authorization:`Bearer ${identity.token}`});
  assert.deepEqual(await readFile(path.join(temporary,'worker-artifacts','probe',identity.id,'results.sqlite')),artifact);
  const denied=await fetch(`${origin}/api/worker/v1/workers/${identity.id}/runs/probe/artifacts/summary.json`,{method:'PUT',headers:{authorization:'Bearer wrong','content-type':'application/json'},body:'{}'});assert.equal(denied.status,401);
  const oversized=await fetch(origin+'/login',{method:'POST',headers:{origin,'content-type':'application/x-www-form-urlencoded'},body:Readable.from([Buffer.alloc(700000),Buffer.alloc(700000)]),duplex:'half'});assert.equal(oversized.status,413);
 }finally{
  if(child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await once(child,'exit');}
  const daemon=JSON.parse(await readFile(path.join(temporary,'worker-service.json'),'utf8').catch(()=>'null'));if(daemon)try{process.kill(daemon.pid,'SIGTERM');}catch{}
 }
});
