import test from 'node:test';import assert from 'node:assert/strict';import {mkdtemp,writeFile} from 'node:fs/promises';import {DatabaseSync} from 'node:sqlite';import os from 'node:os';import path from 'node:path';
import {readTelemetry,startedTelemetry} from '../server/telemetry.mjs';
import {aggregate} from '../server/distributed-runner.mjs';
test('live and distributed counters include HTTP errors from every operation',async()=>{
 const {telemetryPoint}=await import('../server/telemetry.mjs');
 const p={elapsed_seconds:1,http_attempts:30,http_errors:20,confirm_http_attempts:10,confirm_http_errors:1,recorder:{persisted:30,enqueued:30},flows:{}};
 assert.equal(telemetryPoint(p).errors,100*20/30);
 assert.equal(telemetryPoint({...p,elapsed_seconds:2,http_attempts:40,http_errors:25,confirm_http_attempts:20},telemetryPoint(p)).errors,50);
 const merged=aggregate([{progress:JSON.stringify(p)},{progress:JSON.stringify({...p,confirm_http_attempts:5,confirm_http_errors:0})}]);
 assert.equal(merged.confirm_http_attempts,15);assert.equal(merged.confirm_http_errors,1);assert.equal(merged.http_errors,40);
});
test('historical error graph includes measured requests across all operations',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'confirm-errors-')),db=new DatabaseSync(path.join(directory,'results.sqlite'));
 db.exec('CREATE TABLE request_events(event_id INTEGER PRIMARY KEY,run_id TEXT,operation TEXT,role TEXT,observed_at INTEGER,api_status_code INTEGER)');
 const insert=db.prepare('INSERT INTO request_events(run_id,operation,role,observed_at,api_status_code) VALUES(?,?,?,?,?)');
 insert.run('run','pm_session_confirm','measured',1500,0);insert.run('run','payment_confirm','fixture',1500,500);
 insert.run('run','payment_confirm','measured',1500,200);insert.run('run','payment_confirm','measured',2500,500);insert.run('run','payment_confirm','measured',2500,200);insert.run('run','pm_session_confirm','measured',3500,400);insert.run('run','payment_confirm','measured',3500,200);db.close();
 await writeFile(path.join(directory,'telemetry.jsonl'),[1,2,3].map(t=>JSON.stringify({t,errors:100,started:0,flows:{}})).join('\n'));
 const points=await readTelemetry(directory,{run_id:'run',started_at:1000,ended_at:4000,persisted:7});
 assert.deepEqual(points.map(p=>p.errors),[50,50,50]);assert.equal(points.at(-1).displayHttpAttempts,6);assert.equal(points.at(-1).displayHttpErrors,3);
});
test('old recording traffic is reconstructed from first attempts, including failed creates',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'flow-starts-')),db=new DatabaseSync(path.join(directory,'results.sqlite'));
 db.exec('CREATE TABLE request_events(run_id TEXT,flow_id TEXT,observed_at INTEGER,latency_ms REAL)');const insert=db.prepare('INSERT INTO request_events VALUES(?,?,?,?)');
 insert.run('run','run:guest:1:1',6100,5000);insert.run('run','run:guest:2:1',6200,5000);insert.run('run','run:guest:1:1',6300,20);db.close();
 await writeFile(path.join(directory,'telemetry.jsonl'),[{t:2,successes:0,tps:0,flows:{guest:{successes:0,failures:0,tps:0}}},{t:7,successes:1,tps:.2,flows:{guest:{successes:1,failures:1,tps:.2}}}].map(JSON.stringify).join('\n'));
 const points=await readTelemetry(directory,{run_id:'run',started_at:1000,ended_at:8000,persisted:3});
 assert.equal(points[0].started,2);assert.equal(points[0].tps,1);assert.equal(points[0].flows.guest.tps,1);assert.equal(points[1].tps,0);assert.equal(points[1].successfulTps,.2);
});
test('compacted counter history retains rate over the full sampled interval',()=>{
 const points=startedTelemetry([{t:1,started:5,successfulTps:0,flows:{guest:{started:5,successfulTps:0}}},{t:3,started:15,successfulTps:0,flows:{guest:{started:15,successfulTps:0}}}]);assert.equal(points[1].tps,5);assert.equal(points[1].flows.guest.tps,5);
});
test('distributed traffic sums admissions before responses return',()=>{
 const progress=started_flows=>JSON.stringify({elapsed_seconds:10,control:{active_seconds:10,started_flows,flow_starts:{guest:started_flows},in_flight:started_flows},flows:{guest:{successes:0,failures:0}}});
 const p=aggregate([{status:'running',progress:progress(50)},{status:'running',progress:progress(30)}]);assert.equal(p.control.started_flows,80);assert.equal(p.control.flow_starts.guest,80);assert.equal(p.control.in_flight,80);
 const legacy=aggregate([{status:'running',progress:'{}'}]);assert.equal(legacy.control.started_flows,undefined);
});

test('server latency propagates and aggregates samples instead of worker percentiles',async()=>{
 const {telemetryPoint}=await import('../server/telemetry.mjs');
 const point=telemetryPoint({elapsed_seconds:1,server_p99_ms:0,recorder:{persisted:0,enqueued:0},flows:{guest:{server_p99_ms:12}}});
 assert.equal(point.serverP99,0);assert.equal(point.flows.guest.serverP99,12);
 const shards=[{status:'running',progress:JSON.stringify({server_samples_ms:[0,10],flows:{guest:{server_samples_ms:[0,10]}}})},{status:'running',progress:JSON.stringify({server_samples_ms:[30],flows:{guest:{server_samples_ms:[30]}}})}];
 assert.equal(aggregate(shards).server_p99_ms,10);assert.equal(aggregate(shards).flows.guest.server_p99_ms,10);assert.equal(aggregate([{progress:'{}'}]).server_p99_ms,null);
});

test('older chart points recover server latency from recorded responses by interval',async()=>{
 const {restoreServerLatency}=await import('../server/telemetry.mjs');
 const points=[{t:1,flows:{guest:{}}},{t:2,flows:{guest:{}}},{t:3,flows:{guest:{}}},{t:4,serverP99:99,flows:{guest:{serverP99:99}}}];
 const rows=[{observed_at:1500,flow_id:'run:guest:1:1',server_latency_ms:0},{observed_at:1800,flow_id:'run:guest:2:1',server_latency_ms:12},{observed_at:2500,flow_id:'run:guest:3:1',server_latency_ms:20}];
 const result=restoreServerLatency(points,rows,1000);assert.deepEqual(result.map(p=>p.serverP99),[0,20,null,99]);assert.equal(result[1].flows.guest.serverP99,20);assert.equal(result[3].flows.guest.serverP99,99);
});
