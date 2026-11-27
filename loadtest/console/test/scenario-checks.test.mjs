import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,rm} from 'node:fs/promises';
import {DatabaseSync} from 'node:sqlite';
import path from 'node:path';import os from 'node:os';
import {evaluateScenarioChecks,scenarioChecks,faultOccurrences} from '../server/scenario-checks.mjs';
import {defaultResultsLimits,validateResultsLimits} from '../src/results-limits.mjs';
import {executionSample,executionClock,stageIntervals} from '../server/execution-clock.mjs';

const recording={run_id:'run',started_at:1000,ended_at:31000,recording_state:'complete',persisted:100};
const config={load:{total_rps:10,duration_seconds:30},resultsLimits:{...defaultResultsLimits}};
const samples=Array.from({length:31},(_,i)=>({at:1000+i*1000,active:i}));
const rows=Array.from({length:300},(_,i)=>({observed_at:1050+i*100,operation:'payment_confirm',api_status_code:200,status:'succeeded',server_latency_ms:50,latency_ms:10}));
const flows=rows.map(r=>({at:r.observed_at,success:1}));
const evaluate=changes=>evaluateScenarioChecks({config,recording,samples,requests:rows,flows,jobStatus:'complete',...changes});
const row=(group,id)=>group.checks.find(r=>r.id===id);
test('generic steady verdict and drift use the combined settled period, not individual hold verdicts',()=>{
 const cfg={...config,resultsLimits:{...defaultResultsLimits,recoveryWindowSeconds:1},load:{start_rps:5,stages:[{target_rps:5,duration_seconds:20},{target_rps:5,duration_seconds:80}]}};
 const requests=Array.from({length:500},(_,i)=>({observed_at:1050+i*200,operation:'payment_confirm',api_status_code:200,status:'succeeded',server_latency_ms:i<25?50:60,latency_ms:10}));
 const result=evaluateScenarioChecks({config:cfg,recording:{...recording,ended_at:101000},samples:Array.from({length:101},(_,i)=>({at:1000+i*1000,active:i})),requests,flows:requests.map(r=>({at:r.observed_at,success:1})),jobStatus:'complete'});
 assert.equal(row(result.groups[0],'drift').outcome,'fail');
 assert.equal(row(result.steady,'drift').actual,0);
 assert.equal(result.verdict,'pass');
 assert.equal(row(result.steady,'tps').actual,5);
});
test('outage and recovery percentiles use settled samples while keeping genuine slow tails',()=>{
 const cfg={...config,resultsLimits:{...defaultResultsLimits,recoveryWindowSeconds:2},load:{start_rps:5,stages:[{target_rps:5,duration_seconds:10},{target_rps:5,duration_seconds:20,fault:{targetId:'vault',action:'outage'}},{target_rps:5,duration_seconds:30}]},faultPlan:[{index:1,target:{id:'vault',serviceType:'card-vault'}}]};
 const ev=[{at:10000,kind:'scheduled_fault',message:'scheduled step 2'},{at:10000,kind:'fault_requested'},{at:12000,kind:'fault_started'},{at:22000,kind:'fault_restore'},{at:30000,kind:'fault_restored'}];
 const req=(at,ms,extra={})=>({observed_at:at,latency_ms:20,operation:'payment_confirm',api_status_code:200,status:'succeeded',server_latency_ms:ms,...extra});
 const normal=req(5000,50),outage=req(16000,60),recovered=req(40000,70);
 const requests=[normal,outage,recovered,req(1000,9000),req(12500,9000),req(25000,9000),req(31000,9000),req(45000,9000,{flow_started_at:15000})];
 const input={config:cfg,recording:{...recording,started_at:0,ended_at:60000},events:ev,samples:Array.from({length:61},(_,i)=>({at:i*1000,active:i})),flows:[],jobStatus:'complete'};
 const result=evaluateScenarioChecks({...input,requests});
 assert.deepEqual(result.states.map(s=>[s.state,s.count,s.p99]),[['normal',1,50],['outage',1,60],['recovery',1,70]]);
 assert.equal(row(result.groups.find(g=>g.kind==='recovery'),'p99').actual,70);
 const slow=evaluateScenarioChecks({...input,requests:[...requests,req(50000,1500)]});
 assert.ok(slow.states.find(s=>s.state==='recovery').p99>1400);
 assert.equal(row(slow.groups.find(g=>g.kind==='recovery'),'p99').outcome,'fail');
 assert.equal(result.groups.find(g=>g.kind==='fault').checks.find(c=>c.id==='failed').actual,0);
});

