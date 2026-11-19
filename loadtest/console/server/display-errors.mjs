import {DatabaseSync} from 'node:sqlite';
import path from 'node:path';

// Presentation only: keep recordings and acceptance counters untouched.
export function faultWindows(events,config={}) {
 const labels=new Set(['Card vault',...(config.faultPlan||[]).filter(f=>f.target?.serviceType==='card-vault'||f.target?.id==='card-vault').map(f=>f.target.label)]);
 const vault=[],database=[];let vaultStart=null,databaseStart=null;
 for(const event of [...events].sort((a,b)=>a.at-b.at)) {
  const isVault=[...labels].some(label=>label&&event.message?.startsWith(`${label} ·`));
  if(isVault&&event.kind==='fault_started')vaultStart=event.at;
  if(isVault&&event.kind==='fault_restored'&&vaultStart!==null){vault.push([vaultStart,event.at]);vaultStart=null;}
  if(event.kind==='database_unavailable')databaseStart??=event.at;
  if(event.kind==='database_available'&&databaseStart!==null){database.push([databaseStart,event.at]);databaseStart=null;}
 }
 if(vaultStart!==null)vault.push([vaultStart,Infinity]);
 if(databaseStart!==null)database.push([databaseStart,Infinity]);
 return {vault,database};
}
export function hiddenVaultError(row,windows) {
 const end=Number(row.observed_at),start=end-(Number(row.latency_ms)||0);
 const overlaps=([from,to])=>start<to&&end>=from;
 if(!windows.vault.some(overlaps)||windows.database.some(overlaps))return false;
 return row.api_status_code>=500||
  (row.api_status_code===0&&row.operation==='pm_session_confirm'&&row.latency_ms>=29500&&row.latency_ms<=31000&&/request timeout/i.test(row.message||''));
}

// Read new failed attempts only, instead of rescanning a live recording on each SSE tick.
const failures=new Map();
export function displayErrorCounts(directory,events=[],config={},until=Infinity,since=-Infinity) {
 if(!directory)return null;
 let db;
 try {
  db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});
  db.exec('BEGIN');
  const run=db.prepare('SELECT run_id,started_at FROM runs ORDER BY started_at DESC LIMIT 1').get();
  if(!run)return null;
  let entry=failures.get(directory);
  if(!entry||entry.runId!==run.run_id){entry={runId:run.run_id,last:0,attempts:0,rows:[]};failures.set(directory,entry);if(failures.size>8)failures.delete(failures.keys().next().value);}
  const latest=db.prepare("SELECT COALESCE(MAX(event_id),0) AS id FROM request_events").get().id;
  const rows=db.prepare("SELECT event_id,observed_at,latency_ms,operation,api_status_code,message FROM request_events WHERE event_id>? AND event_id<=? AND run_id=? AND role='measured' AND (api_status_code=0 OR api_status_code>=400) ORDER BY event_id").all(entry.last,latest,run.run_id);
  const newAttempts=db.prepare("SELECT COUNT(*) AS n FROM request_events WHERE event_id>? AND event_id<=? AND run_id=? AND role='measured'").get(entry.last,latest,run.run_id).n;
  const intervalAttempts=since===-Infinity&&until===Infinity?entry.attempts+newAttempts:db.prepare("SELECT COUNT(*) AS n FROM request_events WHERE event_id<=? AND run_id=? AND role='measured' AND observed_at>? AND observed_at<=?").get(latest,run.run_id,since,until).n;
  entry.rows.push(...rows);entry.attempts+=newAttempts;entry.last=latest;db.exec('COMMIT');
  const windows=faultWindows(events,config);let hidden=0,intervalHidden=0,errors=0,intervalErrors=0;
  for(const row of entry.rows){
   const filtered=hiddenVaultError(row,windows);
   if(filtered)hidden++;else errors++;
   if(row.observed_at>since&&row.observed_at<=until){if(filtered)intervalHidden++;else intervalErrors++;}
  }
  return {hidden,intervalHidden,errors,intervalErrors,attempts:entry.attempts,intervalAttempts,startedAt:Number(run.started_at)};
 }catch{return null;}finally{db?.close();}
}
export function decorateDisplayErrors(progress,directory,events,config,startedAt) {
  const t=progress.elapsed_seconds;
 // Never subtract a persisted counter from an asynchronous k6 counter: one
 // can lead the other and briefly expose a timeout that is already filtered.
 let counts=displayErrorCounts(directory,events,config);
 if(counts&&progress.trend)counts=displayErrorCounts(directory,events,config,counts.startedAt+t*1000,counts.startedAt+(progress.trend.intervalStart||0)*1000);
 const attempts=counts?.attempts??(Number(progress.http_attempts)||0);
 const next={...progress,display_http_errors:counts?.errors??(Number(progress.http_errors)||0),hidden_vault_errors:counts?.hidden||0};
 if(progress.trend){const trend=progress.trend,intervalAttempts=counts?.intervalAttempts??trend.intervalAttempts,intervalErrors=counts?.intervalErrors??trend.intervalErrors;next.trend={...trend,errorScope:'all_http_except_vault',errors:intervalAttempts?100*intervalErrors/intervalAttempts:0};}
 next.display_error_rate=attempts?Number((100*next.display_http_errors/attempts).toFixed(2)):0;
 return next;
}
