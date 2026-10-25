import {compactSamples} from '../src/chart-timeline.mjs';
import { appendFile } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createInterface } from 'node:readline';
import path from 'node:path';
import {DatabaseSync} from 'node:sqlite';
import {faultWindows,hiddenVaultError} from './display-errors.mjs';
import {successfulConfirmWhere,latencyP99} from './successful-latency.mjs';

const legacyStarts=new Map();
const completedFlows=p=>Object.values(p.flows||{}).reduce((sum,f)=>sum+(Number(f.successes)||0)+(Number(f.failures)||0),0);
const admitted=p=>Number.isFinite(p.started)?p.started:completedFlows(p)+(Number(p.inFlight)||0);

export function telemetryPoint(progress,previous=null) {
  if(!Number.isFinite(progress.elapsed_seconds))return null;
  const successes=Object.values(progress.flows || {}).reduce((sum,flow)=>sum+(Number(flow.successes)||0),0);
  const completed=Object.values(progress.flows||{}).reduce((sum,flow)=>sum+(Number(flow.successes)||0)+(Number(flow.failures)||0),0);
  const started=Number.isFinite(progress.control?.started_flows)?progress.control.started_flows:completed+(Number(progress.control?.in_flight)||0);
  const interval=progress.elapsed_seconds-(previous?.t || 0);
  const httpAttempts=Number(progress.http_attempts)||0,httpErrors=Number(progress.http_errors)||0;
  const confirmHttpAttempts=progress.confirm_http_attempts ?? null,confirmHttpErrors=progress.confirm_http_errors ?? null;
  const attemptsDelta=Math.max(0,httpAttempts-(previous?.httpAttempts||0)),errorsDelta=Math.max(0,httpErrors-(previous?.httpErrors||0));
  // Use counter deltas over the displayed interval, including the final partial sample.
  return {t:progress.elapsed_seconds,successes,started,tps:interval>0?Math.max(0,started-(previous?admitted(previous):0))/interval:0,
    successfulTps:interval>0?(successes-(previous?.successes || 0))/interval:0,
    httpAttempts,httpErrors,confirmHttpAttempts,confirmHttpErrors,errorScope:'all_http',intervalStart:previous?.t||0,intervalAttempts:attemptsDelta,intervalErrors:errorsDelta,errors:attemptsDelta?100*errorsDelta/attemptsDelta:0,
    recorded:progress.recorder.persisted,serverP99:progress.server_p99_ms ?? null,p99:progress.p99_ms ?? null,activeSeconds:progress.control?.active_seconds ?? progress.elapsed_seconds,
    state:progress.control?.state || 'running',offeredTps:progress.control?.offered_tps ?? null,
    inFlight:progress.control?.in_flight ?? null,backlog:progress.recorder.enqueued-progress.recorder.persisted,
    flows:Object.fromEntries(Object.entries(progress.flows || {}).map(([name,f])=>{const started=progress.control?.flow_starts?.[name]??((Number(f.successes)||0)+(Number(f.failures)||0));const old=previous?.flows?.[name];return [name,{successes:Number(f.successes)||0,failures:Number(f.failures)||0,started,serverP99:f.server_p99_ms ?? null,p90:f.p90_ms,p99:f.p99_ms,tps:interval>0?Math.max(0,started-(old?.started??((Number(old?.successes)||0)+(Number(old?.failures)||0))))/interval:0,successfulTps:interval>0?((Number(f.successes)||0)-(old?.successes || 0))/interval:0}];}))};
}
export async function appendTelemetry(directory,point) {
  if(point)await appendFile(path.join(directory,'telemetry.jsonl'),JSON.stringify(point)+'\n',{mode:0o600});
}
// Earlier recordings contain completion counters only. Reconstruct their traffic
// from the first HTTP attempt per flow, including attempts that never returned an ID.
function recordedStarts(directory,recording){
 if(!recording?.ended_at)return null;
 const key=`${directory}:${recording.ended_at}:${recording.persisted}`;
 if(legacyStarts.has(key))return legacyStarts.get(key);
 let db;const bins=new Map();
 try{db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  for(const row of db.prepare('SELECT flow_id,MIN(observed_at-latency_ms) AS at FROM request_events WHERE run_id=? GROUP BY flow_id').iterate(recording.run_id)){
   const t=Math.max(0,Math.ceil((Number(row.at)-recording.started_at)/100)/10),name=String(row.flow_id).split(':').at(-3),key=`${t}:${name}`;
   const bin=bins.get(key)||{t,name,count:0};bin.count++;bins.set(key,bin);
  }
 }catch{return null;}finally{db?.close();}
 const starts=[...bins.values()].sort((a,b)=>a.t-b.t);
 legacyStarts.set(key,starts);if(legacyStarts.size>8)legacyStarts.delete(legacyStarts.keys().next().value);
 return starts;
}
export function startedTelemetry(points,starts=null){
 let previous=null,index=0,total=0;const byFlow={};
 return points.map(point=>{
  if(starts){while(index<starts.length&&starts[index].t<=point.t){const count=starts[index].count??1;total+=count;byFlow[starts[index].name]=(byFlow[starts[index].name]||0)+count;index++;}}
  const started=point.started??(starts?total:Math.max(previous?.started||0,admitted(point))),interval=point.t-(previous?.t||0);
  const flows=Object.fromEntries(Object.entries(point.flows||{}).map(([name,f])=>{const count=f.started??(starts?byFlow[name]||0:Object.keys(point.flows).length===1?started:(Number(f.successes)||0)+(Number(f.failures)||0));return [name,{...f,started:count,successfulTps:f.successfulTps??f.tps,tps:interval>0?Math.max(0,count-(previous?.flows?.[name]?.started||0))/interval:0}];}));
  const next={...point,flows,started,successfulTps:point.successfulTps??point.tps,tps:interval>0?Math.max(0,started-(previous?.started||0))/interval:0};previous=next;return next;
 });
}
// Restore latency intervals when an older supervisor omitted the chart field.
export function restoreServerLatency(points,rows,startedAt,replace=false){
 const iterator=rows[Symbol.iterator]();let current=iterator.next(),previous=0;
 const percentile=values=>{values.sort((a,b)=>a-b);return values.length?values[Math.floor((values.length-1)*.99)]:null;};
 const result=points.map(point=>{
  const values=[],flows={};
  while(!current.done&&(current.value.observed_at-startedAt)/1000<=point.t){
   const row=current.value,value=row.server_latency_ms;current=iterator.next();
   if((row.observed_at-startedAt)/1000<=previous||!Number.isFinite(value)||value<0)continue;
   if(values.length<8192)values.push(value);
   const name=String(row.flow_id).split(':').at(-3);flows[name]||=[];if(flows[name].length<8192)flows[name].push(value);
  }
  previous=point.t;
  return {...point,serverP99:replace?latencyP99(values):point.serverP99===undefined?percentile(values):point.serverP99,flows:Object.fromEntries(Object.entries(point.flows||{}).map(([name,f])=>[name,{...f,serverP99:replace?latencyP99(flows[name]||[]):f.serverP99===undefined?percentile(flows[name]||[]):f.serverP99}]))};
 });
 iterator.return?.();return result;
}
function recoverServerLatency(directory,recording,points){
 if(!recording?.started_at)return points;
 let db;
 try{db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  if(!db.prepare('PRAGMA table_info(request_events)').all().some(c=>c.name==='server_latency_ms'))return points;
  const rows=db.prepare(`SELECT observed_at,flow_id,server_latency_ms FROM request_events WHERE run_id=? AND ${successfulConfirmWhere} ORDER BY observed_at,event_id`).iterate(recording.run_id);
  return restoreServerLatency(points,rows,recording.started_at,true);
 }catch{return points;}finally{db?.close();}
}
export function restoreDisplayErrors(points,rows,startedAt,events=[],config={}){
 const iterator=rows[Symbol.iterator]();let current=iterator.next(),attempts=0,errors=0,hidden=0,previousAttempts=0,previousErrors=0;
 const windows=faultWindows(events,config);
 const result=points.map(point=>{
  while(!current.done&&(current.value.observed_at-startedAt)/1000<=point.t){
   const row=current.value;attempts++;
   if(row.api_status_code===0||row.api_status_code>=400){if(hiddenVaultError(row,windows))hidden++;else errors++;}
   current=iterator.next();
  }
  const delta=attempts-previousAttempts,failed=errors-previousErrors;previousAttempts=attempts;previousErrors=errors;
  return {...point,errorScope:'all_http_except_vault',displayHttpAttempts:attempts,displayHttpErrors:errors,hiddenVaultErrors:hidden,errors:delta?100*failed/delta:0};
 });iterator.return?.();return result;
}
function recoverDisplayErrors(directory,recording,points,events,config){
 if(!recording?.started_at)return points;
 let db;
 try{db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  const columns=new Set(db.prepare('PRAGMA table_info(request_events)').all().map(c=>c.name));
  const optional=['latency_ms','message'].map(c=>columns.has(c)?c:`NULL AS ${c}`).join(',');
  const rows=db.prepare(`SELECT observed_at,api_status_code,operation,${optional} FROM request_events WHERE run_id=? AND role='measured' ORDER BY observed_at,event_id`).iterate(recording.run_id);
  return restoreDisplayErrors(points,rows,recording.started_at,events,config);
 }catch{return points;}finally{db?.close();}
}
export async function readTelemetry(directory,recording=null,events=[],config={}) {
  const stream=createReadStream(path.join(directory,'telemetry.jsonl'),{encoding:'utf8'});
  const lines=createInterface({input:stream,crlfDelay:Infinity});let points=[];
  try {
    for await(const line of lines){
      try {const point=JSON.parse(line);if(Number.isFinite(point.t))points.push(point);}catch{/* Ignore an interrupted trailing write. */}
      if(points.length>3600)points=compactSamples(points);
    }
  }catch(e){if(e.code!=='ENOENT')throw e;}
  finally{lines.close();stream.destroy();}
  return compactSamples(startedTelemetry(recoverDisplayErrors(directory,recording,recoverServerLatency(directory,recording,points),events,config),points.some(p=>p.started==null)?recordedStarts(directory,recording):null));
}
