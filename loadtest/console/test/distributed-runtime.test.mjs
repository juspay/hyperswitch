import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,readFile,writeFile} from 'node:fs/promises';
import {DatabaseSync} from 'node:sqlite';
import {createHash} from 'node:crypto';
import os from 'node:os';
import path from 'node:path';

const root=await mkdtemp(path.join(os.tmpdir(),'distributed-runtime-'));process.env.LOADTEST_DATA_DIR=root;process.env.LOADTEST_CONSOLE_ROOT=path.resolve('.');process.env.LOADTEST_ROOT=path.resolve('..');
const seed=JSON.parse(await readFile('../loadtest.config.example.json','utf8'));seed.output=path.join(root,'artifacts');process.env.LOADTEST_CONFIG_FILE=path.join(root,'seed.json');await writeFile(process.env.LOADTEST_CONFIG_FILE,JSON.stringify(seed));
const state=await import('../server/state.mjs');state.initializeConfig();
const registry=await import('../server/worker-registry.mjs'),{launchDistributedRun}=await import('../server/distributed-runner.mjs'),{readWorkerBundle,storeWorkerArtifact}=await import('../server/worker-artifacts.mjs');

const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
test('control plane prepares synchronized workers, receives artifacts and merges a run',async()=>{
  const fixturePath=path.join(root,'fixtures'),runPath=path.join(root,'run');await mkdir(fixturePath);await mkdir(runPath);
  const merchants=[{merchant_id:'m1'},{merchant_id:'m2'}],merchantText=JSON.stringify(merchants);await writeFile(path.join(fixturePath,'merchants.json'),merchantText);await writeFile(path.join(fixturePath,'manifest.json'),JSON.stringify({schema_version:1,state:'ready',router:'https://router.example',merchants_sha256:createHash('sha256').update(merchantText).digest('hex')}));
  const identities=[0,1].map(index=>registry.enrollWorker({name:`worker-${index}`,capacityTps:5,capacityVus:50},'secret','secret'));
  const config={mode:'single',services:{router:'https://router.example'},load:{total_rps:10,duration_seconds:1,max_vus_multiplier:5},scenarios:[{name:'flow',scenario:'guest',merchant_path:'non_modular',weight:100}]};
  let finished;const runner=await launchDistributedRun({runId:'run-test',runPath,config,fixturePath,onDone:(error,result)=>finished={error,result}});
  assert.equal(runner.plan.length,2);
  for(const identity of identities){const command=registry.pendingWorkerCommands(identity.id,identity.token)[0],bundle=await readWorkerBundle(command.payload.bundleId,identity.id);assert.equal(JSON.parse(bundle.body).merchants.length,1);registry.acknowledgeWorkerCommand(identity.id,identity.token,command.id);registry.heartbeatWorker(identity.id,identity.token,{status:'ready',runId:'run-test',runStatus:'ready'});}
  await delay(1200);
  for(const identity of identities){const command=registry.pendingWorkerCommands(identity.id,identity.token)[0];assert.equal(command.kind,'start');registry.acknowledgeWorkerCommand(identity.id,identity.token,command.id);}
  for(const action of ['gate','release','recover']){
    const payload=action==='release'?{until_active_seconds:20}:undefined,controlled=runner.control(action,payload);
    for(const identity of identities){const command=registry.pendingWorkerCommands(identity.id,identity.token)[0];assert.equal(command.kind,action);assert.ok(command.payload.effectiveAt<=Date.now());if(action==='release')assert.equal(command.payload.until_active_seconds,20);registry.acknowledgeWorkerCommand(identity.id,identity.token,command.id);}
    await controlled;
  }
  const schema=await readFile('../k6/scenario-mix/recorder/schema.sql','utf8');
  for(const [index,identity] of identities.entries()){
    const file=path.join(root,`shard-${index}.sqlite`),db=new DatabaseSync(file);db.exec(schema);db.prepare("INSERT INTO runs(run_id,recording_state,started_at,ended_at,enqueued,persisted,metadata) VALUES(?,'complete',1,2,1,1,'{}')").run(`source-${index}`);db.prepare("INSERT INTO request_events(event_id,run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,latency_ms) VALUES(1,?,1,?,'payment_confirm','measured',1,'POST','/',200,?)").run(`source-${index}`,`source-${index}:flow:1`,10+index);db.close();
    for(const [name,data] of [['results.sqlite',await readFile(file)],['summary.json',Buffer.from(JSON.stringify({metrics:{iterations:{type:'counter',values:{count:1}}}}))]])await storeWorkerArtifact({workerId:identity.id,runId:'run-test',name,body:new Blob([data]).stream(),contentLength:data.length});
    registry.heartbeatWorker(identity.id,identity.token,{status:'idle',runId:'run-test',runStatus:'complete'});
  }
  await Promise.race([runner.done,delay(5000).then(()=>{throw new Error('Distributed run did not finish');})]);assert.equal(finished.error,null);assert.equal(finished.result.merged.workers,2);assert.ok((await readFile(path.join(runPath,'results.sqlite'))).length>0);
});

