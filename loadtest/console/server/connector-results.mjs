import {DatabaseSync} from 'node:sqlite';
import path from 'node:path';
import {latencyPercentile} from './successful-latency.mjs';
export function connectorResults(directory,plan){
 const db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
 try{
  const columns=db.prepare('PRAGMA table_info(request_events)').all().map(c=>c.name);
  if(!columns.includes('requested_connector'))return null;
  const assigned=db.prepare("SELECT requested_connector connector,COUNT(DISTINCT flow_id) assigned FROM request_events WHERE role='measured' AND requested_connector IS NOT NULL GROUP BY requested_connector").all();
  const confirmed=db.prepare("SELECT connector,COUNT(*) confirmations,SUM(CASE WHEN api_status_code BETWEEN 200 AND 299 AND status IN ('succeeded','requires_capture','processing') THEN 1 ELSE 0 END) successes FROM request_events WHERE role='measured' AND operation='payment_confirm' GROUP BY connector").all();
  const reportedTotal=confirmed.filter(r=>r.connector!=null).reduce((sum,r)=>sum+r.confirmations,0);
  const names=[...new Set([...assigned.map(r=>r.connector),...confirmed.map(r=>r.connector).filter(Boolean)])];
  return {rows:names.map(connector=>{
   const a=assigned.find(r=>r.connector===connector),c=confirmed.find(r=>r.connector===connector);
   const latencies=db.prepare("SELECT server_latency_ms,latency_ms FROM request_events WHERE role='measured' AND operation='payment_confirm' AND connector=? AND api_status_code BETWEEN 200 AND 299 AND status IN ('succeeded','requires_capture','processing')").all(connector);
   return {connector,assigned:a?.assigned||0,targetPercent:plan?.mix?.find(r=>r.connector===connector)?.weight??null,observedPercent:reportedTotal?100*(c?.confirmations||0)/reportedTotal:0,confirmations:c?.confirmations||0,successes:c?.successes||0,failures:(c?.confirmations||0)-(c?.successes||0),hsP99:latencyPercentile(latencies.map(r=>r.server_latency_ms).filter(Number.isFinite),.99),fullP99:latencyPercentile(latencies.map(r=>r.latency_ms).filter(Number.isFinite),.99)};
  }),unattributedConfirmations:confirmed.find(r=>r.connector==null)?.confirmations||0};
 }finally{db.close();}
}
