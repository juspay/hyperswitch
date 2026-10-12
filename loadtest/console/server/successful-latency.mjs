import {DatabaseSync} from 'node:sqlite';
import path from 'node:path';
import {faultWindows} from './display-errors.mjs';
export const successfulConfirmWhere="operation='payment_confirm' AND role='measured' AND api_status_code>=200 AND api_status_code<300 AND status IN ('succeeded','requires_capture','processing') AND server_latency_ms IS NOT NULL AND server_latency_ms>=0";
export function latencyPercentile(values,percentile){
 if(!values.length)return null;
 values.sort((a,b)=>a-b);const index=(values.length-1)*percentile,low=Math.floor(index),high=Math.ceil(index);
 return values[low]+(values[high]-values[low])*(index-low);
}
export const latencyP99=values=>latencyPercentile(values,.99);
export function latencyPhase(row,windows,recoverySeconds=10){
 const end=row.observed_at,start=end-(row.latency_ms||0),outages=[...windows.vault,...windows.database];
 const overlap=([from,to])=>start<to&&end>=from;
 if(outages.some(overlap))return 'outage';
 if(outages.filter(([,to])=>Number.isFinite(to)).map(([,to])=>[to,to+recoverySeconds*1000]).some(overlap))return 'recovery';
 return 'normal';
}
const cache=new Map();
export function successfulServerLatency(directory,recording,events=[],config={}){
 const windows=faultWindows(events,config),recoverySeconds=10;
 const key=recording?.ended_at?`${directory}:${recording.run_id}:${recording.persisted}:${recording.ended_at}:${JSON.stringify(windows)}`:null;
 if(key&&cache.has(key))return cache.get(key);
 let db;
 try{
  db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  if(!db.prepare('PRAGMA table_info(request_events)').all().some(c=>c.name==='server_latency_ms'))return null;
  const all=[],flows={},phases={normal:[],outage:[],recovery:[]};
  for(const row of db.prepare(`SELECT flow_id,server_latency_ms,observed_at,latency_ms FROM request_events WHERE run_id=? AND ${successfulConfirmWhere}`).iterate(recording.run_id)){
   if(!Number.isFinite(row.server_latency_ms))continue;
   all.push(row.server_latency_ms);const name=String(row.flow_id).split(':').at(-3);(flows[name]??=[]).push(row.server_latency_ms);
   phases[latencyPhase(row,windows,recoverySeconds)].push(row.server_latency_ms);
  }
  const result={p99:latencyP99(all),flows:Object.fromEntries(Object.entries(flows).map(([name,values])=>[name,latencyP99(values)])),recoverySeconds,
   phases:Object.entries(phases).map(([state,values])=>({state,count:values.length,p50:latencyPercentile(values,.5),p95:latencyPercentile(values,.95),p99:latencyP99(values)}))};
  if(key){cache.set(key,result);if(cache.size>8)cache.delete(cache.keys().next().value);}
  return result;
 }catch(error){if(error.code==='ERR_SQLITE_ERROR')return null;throw error;}finally{db?.close();}
}
