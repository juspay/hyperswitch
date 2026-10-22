import test from 'node:test';
import assert from 'node:assert/strict';
import {DatabaseSync} from 'node:sqlite';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import os from 'node:os';import path from 'node:path';
import {successfulServerLatency,latencyPhase,latencyPercentile} from '../server/successful-latency.mjs';
import {readTelemetry} from '../server/telemetry.mjs';
test('summary and existing chart values exclude HTTP and payment-status failures',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'successful-latency-'));
 try{
  const db=new DatabaseSync(path.join(directory,'results.sqlite'));
  db.exec('CREATE TABLE request_events(event_id INTEGER,run_id TEXT,operation TEXT,role TEXT,api_status_code INTEGER,status TEXT,server_latency_ms REAL,flow_id TEXT,observed_at INTEGER,latency_ms REAL DEFAULT 0)');
  const insert=db.prepare('INSERT INTO request_events(event_id,run_id,operation,role,api_status_code,status,server_latency_ms,flow_id,observed_at) VALUES(?,?,?,?,?,?,?,?,?)');
  const rows=[[200,'succeeded',100],[200,'requires_capture',110],[500,'succeeded',20000],[200,'failed',30000],[0,'succeeded',40000]];
  rows.forEach(([code,status,latency],i)=>insert.run(i,'run','payment_confirm','measured',code,status,latency,'merchant:guest:1:1',1200+i));
  insert.run(6,'failed-run','payment_confirm','measured',500,'failed',50000,'merchant:guest:1:1',1400);
  db.close();
  const recording={run_id:'run',started_at:1000,ended_at:2000,persisted:5};
  const result=successfulServerLatency(directory,recording);
  assert.equal(result.p99,109.9);assert.equal(result.phases[0].p50,105);assert.equal(result.phases[0].p95,109.5);assert.equal(result.phases[1].count,0);
  const phased=successfulServerLatency(directory,recording,[{at:1100,kind:'database_unavailable'},{at:1500,kind:'database_available'}]);assert.equal(phased.phases[1].count,2);assert.equal(phased.phases[0].count,0);assert.equal(result.flows.guest,109.9);
  assert.equal(successfulServerLatency(directory,{...recording,run_id:'failed-run'}).p99,null);
  await writeFile(path.join(directory,'telemetry.jsonl'),JSON.stringify({t:1,started:5,serverP99:40000,flows:{guest:{started:5,serverP99:40000}}})+'\n');
  const chart=await readTelemetry(directory,recording);
  assert.equal(chart[0].serverP99,109.9);assert.equal(chart[0].flows.guest.serverP99,109.9);
 }finally{await rm(directory,{recursive:true,force:true});}
});

test('phase classification uses request overlap, prioritizes outages, and bounds recovery',()=>{
 const windows={vault:[[1000,2000]],database:[[2500,3000]]};
 assert.equal(latencyPhase({observed_at:2200,latency_ms:300},windows),'outage');
 assert.equal(latencyPhase({observed_at:2400,latency_ms:100},windows),'recovery');
 assert.equal(latencyPhase({observed_at:2600,latency_ms:100},windows),'outage');
 assert.equal(latencyPhase({observed_at:14000,latency_ms:100},windows),'normal');
 assert.equal(latencyPercentile([],.5),null);assert.equal(latencyPercentile([0],.99),0);
});