test('steady measurements exclude recovery and late outage completions without trimming healthy tail latency',()=>{
 const start=1000,end=61000;
 const cfg={...config,faultPlan:[{index:1,target:{id:'vault',label:'Card vault',serviceType:'card-vault'}}],load:{start_rps:5,stages:[{target_rps:5,duration_seconds:20},{target_rps:5,duration_seconds:10,fault:{targetId:'vault',action:'outage'}},{target_rps:5,duration_seconds:30}]}};
 const ev=[{at:21000,kind:'scheduled_fault',message:'scheduled step 2'},{at:21000,kind:'fault_requested'},{at:22000,kind:'fault_started'},{at:30000,kind:'fault_restore'},{at:31000,kind:'fault_restored'}];
 const healthy=Array.from({length:200},(_,i)=>({observed_at:i<100?1050+i*200:41050+(i-100)*200,operation:'payment_confirm',api_status_code:200,status:'succeeded',latency_ms:10,server_latency_ms:i>=190?1000:50}));
 const delayed=Array.from({length:100},()=>({...healthy[0],observed_at:45000,latency_ms:20000,server_latency_ms:20000}));
 const recovery={...healthy[0],observed_at:46000,latency_ms:11000,server_latency_ms:11000};
 const requests=[...healthy,...delayed,recovery];
 const result=evaluateScenarioChecks({config:cfg,recording:{...recording,started_at:start,ended_at:end},events:ev,samples:Array.from({length:61},(_,i)=>({at:start+i*1000,active:i})),requests,flows:requests.map(r=>({at:r.observed_at,startedAt:r.observed_at-r.latency_ms,success:1})),jobStatus:'complete'});
 assert.equal(result.steady.durationSeconds,40);
 assert.equal(row(result.steady,'tps').actual,5);
 assert.equal(row(result.steady,'throughput').actual,100);
 assert.equal(row(result.steady,'completed').actual,200);
 assert.equal(row(result.steady,'p99').actual,50);
 assert.equal(row(result.steady,'p99').outcome,'pass');
 const recovered=result.groups.find(g=>g.kind==='recovery');
 assert.equal(row(recovered,'p99').actual,1000);assert.equal(row(recovered,'p99').limit,300);assert.equal(row(recovered,'p99').outcome,'fail');
 assert.equal(result.states.find(s=>s.state==='normal').count,50);
 assert.equal(result.states.find(s=>s.state==='outage').count,0);
 assert.equal(result.states.find(s=>s.state==='recovery').count,100);
});

test('holds count successful completions, payment HTTP errors and successful-only percentiles',()=>{
 const errors=[{...rows[0],api_status_code:500,server_latency_ms:50000},{...rows[1],api_status_code:200,status:'failed',server_latency_ms:60000},{...rows[2],operation:'pm_session_confirm',api_status_code:0}];
 const group=evaluate({requests:[...rows,...errors]}).groups[0];
 assert.equal(row(group,'throughput').actual,100);assert.equal(row(group,'throughput').outcome,'pass');
 assert.equal(row(group,'completed').actual,300);assert.equal(row(group,'p99').actual,50);
 assert.equal(row(group,'errors').actual,100/302);assert.equal(row(group,'errors').outcome,'fail');assert.equal(row(group,'drift').actual,0);
});

