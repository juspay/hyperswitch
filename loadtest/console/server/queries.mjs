import { DatabaseSync } from 'node:sqlite';
import { readdir, stat, readFile } from 'node:fs/promises';
import path from 'node:path';
import pg from 'pg';
import { readJson, runDirectory, configPath,dataRoot } from './state.mjs';
import { readTelemetry } from './telemetry.mjs';
import {loadShape,loadTimeline} from '../src/load-shape.mjs';
import {evaluateAcceptance} from './acceptance.mjs';

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
  const shape=loadShape(config.load),duration=shape.duration;
  const elapsed = Math.min(duration,Math.max(0,(recording.ended_at || Date.now())-(recording.started_at || Date.now()))/1000);
  const flows=(config.scenarios || []).map(f=>({
    name:f.name,label:labels[f.scenario] || f.name,
    offeredTps:Number((shape.peak*f.weight/100).toFixed(2)),
    successes:value(summary,`scenario_success_${f.name}`,'count'),
    successfulTps:Number(value(summary,`achieved_tps_${f.name}`,'value').toFixed(2)),
    confirmP99:metric[`payment_confirm_ms_${f.name}`]?.values?.['p(99)'] == null ? null : Number(metric[`payment_confirm_ms_${f.name}`].values['p(99)'].toFixed(1)),
    confirmP90:metric[`payment_confirm_ms_${f.name}`]?.values?.['p(90)'] == null ? null : Math.round(metric[`payment_confirm_ms_${f.name}`].values['p(90)']),
    errors:value(summary,`scenario_failure_${f.name}`,'count')
  }));
  const result={id,...directory,path:undefined,runId:recording.run_id,startedAt:recording.started_at || (await stat(directory.path)).birthtimeMs,
    endedAt:recording.ended_at,recordingState:recording.recording_state,
    complete:recording.recording_state==='complete' && recording.enqueued===recording.persisted,
    profile:config.profile || null,fixtureSetId:config.fixtureSet?.id,merchantCount:config.fixtureSet?.merchantCount,
    timeline:loadTimeline(config.load,config.faultPlan),durationSeconds:duration,elapsedSeconds:Math.round(elapsed),offeredTps:shape.peak,expectedAverageTps:shape.average,
    iterations:live?.control?flows.reduce((s,f)=>s+f.successes+f.errors,0):value(summary,'iterations','count'),
    confirmP99:metric.payment_confirm_latency_ms?.values?.['p(99)'] == null ? null : Number(metric.payment_confirm_latency_ms.values['p(99)'].toFixed(1)),
    activeSeconds:live?.control?.active_seconds ?? elapsed,wallSeconds:live?.control?.wall_seconds ?? elapsed,successfulTps:Number(flows.reduce((s,f)=>s+f.successfulTps,0).toFixed(2)),
    errorRate:Number((value(summary,'http_req_failed','rate')*100).toFixed(2)),dropped:live?.control?.dropped ?? value(summary,'dropped_iterations','count'),
    enqueued:Number(recording.enqueued),persisted:Number(recording.persisted),gaps:Number(recording.enqueued)-Number(recording.persisted),payments,flows};
  if (!final && live) {
    result.live=true;result.loadState=live.control?.state || 'running';result.elapsedSeconds=Math.round(Math.min(duration,live.control?.active_seconds ?? live.elapsed_seconds));result.confirmP99=live.p99_ms ?? null;
    result.iterations=live.iterations;result.successfulTps=Number(live.successful_tps.toFixed(2));result.dropped=live.dropped;
    result.errorRate=live.http_attempts ? Number((100*live.http_errors/live.http_attempts).toFixed(2)) : 0;
    result.enqueued=live.recorder.enqueued;result.persisted=live.recorder.persisted;result.gaps=result.enqueued-result.persisted;
    for (const f of result.flows) Object.assign(f,live.flows[f.name] ? {successfulTps:Number(live.flows[f.name].tps.toFixed(2)),successes:live.flows[f.name].successes,errors:live.flows[f.name].failures,confirmP90:live.flows[f.name].p90_ms,confirmP99:live.flows[f.name].p99_ms} : {});
  }
  // A crash-recovered job cannot be mistaken for a running or complete recording.
  try {
    const control=new DatabaseSync(path.join(dataRoot,'console.sqlite'),{readOnly:true});
    try {
      const job=control.prepare("SELECT id,status,output FROM jobs WHERE run_path=? AND kind='load' ORDER BY started_at DESC LIMIT 1").get(directory.path);
      if(job)result.jobStatus=job.status;
      if(job && includeTelemetry)result.diagnostics=job.output;
      if(job)result.activity=control.prepare('SELECT event_id AS id,at,kind,message FROM job_events WHERE job_id=? ORDER BY event_id').all(job.id);
      if(job && ['complete','cancelled','failed','interrupted'].includes(job.status)){result.loadState='completed';result.live=false;}
      if(job?.status==='interrupted'){result.recordingState='incomplete';result.complete=false;result.live=false;}
    }finally{control.close();}
  }catch { /* Legacy CLI runs have no console job. */ }
  if(!result.activity){const lines=await readFile(path.join(directory.path,'activity.jsonl'),'utf8').catch(e=>{if(e.code==='ENOENT')return '';throw e;});result.activity=lines.trim().split('\n').filter(Boolean).flatMap(line=>{try{return [JSON.parse(line)];}catch{return [];}}).sort((a,b)=>a.id-b.id);}
  const reconciliation = await readJson(path.join(directory.path,'reconciliation.json'));
  if(reconciliation) result.reconciliation=reconciliation;
  if(live?.control)result.elapsedSeconds=Math.round(live.control.active_seconds);
  if(!result.live)result.loadState='completed';
  if(includeTelemetry)result.telemetry=await readTelemetry(directory.path);
  result.acceptance=evaluateAcceptance(result,config.acceptance);
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
  output.exec('CREATE TABLE IF NOT EXISTS transport_comparisons(merchant_id TEXT,payment_id TEXT,attempts INTEGER,database_status TEXT,outcome TEXT); DELETE FROM transport_comparisons;');
  const insertTransport=output.prepare('INSERT INTO transport_comparisons VALUES(?,?,?,?,?)');
  const pool=new pg.Pool({...database,ssl:database.ssl?{rejectUnauthorized:true}:false,max:1,connectionTimeoutMillis:5000});
  let client;
  const result={runId:run.runId,recordingState:run.recordingState,comparedAt:new Date().toISOString(),match:0,mismatch:0,missing:0,total:0,discrepancies:[],truncated:false};
  const started=Date.now();
  const expected=Number(input.prepare("SELECT COUNT(*) AS n FROM payments WHERE run_id=? AND role='measured'").get(run.runId).n);
  result.expected=expected;result.timeline=[{t:0,match:0,mismatch:0,missing:0,compared:0}];
  const report=()=>progress({compared:result.total,expected,match:result.match,mismatch:result.mismatch,missing:result.missing,elapsedSeconds:(Date.now()-started)/1000,timeline:result.timeline});
  report();
  try {
    client=await pool.connect();
    await client.query('BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY');await client.query("SET LOCAL statement_timeout='30s'");
    let last='',lastMerchant='';
    const batch=input.prepare("SELECT merchant_id,payment_id,status FROM payments WHERE run_id=? AND role='measured' AND (payment_id,merchant_id)>(?,?) ORDER BY payment_id,merchant_id LIMIT 1000");
    for(;;) {
      const recorded=batch.all(run.runId,last,lastMerchant);if(!recorded.length)break;
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
      last=recorded.at(-1).payment_id;lastMerchant=recorded.at(-1).merchant_id;
      result.timeline.push({t:(Date.now()-started)/1000,match:result.match,mismatch:result.mismatch,missing:result.missing,compared:result.total});
      if(result.timeline.length>600)result.timeline=result.timeline.filter((_,i,all)=>i%2===0||i===all.length-1);
      report();
    }
    const transport={attempts:0,checkedPayments:0,found:0,missing:0,unresolved:0,byOperation:[],details:[],truncated:false};
    const totals=input.prepare("SELECT COUNT(*) AS attempts,SUM(CASE WHEN payment_id IS NULL OR payment_id='' OR merchant_id IS NULL OR merchant_id='' THEN 1 ELSE 0 END) AS unresolved FROM request_events WHERE run_id=? AND api_status_code=0").get(run.runId);
    transport.attempts=Number(totals.attempts);transport.unresolved=Number(totals.unresolved||0);
    transport.byOperation=input.prepare("SELECT operation,COUNT(*) AS attempts,SUM(CASE WHEN payment_id IS NULL OR payment_id='' OR merchant_id IS NULL OR merchant_id='' THEN 1 ELSE 0 END) AS unresolved FROM request_events WHERE run_id=? AND api_status_code=0 GROUP BY operation").all(run.runId);
    input.exec('CREATE TEMP TABLE failed_payment_pairs(merchant_id TEXT,payment_id TEXT,attempts INTEGER)');
    input.prepare("INSERT INTO failed_payment_pairs SELECT merchant_id,payment_id,COUNT(*) FROM request_events WHERE run_id=? AND api_status_code=0 AND payment_id IS NOT NULL AND payment_id!='' AND merchant_id IS NOT NULL AND merchant_id!='' GROUP BY merchant_id,payment_id").run(run.runId);
    input.exec('CREATE INDEX failed_payment_pairs_cursor ON failed_payment_pairs(payment_id,merchant_id)');
    const failedBatch=input.prepare('SELECT * FROM failed_payment_pairs WHERE (payment_id,merchant_id)>(?,?) ORDER BY payment_id,merchant_id LIMIT 1000');
    const discrepancyMap=new Map(result.discrepancies.map(r=>[JSON.stringify([r.merchantId,r.paymentId]),r]));
    last='';lastMerchant='';
    for(;;){
      const rows=failedBatch.all(last,lastMerchant);if(!rows.length)break;
      const actual=await client.query(`SELECT p.merchant_id,p.payment_id,p.status::text AS status FROM payment_intent p JOIN unnest($1::text[],$2::text[]) AS wanted(merchant_id,payment_id) ON p.merchant_id=wanted.merchant_id AND p.payment_id=wanted.payment_id`,[rows.map(r=>r.merchant_id),rows.map(r=>r.payment_id)]);
      output.exec('BEGIN');
      try{const comparisons=comparePayments(rows,actual.rows);for(const [index,comparison] of comparisons.entries()){
        const row=rows[index];
        const outcome=comparison.outcome==='missing'?'missing':'found';transport[outcome]++;transport.checkedPayments++;
        const detail={merchantId:comparison.merchantId,paymentId:comparison.paymentId,attempts:row.attempts,databaseStatus:comparison.databaseStatus,outcome};
        insertTransport.run(detail.merchantId,detail.paymentId,detail.attempts,detail.databaseStatus,outcome);
        if(transport.details.length<100)transport.details.push(detail);else transport.truncated=true;
        const discrepancy=discrepancyMap.get(JSON.stringify([detail.merchantId,detail.paymentId]));if(discrepancy)discrepancy.reason=outcome==='missing'?'Transport failure; payment not found':'Transport failure; recorded status differs from PostgreSQL';
      }output.exec('COMMIT');}catch(e){output.exec('ROLLBACK');throw e;}
      last=rows.at(-1).payment_id;lastMerchant=rows.at(-1).merchant_id;
    }
    result.transport=transport;
    await client.query('COMMIT');
    if(!result.total&&!transport.attempts) throw new Error('No recorded measured payments in this run');
    result.elapsedSeconds=(Date.now()-started)/1000;
    const {atomicJson}=await import('./state.mjs');await atomicJson(path.join(directory,'reconciliation.json'),result);
    return result;
  } finally {
    if(client) {await client.query('ROLLBACK').catch(()=>{});client.release();}await pool.end();input.close();output.close();
  }
}

export async function testDatabase(database) {
  if(!database.user)throw new Error('Choose an existing PostgreSQL user in Settings → Database');
  const port=Number(database.port);if(!Number.isInteger(port)||port<1||port>65535)throw new Error('Invalid database port');
  const client=new pg.Client({...database,port,ssl:database.ssl?{rejectUnauthorized:true}:false,connectionTimeoutMillis:5000});
  try {
    await client.connect();await client.query('BEGIN READ ONLY');await client.query("SET LOCAL statement_timeout='5s'");
    await client.query('SELECT merchant_id,payment_id,status FROM payment_intent LIMIT 0');
    const result=await client.query('SELECT current_user AS username,current_database() AS database');
    return {ok:true,...result.rows[0],testedAt:new Date().toISOString(),message:'Connected · payment_intent is readable'};
  }catch(e){throw new Error(`Database connection check failed: ${e.message}. Check Settings → Database; no PostgreSQL role is created automatically.`);}
  finally{await client.end().catch(()=>{});}
}
