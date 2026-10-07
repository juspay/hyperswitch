import { DatabaseSync } from 'node:sqlite';
import { readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import pg from 'pg';
import { readJson, runDirectory, configPath,dataRoot } from './state.mjs';
import { readTelemetry } from './telemetry.mjs';

const value = (summary,name,key,fallback=0) => Number(summary?.metrics?.[name]?.values?.[key] ?? fallback);
const labels = { guest:'Guest payment',cit_off_session:'CIT off-session',cit_on_session:'CIT on-session',sdk_checkout:'SDK checkout',
  ptv_on_session:'Pay to vault · on-session',ptv_off_session:'Pay to vault · off-session',mit:'Merchant-initiated payment',
  cit_metadata_changed:'Card metadata update',saved_card_checkout:'Saved-card checkout' };

export async function describeRun(id,{includeTelemetry=true}={}) {
  const directory = runDirectory(id);
  const [config,summary,live] = await Promise.all([readJson(path.join(directory.path,'config.json'),{}),readJson(path.join(directory.path,'summary.json'),{}),readJson(path.join(directory.path,'progress.json'),null)]);
  let recording = {recording_state:'missing',enqueued:0,persisted:0}, payments=0;
  try {
    const db = new DatabaseSync(path.join(directory.path,'results.sqlite'),{readOnly:true});
    try {
      recording=db.prepare('SELECT * FROM runs ORDER BY started_at DESC LIMIT 1').get() || recording;
      // Live progress comes from k6's counters; do not scan projections during load.
      if(recording.ended_at) payments=Number(db.prepare("SELECT count(*) n FROM payments WHERE run_id=? AND role='measured'").get(recording.run_id).n);
    } finally { db.close(); }
  } catch(e) { if(e.code !== 'ERR_SQLITE_ERROR') throw e; }
  const metric = summary.metrics || {};
  const final = Boolean(summary.metrics);
  const duration = config.load?.duration_seconds || 0;
  const elapsed = Math.min(duration,Math.max(0,(recording.ended_at || Date.now())-(recording.started_at || Date.now()))/1000);
  const flows=(config.scenarios || []).map(f=>({
    name:f.name,label:labels[f.scenario] || f.name,
    offeredTps:Number(((config.load?.total_rps || 0)*f.weight/100).toFixed(2)),
    successes:value(summary,`scenario_success_${f.name}`,'count'),
    successfulTps:Number(value(summary,`achieved_tps_${f.name}`,'value').toFixed(2)),
    confirmP90:metric[`payment_confirm_ms_${f.name}`]?.values?.['p(90)'] == null ? null : Math.round(metric[`payment_confirm_ms_${f.name}`].values['p(90)']),
    errors:value(summary,`scenario_failure_${f.name}`,'count')
  }));
  const result={id,...directory,path:undefined,runId:recording.run_id,startedAt:recording.started_at || (await stat(directory.path)).birthtimeMs,
    endedAt:recording.ended_at,recordingState:recording.recording_state,
    complete:recording.recording_state==='complete' && recording.enqueued===recording.persisted,
    durationSeconds:duration,elapsedSeconds:Math.round(elapsed),offeredTps:config.load?.total_rps || 0,
    iterations:value(summary,'iterations','count'),successfulTps:Number(flows.reduce((s,f)=>s+f.successfulTps,0).toFixed(2)),
    errorRate:Number((value(summary,'http_req_failed','rate')*100).toFixed(2)),dropped:value(summary,'dropped_iterations','count'),
    enqueued:Number(recording.enqueued),persisted:Number(recording.persisted),gaps:Number(recording.enqueued)-Number(recording.persisted),payments,flows};
  if (!final && live) {
    result.live=true;result.elapsedSeconds=Math.round(Math.min(duration,live.elapsed_seconds));
    result.iterations=live.iterations;result.successfulTps=Number(live.successful_tps.toFixed(2));result.dropped=live.dropped;
    result.errorRate=live.http_attempts ? Number((100*live.http_errors/live.http_attempts).toFixed(2)) : 0;
    result.enqueued=live.recorder.enqueued;result.persisted=live.recorder.persisted;result.gaps=result.enqueued-result.persisted;
    for (const f of result.flows) Object.assign(f,live.flows[f.name] ? {successfulTps:Number(live.flows[f.name].tps.toFixed(2)),successes:live.flows[f.name].successes,errors:live.flows[f.name].failures,confirmP90:live.flows[f.name].p90_ms} : {});
  }
  // A crash-recovered job cannot be mistaken for a running or complete recording.
  try {
    const control=new DatabaseSync(path.join(dataRoot,'console.sqlite'),{readOnly:true});
    try {
      const job=control.prepare('SELECT status FROM jobs WHERE run_path=? ORDER BY started_at DESC LIMIT 1').get(directory.path);
      if(job?.status==='interrupted'){result.recordingState='incomplete';result.complete=false;result.live=false;}
    }finally{control.close();}
  }catch { /* Legacy CLI runs have no console job. */ }
  const reconciliation = await readJson(path.join(directory.path,'reconciliation.json'));
  if(reconciliation) result.reconciliation=reconciliation;
  if(includeTelemetry)result.telemetry=await readTelemetry(directory.path);
  return result;
}

export async function listRuns() {
  const config=await readJson(configPath), runs=[];
  for(const mode of ['single','tenancy']) {
    const entries=await readdir(path.join(config.output,mode,'runs'),{withFileTypes:true}).catch(e=>{if(e.code==='ENOENT')return [];throw e;});
    for(const entry of entries) if(entry.isDirectory()) {
      try { runs.push(await describeRun(`${mode}:${entry.name}`,{includeTelemetry:false})); } catch { /* Skip directories without readable run artifacts. */ }
    }
  }
  return runs.sort((a,b)=>b.startedAt-a.startedAt);
}

export function requestHistory({id,after=0,limit=100}) {
  const {path:directory}=runDirectory(id), db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  try {
    const rows=db.prepare(`SELECT event_id,flow_id,merchant_id,operation,role,observed_at,payment_id,customer_id,
      status,api_status_code,request_id,latency_ms,message,error_body FROM request_events WHERE event_id>? ORDER BY event_id LIMIT ?`).all(after,limit+1);
    return {rows:rows.slice(0,limit),next:rows.length>limit?rows[limit-1].event_id:null};
  } finally {db.close();}
}

export function comparePayments(recorded,actual) {
  const key=(merchant,payment)=>JSON.stringify([merchant,payment]);
  const statuses=new Map(actual.map(r=>[key(r.merchant_id,r.payment_id),r.status]));
  return recorded.map(r=>{
    const databaseStatus=statuses.get(key(r.merchant_id,r.payment_id));
    return {merchantId:r.merchant_id,paymentId:r.payment_id,recordedStatus:r.status,databaseStatus:databaseStatus ?? null,
      outcome:databaseStatus===undefined?'missing':databaseStatus===r.status?'match':'mismatch'};
  });
}

export async function reconcileRun({id,database,allowIncomplete=false}, progress=()=>{}) {
  const run=await describeRun(id), {path:directory}=runDirectory(id);
  if(!run.complete && !allowIncomplete) throw new Error('Recording is incomplete; explicitly allow incomplete data to inspect it');
  if(!run.runId) throw new Error('No recording database');
  if(!database.user) throw new Error('Configure a read-only database user in Settings');
  const input=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  const output=new DatabaseSync(path.join(directory,'reconciliation.sqlite'));
  output.exec(`CREATE TABLE IF NOT EXISTS comparisons(merchant_id TEXT,payment_id TEXT,recorded_status TEXT,database_status TEXT,outcome TEXT); DELETE FROM comparisons;`);
  const insert=output.prepare('INSERT INTO comparisons VALUES(?,?,?,?,?)');
  const pool=new pg.Pool({...database,ssl:database.ssl?{rejectUnauthorized:true}:false,max:1,connectionTimeoutMillis:5000});
  let client;
  const result={runId:run.runId,recordingState:run.recordingState,comparedAt:new Date().toISOString(),match:0,mismatch:0,missing:0,total:0,discrepancies:[],truncated:false};
  try {
    client=await pool.connect();
    await client.query('BEGIN READ ONLY');await client.query("SET LOCAL statement_timeout='30s'");
    let last='';
    const batch=input.prepare("SELECT merchant_id,payment_id,status FROM payments WHERE run_id=? AND role='measured' AND payment_id>? ORDER BY payment_id LIMIT 1000");
    for(;;) {
      const recorded=batch.all(run.runId,last);if(!recorded.length)break;
      const actual=await client.query(`SELECT p.merchant_id,p.payment_id,p.status::text AS status FROM payment_intent p
        JOIN unnest($1::text[],$2::text[]) AS wanted(merchant_id,payment_id)
        ON p.merchant_id=wanted.merchant_id AND p.payment_id=wanted.payment_id`,[recorded.map(r=>r.merchant_id),recorded.map(r=>r.payment_id)]);
      output.exec('BEGIN');
      try {
        for(const row of comparePayments(recorded,actual.rows)) {
          result[row.outcome]++;result.total++;insert.run(row.merchantId,row.paymentId,row.recordedStatus,row.databaseStatus,row.outcome);
          if(row.outcome!=='match') {if(result.discrepancies.length<200) result.discrepancies.push(row);else result.truncated=true;}
        }
        output.exec('COMMIT');
      } catch(e) {output.exec('ROLLBACK');throw e;}
      last=recorded.at(-1).payment_id;progress({compared:result.total});
    }
    await client.query('COMMIT');
    if(!result.total) throw new Error('No recorded measured payments in this run');
    const {atomicJson}=await import('./state.mjs');await atomicJson(path.join(directory,'reconciliation.json'),result);
    return result;
  } finally {
    if(client) {await client.query('ROLLBACK').catch(()=>{});client.release();}await pool.end();input.close();output.close();
  }
}