test('ramp target uses integrated offered rate, not its final target',()=>{
 const ramp={...config,load:{start_rps:0,stages:[{target_rps:20,duration_seconds:30}]}};
 const group=evaluate({config:ramp}).groups[0];
 assert.equal(group.kind,'ramp');assert.equal(row(group,'throughput').actual,100);assert.equal(row(group,'p99').limit,500);
});

test('pause plateaus do not inflate active-stage duration or offered work',()=>{
 const clock=executionClock([{at:1000,active:0},{at:11000,active:10},{at:21000,active:10},{at:31000,active:20}],recording);
 const windows=stageIntervals(clock,{start:0,end:20});assert.deepEqual(windows.map(w=>[w.from,w.to]),[[1000,11000],[21000,31000]]);
 const result=evaluate({config:{...config,load:{total_rps:10,duration_seconds:20}},samples:clock.points,flows:flows.filter(f=>f.at<11000||f.at>=21000)});
 assert.equal(row(result.groups[0],'throughput').actual,100);assert.equal(result.groups[0].durationSeconds,20);
 assert.equal(executionSample({control:{active_seconds:10,state:'running'}},{total_rps:10,duration_seconds:20},1000).stageIndex,0);
});

const faultConfig={...config,load:{start_rps:10,stages:[{target_rps:10,duration_seconds:10},{target_rps:10,duration_seconds:10,fault:{targetId:'vault',action:'outage'}},{target_rps:10,duration_seconds:10}]},faultPlan:[{index:1,target:{id:'vault',label:'Card vault'}}]};
const events=[{at:11000,kind:'scheduled_fault',message:'scheduled step 2'},{at:11000,kind:'fault_requested',message:'Card vault · outage requested'},{at:12000,kind:'fault_started',message:'Card vault · outage'},{at:21000,kind:'fault_restore',message:'Card vault · restoring'},{at:23000,kind:'fault_restored',message:'Card vault · recovery verified'}];
test('fault groups include delayed failures, exclude them from normal stages, and time recovery from restore',()=>{
 const error={observed_at:25000,operation:'pm_session_confirm',api_status_code:500,latency_ms:10000,server_latency_ms:4000};
 const summary=evaluate({config:faultConfig,events,requests:[...rows,error]});
 assert.deepEqual(summary.groups.map(g=>g.kind),['hold','fault','recovery','hold','integrity']);
 const fault=summary.groups.find(g=>g.kind==='fault'),after=summary.groups.find(g=>g.id==='stage-2');
 assert.equal(row(fault,'failed').actual,1);assert.equal(row(fault,'recovery').actual,2);assert.equal(row(fault,'errors').limit,null);
 assert.equal(row(after,'errors').actual,null);assert.equal(row(after,'throughput').limit,97);
 assert.equal(summary.groups.find(g=>g.kind==='recovery').state,'partial');
 assert.equal(row(summary.groups.at(-1),'lost').outcome,'not_measured');
});

test('DB availability ends outage before provider operation finishes, repeated faults stay separate',()=>{
 const stages=[{index:0,fault:{}},{index:1,fault:{}}];
 const event=(at,kind,message='')=>({at,kind,message});
 const faults=faultOccurrences([event(1,'scheduled_fault','scheduled step 1'),event(2,'fault_requested'),event(3,'database_unavailable'),event(4,'fault_restore'),event(5,'database_available'),event(10,'fault_restored'),event(11,'scheduled_fault','scheduled step 2'),event(12,'fault_requested'),event(13,'fault_restore'),event(15,'fault_restored')],stages,20);
 assert.equal(faults.length,2);assert.equal(faults[0].to,5);assert.equal(faults[1].recoveredAt,15);assert.equal(faults[1].stageIndex,1);
});

test('a recovery boundary overlapping the next stage does not disable its healthy checks',()=>{
 const group=evaluate({config:faultConfig,events}).groups.find(g=>g.id==='stage-2');
 assert.equal(row(group,'errors').limit,.1);assert.equal(row(group,'throughput').limit,97);
 assert.equal(group.state,'complete');assert.equal(group.durationSeconds,0);
});