test('a failed worker command fails its shard and releases the coordinator',async()=>{
  const fixturePath=path.join(root,'failure-fixtures'),runPath=path.join(root,'failure-run');await mkdir(fixturePath);await mkdir(runPath);
  const merchants=[{merchant_id:'m1'}],merchantText=JSON.stringify(merchants);await writeFile(path.join(fixturePath,'merchants.json'),merchantText);await writeFile(path.join(fixturePath,'manifest.json'),JSON.stringify({schema_version:1,state:'ready',merchants_sha256:createHash('sha256').update(merchantText).digest('hex')}));
  state.controlDb().prepare('UPDATE load_workers SET last_seen=0 WHERE id<>?').run('local');
  const worker=registry.enrollWorker({name:'worker-command-failure',capacityTps:5,capacityVus:50},'secret','secret');
  let finished;const runner=await launchDistributedRun({runId:'run-command-failure',runPath,config:{mode:'single',load:{total_rps:1,duration_seconds:1,max_vus_multiplier:5},scenarios:[]},fixturePath,onDone:(error,result)=>finished={error,result}});
  assert.equal(runner.plan[0].workerId,worker.id);
  const command=registry.pendingWorkerCommands(worker.id,worker.token)[0];registry.acknowledgeWorkerCommand(worker.id,worker.token,command.id,{ok:false,result:{error:'fixture preparation failed'}});
  await Promise.race([runner.done,delay(3000).then(()=>{throw new Error('Failed distributed run did not settle');})]);assert.match(finished.error.message,/fixture preparation failed/);
  assert.equal(state.controlDb().prepare('SELECT status FROM run_shards WHERE run_id=?').get('run-command-failure').status,'failed');
});

test('a lost worker heartbeat fails the run instead of waiting forever',async()=>{
  const fixturePath=path.join(root,'offline-fixtures'),runPath=path.join(root,'offline-run');await mkdir(fixturePath);await mkdir(runPath);
  const merchants=[{merchant_id:'m1'}],merchantText=JSON.stringify(merchants);await writeFile(path.join(fixturePath,'merchants.json'),merchantText);await writeFile(path.join(fixturePath,'manifest.json'),JSON.stringify({schema_version:1,state:'ready',merchants_sha256:createHash('sha256').update(merchantText).digest('hex')}));
  state.controlDb().prepare('UPDATE load_workers SET last_seen=0 WHERE id<>?').run('local');
  const worker=registry.enrollWorker({name:'worker-heartbeat-loss',capacityTps:5,capacityVus:50},'secret','secret');let finished;
  const runner=await launchDistributedRun({runId:'run-heartbeat-loss',runPath,config:{mode:'single',load:{total_rps:1,duration_seconds:1,max_vus_multiplier:5},scenarios:[]},fixturePath,onDone:error=>finished=error});
  state.controlDb().prepare('UPDATE load_workers SET last_seen=0 WHERE id=?').run(worker.id);
  await Promise.race([runner.done,delay(3000).then(()=>{throw new Error('Offline distributed run did not settle');})]);assert.match(finished.message,/heartbeat timed out/);
});
