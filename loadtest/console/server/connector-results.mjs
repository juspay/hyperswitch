import {DatabaseSync} from 'node:sqlite';
import path from 'node:path';
import {latencyStatistics,containedLatencySample} from './successful-latency.mjs';

/** Count all measured confirmation responses, keeping unknown routing separate from confirmation failures. */
export function connectorResults(directory,plan,{latencyWindows=null}={}){
 const db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
 try{
  const columns=db.prepare('PRAGMA table_info(request_events)').all().map(c=>c.name);
  if(!columns.includes('requested_connector'))return null;
  const assigned=db.prepare("SELECT requested_connector connector,COUNT(DISTINCT flow_id) assigned FROM request_events WHERE role='measured' AND requested_connector IS NOT NULL GROUP BY requested_connector").all();
  const confirmed=db.prepare(`SELECT NULLIF(connector,'') connector,COUNT(*) confirmations,
    SUM(CASE WHEN api_status_code BETWEEN 200 AND 299 AND status IN ('succeeded','requires_capture','processing') THEN 1 ELSE 0 END) successes,
    SUM(CASE WHEN api_status_code BETWEEN 500 AND 599 THEN 1 ELSE 0 END) http5xx
    FROM request_events WHERE role='measured' AND operation='payment_confirm' GROUP BY NULLIF(connector,'')`).all();
  const reportedTotal=confirmed.filter(r=>r.connector!=null).reduce((sum,r)=>sum+r.confirmations,0);
  const names=[...new Set([...assigned.map(r=>r.connector).filter(Boolean),...confirmed.map(r=>r.connector).filter(Boolean)])];
  const samples=new Map();
  if(Array.isArray(latencyWindows)&&latencyWindows.length){
   // Compute flow starts before selecting confirmations: a short confirmation
   // must not conceal earlier flow work that overlapped a fault or transition.
   const latencies=db.prepare(`WITH measured AS (
     SELECT run_id,flow_id,operation,connector,server_latency_ms,latency_ms,observed_at,api_status_code,status,MIN(observed_at-COALESCE(latency_ms,0)) OVER (PARTITION BY run_id,flow_id) flow_started_at
     FROM request_events WHERE role='measured'
    ) SELECT NULLIF(connector,'') connector,server_latency_ms,latency_ms,observed_at,flow_started_at
    FROM measured WHERE operation='payment_confirm' AND api_status_code BETWEEN 200 AND 299
    AND status IN ('succeeded','requires_capture','processing')`).iterate();
   for(const row of latencies){
    if(!containedLatencySample(row,latencyWindows))continue;
    if(!samples.has(row.connector))samples.set(row.connector,{hs:[],full:[]});
    const sample=samples.get(row.connector);sample.hs.push(row.server_latency_ms);sample.full.push(row.latency_ms);
   }
  }
  const row=connector=>{
   const a=connector?assigned.find(r=>r.connector===connector):null,c=confirmed.find(r=>r.connector===connector);
   const values=samples.get(connector),latency={hs:latencyStatistics(values?.hs||[]),full:latencyStatistics(values?.full||[])};
   return {connector,assigned:connector?(a?.assigned||0):null,targetPercent:connector?(plan?.mix?.find(r=>r.connector===connector)?.weight??null):null,observedPercent:connector?(reportedTotal?100*(c?.confirmations||0)/reportedTotal:0):null,confirmations:c?.confirmations||0,successes:c?.successes||0,failures:(c?.confirmations||0)-(c?.successes||0),http5xx:c?.http5xx||0,latency,hsP99:latency.hs.p99,fullP99:latency.full.p99};
  };
  const unattributed=confirmed.find(r=>r.connector==null);
  return {scope:'all_periods',latencyScope:Array.isArray(latencyWindows)?'settled_steady_load':null,rows:names.map(row),inconclusive:unattributed?row(null):null,unattributedConfirmations:unattributed?.confirmations||0};
 }finally{db.close();}
}
