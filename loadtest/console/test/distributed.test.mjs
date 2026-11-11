import test from 'node:test';
import assert from 'node:assert/strict';
import {planWorkerShards} from '../server/distributed.mjs';
import {mergeWorkerShards} from '../server/merge-shards.mjs';
import {workerLoadConfig} from '../server/worker-artifacts.mjs';
import {shardLoadConfig} from '../../worker/executor.mjs';
import {DatabaseSync} from 'node:sqlite';
import {mkdtemp,readFile,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

const now=Date.now();
test('distributed planner selects healthy capacity and creates exact contiguous shards',()=>{
  const workers=Array.from({length:5},(_,index)=>({id:`w${index}`,name:`worker-${index}`,status:'idle',lastSeen:index===4?now-20000:now,capacityTps:250,capacityVus:1000}));
  const plan=planWorkerShards(1000,3000,workers,{now});
  assert.equal(plan.length,4);assert.equal(plan.reduce((sum,item)=>sum+item.offeredTps,0),1000);
  assert.equal(plan[0].segmentStart,0);assert.equal(plan.at(-1).segmentEnd,1);
  plan.slice(1).forEach((item,index)=>assert.equal(item.segmentStart,plan[index].segmentEnd));
});

test('distributed planner rejects insufficient live workers',()=>{
  assert.throws(()=>planWorkerShards(1000,1000,[{id:'w1',status:'idle',lastSeen:now,capacityTps:250,capacityVus:1000}],{now}),/250\/1000 TPS/);
});

test('worker bundles contain load inputs but no fault or fixture-control credentials',()=>{
  const config={mode:'single',services:{router:'https://router'},load:{start_rps:1,stages:[{target_rps:4,duration_seconds:60,fault:{targetId:'postgres',action:'failover'}}]},scenarios:[{name:'guest'}],payment:{amount:100},thresholds:{failed:['rate<1']},fixtures:{admin_api_key:'secret',superposition:{token:'secret'}},faultPlan:[{target:{adapter:'gcp-alloydb',project:'secret-project'}}],database:{password:'secret'}};
  const worker=workerLoadConfig(config);assert.equal(worker.load.stages[0].target_rps,4);assert.equal(worker.load.stages[0].fault,undefined);assert.equal(worker.fixtures,undefined);assert.equal(worker.faultPlan,undefined);assert.equal(worker.database,undefined);assert.doesNotMatch(JSON.stringify(worker),/secret-project|admin_api_key|superposition/);
});

test('each worker scales the entire ramp while preserving its timing',()=>{
  const config=shardLoadConfig({load:{start_rps:100,stages:[{target_rps:200,duration_seconds:60},{target_rps:1000,duration_seconds:120}],pre_allocated_vus_multiplier:4,max_vus_multiplier:12}},{offeredTps:250,totalTps:1000});
  assert.equal(config.load.start_rps,25);assert.deepEqual(config.load.stages.map(stage=>stage.target_rps),[50,250]);assert.deepEqual(config.load.stages.map(stage=>stage.duration_seconds),[60,120]);
});

test('worker SQLite shards merge with unique events and exact latency percentiles',async()=>{
  const root=await mkdtemp(path.join(os.tmpdir(),'loadtest-shards-')),schema=await readFile('../k6/scenario-mix/recorder/schema.sql','utf8'),shards=[];
  for(let n=0;n<2;n++){
    const directory=path.join(root,`worker-${n}`),file=path.join(directory,'results.sqlite');await import('node:fs/promises').then(fs=>fs.mkdir(directory));const db=new DatabaseSync(file);db.exec(schema);
    db.prepare("INSERT INTO runs(run_id,recording_state,started_at,ended_at,enqueued,persisted,metadata) VALUES(?,'complete',?,?,2,2,'{}')").run(`source-${n}`,1000+n,2000+n);
    for(let e=0;e<2;e++){const id=e+1,payment=`pay-${n}-${e}`;db.prepare("INSERT INTO request_events(event_id,run_id,attempt_id,flow_id,operation,role,observed_at,method,url,payment_id,status,api_status_code,latency_ms) VALUES(?,?,?,?,?,'measured',?,'POST','/payments',?,'succeeded',200,?)").run(id,`source-${n}`,id,`source-${n}:flow:${id}`,'payment_confirm',1100+id,payment,10+n*20+e);db.prepare("INSERT INTO payments(run_id,payment_id,role,api_status_code,observed_at,response_event_id,status) VALUES(?,?,'measured',200,?,?, 'succeeded')").run(`source-${n}`,payment,1100+id,id);}
    db.prepare('UPDATE request_events SET server_latency_ms=? WHERE event_id=1').run(n===0?0:20);
    db.close();const summary=path.join(directory,'summary.json');await writeFile(summary,JSON.stringify({metrics:{iterations:{type:'counter',values:{count:2}},http_req_failed:{type:'rate',values:{passes:n+1,fails:9-n},thresholds:{'rate<0.01':{ok:n===0}}}}}));shards.push({workerId:`w${n}`,database:file,summary});
  }
  const runPath=path.join(root,'merged');await import('node:fs/promises').then(fs=>fs.mkdir(runPath));const result=await mergeWorkerShards({runPath,runId:'canonical',shards,config:{scenarios:[{name:'flow'}]}});assert.equal(result.enqueued,4);
  const merged=new DatabaseSync(result.database,{readOnly:true});assert.equal(merged.prepare('SELECT count(*) n FROM request_events').get().n,4);assert.equal(merged.prepare('SELECT count(*) n FROM payments').get().n,4);assert.equal(merged.prepare('SELECT run_id FROM runs').get().run_id,'canonical');merged.close();
  const summary=JSON.parse(await readFile(result.summary,'utf8'));assert.equal(summary.metrics.iterations.values.count,4);assert.equal(summary.metrics.payment_confirm_latency_ms.values['p(99)'],31);assert.equal(summary.metrics.payment_confirm_server_latency_ms.values['p(99)'],20);assert.equal(summary.metrics.payment_confirm_server_ms_flow.values['p(99)'],20);assert.equal(summary.metrics.http_req_failed.values.rate,.15);assert.equal(summary.metrics.http_req_failed.thresholds['rate<0.01'].ok,false);
});
