import {DatabaseSync} from 'node:sqlite';
import {existsSync} from 'node:fs';
import path from 'node:path';
import {runDirectory} from './state.mjs';
const outcomes=['match','missing','mismatch','pending','creation_unknown'];
export const bucketWidths=[1000,4000,16000,64000,256000,1024000,4096000,16384000,65536000];
export function createPaymentIndex(db){
 db.exec(`PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;
 CREATE TABLE IF NOT EXISTS reconciliation_meta(key TEXT PRIMARY KEY,value REAL);
 CREATE TABLE IF NOT EXISTS reconciliation_payments(id INTEGER PRIMARY KEY,elapsed_ms INTEGER NOT NULL,merchant_id TEXT NOT NULL,payment_id TEXT NOT NULL,recorded_status TEXT,database_status TEXT,outcome TEXT NOT NULL);
 CREATE INDEX IF NOT EXISTS recon_time ON reconciliation_payments(elapsed_ms,id);
 CREATE INDEX IF NOT EXISTS recon_outcome_time ON reconciliation_payments(outcome,elapsed_ms,id);
 CREATE TABLE IF NOT EXISTS reconciliation_buckets(level INTEGER,bucket INTEGER,matched INTEGER DEFAULT 0,missing INTEGER DEFAULT 0,mismatch INTEGER DEFAULT 0,pending INTEGER DEFAULT 0,PRIMARY KEY(level,bucket));`);
 const columns=new Set(db.prepare('PRAGMA table_info(reconciliation_payments)').all().map(r=>r.name));
 for(const [name,type] of Object.entries({event_id:'INTEGER',operation:'TEXT',api_status_code:'INTEGER',message:'TEXT',error_body:'TEXT',failure_category:'TEXT'}))if(!columns.has(name))db.exec(`ALTER TABLE reconciliation_payments ADD COLUMN ${name} ${type}`);
 if(!db.prepare('PRAGMA table_info(reconciliation_buckets)').all().some(r=>r.name==='creation_unknown'))db.exec('ALTER TABLE reconciliation_buckets ADD COLUMN creation_unknown INTEGER DEFAULT 0');
 db.exec("DROP INDEX IF EXISTS recon_identity; CREATE UNIQUE INDEX recon_identity ON reconciliation_payments(merchant_id,payment_id) WHERE payment_id!=''; CREATE INDEX IF NOT EXISTS recon_failure_time ON reconciliation_payments(failure_category,elapsed_ms,id)");
}
export function seedPaymentIndex(db,source,run){
 createPaymentIndex(db);db.prepare('ATTACH DATABASE ? AS recording').run(source);
 try{db.exec('BEGIN');db.exec('DELETE FROM reconciliation_payments; DELETE FROM reconciliation_buckets; DELETE FROM reconciliation_meta;');
 db.exec(`CREATE TEMP TABLE first_observed AS SELECT merchant_id,payment_id,MIN(observed_at) AS at FROM recording.request_events WHERE payment_id IS NOT NULL AND payment_id!='' GROUP BY merchant_id,payment_id;
 CREATE UNIQUE INDEX first_observed_identity ON first_observed(merchant_id,payment_id);`);
 db.prepare(`INSERT INTO reconciliation_payments(elapsed_ms,merchant_id,payment_id,recorded_status,outcome)
 SELECT MAX(0,CAST(COALESCE(f.at,p.observed_at)-? AS INTEGER)),p.merchant_id,p.payment_id,p.status,'pending' FROM recording.payments p LEFT JOIN first_observed f ON f.merchant_id=p.merchant_id AND f.payment_id=p.payment_id WHERE p.run_id=? AND p.role='measured' ORDER BY COALESCE(f.at,p.observed_at),p.merchant_id,p.payment_id`).run(run.startedAt,run.runId);
 db.exec('DROP TABLE first_observed');
 // Keep attempts without an ID as evidence, without inventing application payments.
 db.prepare(`INSERT INTO reconciliation_payments(elapsed_ms,merchant_id,payment_id,outcome,event_id,operation,api_status_code,message,error_body,failure_category)
 SELECT MAX(0,CAST(observed_at-? AS INTEGER)),COALESCE(merchant_id,''),'','creation_unknown',event_id,operation,api_status_code,message,error_body,'creation_unknown'
 FROM recording.request_events WHERE run_id=? AND role='measured' AND operation='payment_create' AND (payment_id IS NULL OR payment_id='') AND (api_status_code=0 OR api_status_code>=400) ORDER BY observed_at,event_id`).run(run.startedAt,run.runId);
 // A final failed confirmation is distinct from its database status comparison.
 db.prepare(`UPDATE reconciliation_payments SET (event_id,operation,api_status_code,message,error_body,failure_category)=(
 SELECT e.event_id,e.operation,e.api_status_code,e.message,e.error_body,'confirmation_failed' FROM recording.request_events e
 WHERE e.run_id=? AND e.role='measured' AND e.merchant_id=reconciliation_payments.merchant_id AND e.payment_id=reconciliation_payments.payment_id AND e.operation='payment_confirm'
 AND (e.api_status_code=0 OR e.api_status_code>=400) AND NOT EXISTS(SELECT 1 FROM recording.request_events later WHERE later.run_id=e.run_id AND later.merchant_id=e.merchant_id AND later.payment_id=e.payment_id AND later.operation='payment_confirm' AND later.event_id>e.event_id AND later.api_status_code BETWEEN 200 AND 299)
 ORDER BY e.event_id DESC LIMIT 1) WHERE payment_id!=''`).run(run.runId);
 rebuildBuckets(db);
 const end=db.prepare('SELECT MAX(elapsed_ms) AS end FROM reconciliation_payments').get().end||0;
 db.prepare('INSERT INTO reconciliation_meta VALUES(?,?)').run('duration',Math.max(run.durationSeconds*1000,end+1,1));
 db.prepare('INSERT INTO reconciliation_meta VALUES(?,?)').run('startedAt',run.startedAt);
 db.exec('COMMIT');}catch(e){db.exec('ROLLBACK');throw e;}finally{db.exec('DETACH DATABASE recording');}
}
export function rebuildBuckets(db){
 db.exec('DELETE FROM reconciliation_buckets');
 const insert=db.prepare(`INSERT INTO reconciliation_buckets SELECT ?,CAST(elapsed_ms/? AS INTEGER),SUM(outcome='match'),SUM(outcome='missing'),SUM(outcome='mismatch'),SUM(outcome='pending'),SUM(outcome='creation_unknown') FROM reconciliation_payments GROUP BY CAST(elapsed_ms/? AS INTEGER)`);
 bucketWidths.forEach((width,level)=>insert.run(level,width,width));
}
export function outcomeWriter(db){
 const update=db.prepare('UPDATE reconciliation_payments SET database_status=?,outcome=? WHERE merchant_id=? AND payment_id=? RETURNING elapsed_ms');
 const bucket=db.prepare('UPDATE reconciliation_buckets SET matched=matched+?,missing=missing+?,mismatch=mismatch+?,pending=pending-? WHERE level=? AND bucket=?');
 return rows=>{
  const deltas=new Map();
  for(const row of rows){const saved=update.get(row.databaseStatus,row.outcome,row.merchantId,row.paymentId);if(!saved)continue;
   bucketWidths.forEach((width,level)=>{const n=Math.floor(saved.elapsed_ms/width),key=`${level}:${n}`,counts=deltas.get(key)||[0,0,0,0,level,n];counts[outcomes.indexOf(row.outcome)]++;counts[3]++;deltas.set(key,counts);});
  }
  for(const counts of deltas.values())bucket.run(...counts);
 };
}
function openIndex(id){const db=new DatabaseSync(path.join(runDirectory(id).path,'reconciliation.sqlite'),{readOnly:true});db.exec('PRAGMA busy_timeout=5000');return db;}
const filterSql=filter=>filter==='discrepancies'?"outcome IN ('missing','mismatch','creation_unknown')":filter==='confirmation_failed'?"failure_category='confirmation_failed'":outcomes.includes(filter)?'outcome=?':'1=1';
const filterArgs=filter=>outcomes.includes(filter)?[filter]:[];
const counts=row=>({match:Number(row?.matched||0),missing:Number(row?.missing||0),mismatch:Number(row?.mismatch||0),pending:Number(row?.pending||0),creation_unknown:Number(row?.creation_unknown||0)});
export function paymentBars({id,from=0,to,bars=400,filter='all',detailLimit=600,view='auto'}){
 if(!existsSync(path.join(runDirectory(id).path,'reconciliation.sqlite')))return {preparing:true};
 const db=openIndex(id);
 try{
  if(!db.prepare("SELECT 1 FROM sqlite_master WHERE name='reconciliation_payments'").get())return {legacy:true};
  const savedDuration=db.prepare("SELECT value FROM reconciliation_meta WHERE key='duration'").get()?.value;if(!savedDuration)return {preparing:true};const duration=savedDuration;
  from=Math.max(0,Number(from)||0);to=Math.min(duration,to==null?duration:Number(to));bars=Math.max(20,Math.min(800,Math.floor(Number(bars)||400)));detailLimit=Math.max(bars,Math.min(800,Math.floor(Number(detailLimit)||600)));
  if(!Number.isFinite(to)||to<=from)throw Error('Invalid chart time range');
  const hasAttempts=db.prepare('PRAGMA table_info(reconciliation_buckets)').all().some(r=>r.name==='creation_unknown');
  const totals=counts(db.prepare(`SELECT SUM(matched) matched,SUM(missing) missing,SUM(mismatch) mismatch,SUM(pending) pending${hasAttempts?',SUM(creation_unknown) creation_unknown':''} FROM reconciliation_buckets WHERE level=0`).get());
  // Probe only enough rows to choose detail or aggregation; never transfer all payments.
  const rows=db.prepare(`SELECT * FROM reconciliation_payments WHERE elapsed_ms>=? AND elapsed_ms<? AND ${filterSql(filter)} ORDER BY elapsed_ms,id LIMIT ?`).all(from,to,...filterArgs(filter),detailLimit+1);
  if(rows.length<=detailLimit&&view!=='timeline')return {mode:'payments',from,to,duration,totals,rows};
  let level=bucketWidths.findIndex(w=>(to-from)/w<=bars);if(level<0)level=bucketWidths.length-1;const width=bucketWidths[level];
  const result=filter==='confirmation_failed'?db.prepare(`SELECT CAST(elapsed_ms/? AS INTEGER) bucket,SUM(outcome='match') matched,SUM(outcome='missing') missing,SUM(outcome='mismatch') mismatch,SUM(outcome='pending') pending,0 creation_unknown FROM reconciliation_payments WHERE elapsed_ms>=? AND elapsed_ms<? AND failure_category='confirmation_failed' GROUP BY bucket ORDER BY bucket`).all(width,from,to):db.prepare(`SELECT bucket,matched,missing,mismatch,pending${hasAttempts?',creation_unknown':''} FROM reconciliation_buckets WHERE level=? AND bucket>=? AND bucket<=? ORDER BY bucket`).all(level,Math.floor(from/width),Math.floor((to-1)/width));
  // Partial edge buckets use indexed exact queries; the interior uses pre-aggregates.
  for(const row of result){const a=Math.max(from,row.bucket*width),b=Math.min(to,(row.bucket+1)*width);if(a!==row.bucket*width||b!==(row.bucket+1)*width)Object.assign(row,db.prepare(`SELECT SUM(outcome='match') matched,SUM(outcome='missing') missing,SUM(outcome='mismatch') mismatch,SUM(outcome='pending') pending,SUM(outcome='creation_unknown') creation_unknown FROM reconciliation_payments WHERE elapsed_ms>=? AND elapsed_ms<? AND ${filterSql(filter)}`).get(a,b,...filterArgs(filter)));row.from=a;row.to=b;}
  return {mode:'buckets',from,to,duration,width,totals,rows:result.map(row=>({...row,...counts(row)}))};
 }finally{db.close();}
}
export function paymentPage({id,from=0,to=Number.MAX_SAFE_INTEGER,filter='discrepancies',afterTime=-1,afterId=0,limit=100,search=''}){
 limit=Math.max(1,Math.min(200,Math.floor(Number(limit)||100)));
 const db=openIndex(id);try{
  const rows=db.prepare(`SELECT * FROM reconciliation_payments WHERE elapsed_ms>=? AND elapsed_ms<? AND (elapsed_ms,id)>(?,?) AND ${filterSql(filter)} ${search?"AND (payment_id LIKE ? ESCAPE '\\' OR merchant_id LIKE ? ESCAPE '\\')":''} ORDER BY elapsed_ms,id LIMIT ?`).all(Number(from),Number(to),Number(afterTime),Number(afterId),...filterArgs(filter),...(search?Array(2).fill('%'+search.replace(/[\\%_]/g,'\\$&')+'%'):[]),Math.min(200,Number(limit)||100)+1);
  const end=rows[Math.min(limit,200)-1];return {rows:rows.slice(0,limit),next:rows.length>limit?{afterTime:end.elapsed_ms,afterId:end.id}:null};
 }finally{db.close();}
}
export function nextDiscrepancy({id,after=0}){
 const db=openIndex(id);try{return db.prepare("SELECT elapsed_ms,id FROM reconciliation_payments WHERE outcome IN ('missing','mismatch','creation_unknown') AND elapsed_ms>=? ORDER BY elapsed_ms,id LIMIT 1").get(Number(after))||null;}finally{db.close();}
}
export function paymentDetail({id,payment,merchant,rowId}){
 const db=openIndex(id);try{const row=rowId?db.prepare('SELECT * FROM reconciliation_payments WHERE id=?').get(Number(rowId)):db.prepare('SELECT * FROM reconciliation_payments WHERE payment_id=? AND merchant_id=?').get(payment,merchant);if(!row)throw Error('Payment or attempt not found');return row;}finally{db.close();}
}