test('unverified recovery over its deadline fails and simulations cannot pass a real recovery check',()=>{
 const incomplete=events.filter(e=>e.kind!=='fault_restored');
 const result=evaluate({config:faultConfig,events:incomplete,recording:{...recording,ended_at:100000},samples:[...samples,{at:100000,active:30}]});
 const recovery=row(result.groups.find(g=>g.kind==='fault'),'recovery');
 assert.equal(recovery.actual,79);assert.equal(recovery.outcome,'fail');assert.equal(recovery.lowerBound,true);
 const simulation=evaluate({config:faultConfig,events:events.map(e=>e.kind==='fault_started'?{...e,message:'SIMULATED · Card vault · outage'}:e)});
 assert.equal(row(simulation.groups.find(g=>g.kind==='fault'),'recovery').outcome,'recorded');
});

test('custom limits change only the supplied run snapshot',()=>{
 const slow=rows.map(r=>({...r,server_latency_ms:200}));
 assert.equal(row(evaluate({requests:slow}).groups[0],'p99').outcome,'fail');
 const relaxed={...config,resultsLimits:{...defaultResultsLimits,maximumHoldP99Ms:250}};
 assert.equal(row(evaluate({config:relaxed,requests:slow}).groups[0],'p99').outcome,'pass');
 assert.equal(evaluate().limits.maximumHoldP99Ms,160);
 assert.equal(row(evaluate({flows:flows.slice(0,200)}).groups[0],'throughput').outcome,'fail');
});

test('cancelled and missing-clock runs never pass unexecuted stages',()=>{
 const summary=evaluate({config:faultConfig,samples:samples.slice(0,6),jobStatus:'cancelled'});
 assert.equal(summary.verdict,'incomplete');assert.equal(summary.groups[0].state,'partial');assert.equal(row(summary.groups[0],'throughput').outcome,'not_measured');
 assert.equal(summary.groups[1].state,'not_run');assert.equal(summary.groups.find(g=>g.kind==='fault').verdict,'not_run');
 const missing=evaluate({samples:[]});assert.equal(row(missing.groups[0],'throughput').outcome,'not_measured');assert.notEqual(missing.verdict,'pass');
});

test('legacy runs preserve saved limits but do not invent server-latency thresholds',()=>{
 const old=evaluate({config:{load:config.load,acceptance:{minimumSuccessRatio:.95,maximumErrorPercent:1}}});
 assert.equal(row(old.groups[0],'throughput').limit,95);assert.equal(row(old.groups[0],'p99').limit,null);assert.equal(row(old.groups[0],'p99').outcome,'recorded');
 assert.equal(old.verdict,'recorded');
});

test('reconciliation is required before claiming zero acknowledged payments lost',()=>{
 const base={config:faultConfig,events};
 assert.equal(row(evaluate({...base,reconciliation:{recordingState:'complete',missing:0,total:300,expected:300}}).groups.at(-1),'lost').outcome,'pass');
 assert.equal(row(evaluate({...base,reconciliation:{recordingState:'complete',missing:1,total:300,expected:300}}).groups.at(-1),'lost').outcome,'fail');
 assert.equal(row(evaluate({...base,reconciliation:{recordingState:'incomplete',missing:0,total:300,expected:300}}).groups.at(-1),'lost').outcome,'not_measured');
});

test('Results limits reject invalid numbers, bounds and fractional lost-payment limits',()=>{
 for(const limits of [{minimumThroughputPercent:101},{maximumHttpErrorPercent:-1},{maximumHoldP99Ms:Infinity},{recoveryWindowSeconds:0},{maximumLostPayments:.5},{maximumRecoverySeconds:''}])assert.throws(()=>validateResultsLimits(limits),/Invalid Results limit/);
 assert.deepEqual(validateResultsLimits(),defaultResultsLimits);assert.equal(validateResultsLimits({maximumHoldP99Ms:'200'}).maximumHoldP99Ms,200);
});

