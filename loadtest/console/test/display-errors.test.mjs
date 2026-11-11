import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {DatabaseSync} from 'node:sqlite';
import {faultWindows,hiddenVaultError,decorateDisplayErrors} from '../server/display-errors.mjs';
import {restoreDisplayErrors} from '../server/telemetry.mjs';

const events=[
 {at:1000,kind:'fault_started',message:'Card vault · outage'},
 {at:10000,kind:'fault_restore',message:'Card vault · restoring'},
 {at:20000,kind:'fault_restored',message:'Card vault · recovery verified'},
 {at:40000,kind:'database_unavailable',message:'DB · unavailable'},
 {at:50000,kind:'database_available',message:'DB · available'}
];
const timeout={observed_at:35000,latency_ms:30000,api_status_code:0,operation:'pm_session_confirm',message:'request timeout'};
test('vault filtering follows request overlap, preserves DB errors and other transport failures',()=>{
 const windows=faultWindows(events);
 assert.equal(hiddenVaultError(timeout,windows),true); // Completed after recovery; started during the outage.
 assert.equal(hiddenVaultError({...timeout,observed_at:45000},windows),false); // Also overlaps DB unavailability.
 assert.equal(hiddenVaultError({...timeout,operation:'payment_confirm'},windows),false);
 assert.equal(hiddenVaultError({...timeout,latency_ms:1000},windows),false);
 assert.equal(hiddenVaultError({...timeout,message:'connection reset'},windows),false);
 assert.equal(hiddenVaultError({...timeout,observed_at:15000,latency_ms:50,api_status_code:503},windows),true);
 assert.equal(hiddenVaultError({...timeout,observed_at:15000,latency_ms:50,api_status_code:400},windows),false);
 assert.equal(hiddenVaultError({...timeout,observed_at:45000,latency_ms:50,api_status_code:500},windows),false);
 assert.equal(hiddenVaultError(timeout,faultWindows([])),false);
});
test('live and historical display agree while raw errors remain intact, including incremental recording',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'vault-display-'));
 const db=new DatabaseSync(path.join(directory,'results.sqlite'));
 try{
  db.exec('CREATE TABLE runs(run_id TEXT,started_at INTEGER); INSERT INTO runs VALUES(\'run\',0); CREATE TABLE request_events(event_id INTEGER PRIMARY KEY,run_id TEXT,role TEXT,observed_at INTEGER,latency_ms REAL,operation TEXT,api_status_code INTEGER,message TEXT)');
  const insert=db.prepare("INSERT INTO request_events(run_id,role,observed_at,latency_ms,operation,api_status_code,message) VALUES('run','measured',?,?,?,?,?)");
  const rows=[{...timeout},{observed_at:36000,latency_ms:50,operation:'payment_confirm',api_status_code:200,message:''}];
  for(const row of rows)insert.run(row.observed_at,row.latency_ms,row.operation,row.api_status_code,row.message);
  const progress={elapsed_seconds:36,http_attempts:2,http_errors:1,trend:{t:36,intervalStart:0,intervalAttempts:2,intervalErrors:1}};
  let live=decorateDisplayErrors(progress,directory,events,{},0);
  assert.equal(live.display_error_rate,0);assert.equal(live.trend.errors,0);assert.equal(live.http_errors,1);
  // The recorder has persisted another timeout before k6 publishes its new
  // counters. Mixing these two clocks/counters used to create orange spikes.
  const behind=decorateDisplayErrors({...progress,http_errors:0},directory,events,{},-128);
  assert.equal(behind.display_error_rate,0);assert.equal(behind.trend.errors,0);
  const ahead=decorateDisplayErrors({...progress,http_errors:2},directory,events,{},-128);
  assert.equal(ahead.display_error_rate,0);assert.equal(ahead.trend.errors,0);
  const dbError={observed_at:45000,latency_ms:100,operation:'payment_create',api_status_code:500,message:'database unavailable'};
  insert.run(dbError.observed_at,dbError.latency_ms,dbError.operation,dbError.api_status_code,dbError.message);rows.push(dbError);
  live=decorateDisplayErrors({...progress,elapsed_seconds:45,http_attempts:3,http_errors:2,trend:{t:45,intervalStart:36,intervalAttempts:1,intervalErrors:1}},directory,events,{},0);
  const history=restoreDisplayErrors([{t:36},{t:45}],rows,0,events);
  assert.deepEqual(history.map(p=>p.errors),[0,100]);assert.equal(live.trend.errors,100);
  assert.equal(live.display_error_rate,33.33);assert.equal(live.hidden_vault_errors,1);assert.equal(live.http_errors,2);
  assert.equal(db.prepare('SELECT COUNT(*) AS n FROM request_events WHERE api_status_code=0 OR api_status_code>=400').get().n,2);
 }finally{db.close();await rm(directory,{recursive:true,force:true});}
});