test('SQLite-backed summaries persist, honor snapshots and refresh after reconciliation',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'scenario-checks-'));
 try{
  const db=new DatabaseSync(path.join(directory,'results.sqlite'));db.exec(await readFile('../k6/scenario-mix/recorder/schema.sql','utf8'));
  const insert=db.prepare("INSERT INTO request_events(event_id,run_id,attempt_id,flow_id,operation,role,observed_at,method,url,status,api_status_code,latency_ms,server_latency_ms) VALUES(?,'run',?,?,?,'measured',?,'POST','/','succeeded',200,10,50)");
  rows.forEach((r,i)=>{insert.run(i+1,i+1,`run:guest:1:${i}`,'payment_create',r.observed_at-20);insert.run(i+1001,i+1001,`run:guest:1:${i}`,'payment_confirm',r.observed_at);});db.close();
  await writeFile(path.join(directory,'execution.jsonl'),samples.map(s=>JSON.stringify(s)).join('\n'));
  const input={config:faultConfig,recording,events,jobStatus:'complete'};
  const first=await scenarioChecks(directory,input),artifact=JSON.parse(await readFile(path.join(directory,'scenario-checks.json')));
  assert.equal(row(first.groups[0],'completed').actual,100);assert.equal(artifact.summary.version,16);
  const second=await scenarioChecks(directory,{...input,reconciliation:{recordingState:'complete',total:300,expected:300,missing:0}});
  assert.equal(row(second.groups.at(-1),'lost').outcome,'pass');
  assert.notEqual(JSON.parse(await readFile(path.join(directory,'scenario-checks.json'))).fingerprint,artifact.fingerprint);
  const saved=structuredClone(config);validateResultsLimits({minimumThroughputPercent:80});
  assert.equal((await scenarioChecks(directory,{...input,config:saved})).limits.minimumThroughputPercent,97);
 }finally{await rm(directory,{recursive:true,force:true});}
});

test('confirmation completion counts only confirmations including drain, with strict success outside outages',()=>{
 const requests=[2050,3050,4150].map(at=>({operation:'payment_confirm',observed_at:at,api_status_code:200,status:'succeeded',latency_ms:150}));
 const input={config:{...config,load:{total_rps:1,duration_seconds:3}},recording:{...recording,ended_at:4200},samples:[{at:1000,active:0},{at:2000,active:1},{at:3000,active:2},{at:4000,active:3},{at:4200,active:3}],requests:[...requests,{operation:'customer_create',observed_at:1500,api_status_code:500}],jobStatus:'complete',control:{started_flows:4,dropped:0,active_seconds:3}};
 const result=evaluateScenarioChecks(input);
 assert.equal(result.completion.expected,3);assert.equal(result.completion.successful,3);assert.equal(result.completion.check.actual,100);assert.equal(result.completion.check.outcome,'pass');
 assert.equal(result.completion.started,4);assert.equal(result.completion.scope,'payment_confirm_outside_outage');
 const failed=evaluateScenarioChecks({...input,requests:[...requests.slice(0,2),{...requests[2],api_status_code:500,status:'failed'}]});
 assert.equal(failed.completion.check.outcome,'fail');
 const empty=evaluateScenarioChecks({...input,requests:[]});assert.equal(empty.completion.check.outcome,'not_measured');
 const cancelled=evaluateScenarioChecks({...input,jobStatus:'cancelled',recording:{...input.recording,recording_state:'incomplete'}});
 assert.notEqual(cancelled.completion.check.outcome,'pass');assert.equal(cancelled.verdict,'incomplete');
});
test('outage confirmation failures remain visible without failing outside-outage completion',()=>{
 const input={config:faultConfig,recording,samples,events,jobStatus:'complete',requests:[...rows,{operation:'payment_confirm',api_status_code:500,status:'failed',observed_at:22000,latency_ms:1000},{operation:'customer_create',api_status_code:500,observed_at:20000,latency_ms:100}],flows};
 const result=evaluateScenarioChecks(input);
 assert.equal(result.completion.check.outcome,'pass');assert.equal(result.completion.outageFailures,1);assert.ok(result.completion.outageAttempts>1);
 const bad=evaluateScenarioChecks({...input,requests:[...input.requests,{operation:'payment_confirm',api_status_code:500,status:'failed',observed_at:5000,latency_ms:10}]});
 assert.equal(bad.completion.check.outcome,'fail');
});

test('non-confirmation request durations cannot affect any server latency percentile',()=>{
 const result=evaluate({requests:[...rows,{...rows[0],operation:'customer_create',server_latency_ms:999999},{...rows[0],operation:'pm_session_confirm',server_latency_ms:999999}]});
 assert.equal(row(result.steady,'p99').actual,50);assert.equal(result.states.find(s=>s.state==='normal').p99,50);
});

test('unavailable informational outage latency cannot block measured recovery and integrity',()=>{
 const result=evaluate({config:faultConfig,events,requests:rows.filter(r=>r.observed_at<19000||r.observed_at>23000),reconciliation:{recordingState:'complete',total:300,expected:300,missing:0}});
 const fault=result.groups.find(g=>g.kind==='fault');
 assert.equal(row(fault,'p99').actual,null);assert.equal(row(fault,'p99').limit,null);assert.equal(row(fault,'p99').outcome,'recorded');
 assert.equal(row(result.groups.find(g=>g.kind==='integrity'),'lost').outcome,'pass');
});

test('outage payment-creation failures exclude successes, other operations and failures outside outage',()=>{
 const request=(operation,observed_at,api_status_code,latency_ms=10)=>({operation,observed_at,api_status_code,latency_ms});
 const result=evaluate({config:faultConfig,events,requests:[...rows,request('payment_create',20000,500),request('payment_create',26000,0,6000),request('payment_create',20000,200),request('customer_create',20000,500),request('pm_session_create',20000,500),request('payment_confirm',20000,500),request('payment_create',5000,500)]});
 assert.equal(row(result.groups.find(g=>g.kind==='fault'),'failed-creation').actual,2);
});

test('HTTP errors include payment create and confirm failures without dilution by setup requests',()=>{
 const request=(operation,api_status_code)=>({operation,api_status_code,observed_at:5000,latency_ms:10});
 const requests=[request('payment_create',200),request('payment_create',500),request('payment_confirm',200),request('payment_confirm',0),...Array.from({length:100},()=>request('customer_create',200)),request('pm_session_create',500)];
 const result=evaluate({requests});
 assert.equal(row(result.steady,'errors').actual,50);assert.equal(row(result.groups[0],'errors').actual,50);
 assert.equal(result.httpErrorScope,'payment_create_and_confirm');
});

test('DB recovery counts observed unavailability even when availability precedes scheduled restoration',()=>{
 const ev=[{at:10000,kind:'scheduled_fault',message:'scheduled step 2'},{at:10000,kind:'fault_requested'},{at:11000,kind:'database_unavailable'},{at:18000,kind:'database_available'},{at:21000,kind:'fault_restore'},{at:23000,kind:'fault_restored'}];
 const result=evaluate({config:faultConfig,events:ev});
 const recovery=row(result.groups.find(g=>g.kind==='fault'),'recovery');
 assert.equal(recovery.actual,7);assert.equal(recovery.outcome,'pass');
 assert.equal(recovery.label,'Database availability restored after outage');
 const pending=evaluate({config:faultConfig,events:ev.slice(0,3),recording:{...recording,ended_at:100000},samples:[...samples,{at:100000,active:30}]});
 const overdue=row(pending.groups.find(g=>g.kind==='fault'),'recovery');
 assert.equal(overdue.actual,89);assert.equal(overdue.outcome,'fail');
});
