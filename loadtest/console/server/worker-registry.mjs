import {createHash,randomBytes,randomUUID,timingSafeEqual} from 'node:crypto';
import {controlDb} from './state.mjs';
import {HttpError} from './errors.mjs';

const hash=value=>createHash('sha256').update(String(value)).digest('hex');
const parsed=value=>{try{return JSON.parse(value||'{}');}catch{return {};}};
const safeEqual=(left,right)=>{const a=Buffer.from(String(left)),b=Buffer.from(String(right));return a.length===b.length&&timingSafeEqual(a,b);};
const publicWorker=row=>({id:row.id,name:row.name,status:Date.now()-row.last_seen>15000?'offline':row.status,labels:parsed(row.labels),capacityTps:row.capacity_tps,capacityVus:row.capacity_vus,version:row.version,lastSeen:row.last_seen,createdAt:row.created_at});

export function listWorkers(){return controlDb().prepare('SELECT * FROM load_workers ORDER BY name,id').all().map(publicWorker);}
export function enrollWorker(input,providedToken,expectedToken=process.env.LOADTEST_WORKER_ENROLLMENT_TOKEN){
  if(!expectedToken||!safeEqual(providedToken,expectedToken))throw new HttpError(401,'Worker enrollment is not authorized');
  const name=String(input.name||'').trim(),capacityTps=Number(input.capacityTps),capacityVus=Number(input.capacityVus);
  if(!/^[A-Za-z0-9][A-Za-z0-9._-]{0,79}$/.test(name)||!Number.isInteger(capacityTps)||capacityTps<1||!Number.isInteger(capacityVus)||capacityVus<1)throw new Error('Worker needs a valid name and positive TPS/VU capacity');
  const id=randomUUID(),token=randomBytes(32).toString('base64url'),now=Date.now();
  controlDb().prepare("INSERT INTO load_workers(id,name,token_hash,status,labels,capacity_tps,capacity_vus,version,created_at,last_seen) VALUES(?,?,?,'idle',?,?,?,?,?,?)")
    .run(id,name,hash(token),JSON.stringify(input.labels||{}),capacityTps,capacityVus,String(input.version||''),now,now);
  return {id,token};
}
export function registerLocalWorker(input={}){
  const db=controlDb(),id='local',now=Date.now(),existing=db.prepare('SELECT * FROM load_workers WHERE id=?').get(id),token=input.token||randomBytes(32).toString('base64url');
  if(existing){db.prepare("UPDATE load_workers SET status='idle',last_seen=?,version=?,capacity_tps=?,capacity_vus=? WHERE id=?").run(now,String(input.version||process.version),Number(input.capacityTps||100),Number(input.capacityVus||1000),id);return {id,token:null};}
  db.prepare("INSERT INTO load_workers(id,name,token_hash,status,labels,capacity_tps,capacity_vus,version,created_at,last_seen) VALUES(?,'Local worker',?,'idle','{\"environment\":\"local\"}',?,?,?,?,?)")
    .run(id,hash(token),Number(input.capacityTps||100),Number(input.capacityVus||1000),String(input.version||process.version),now,now);return {id,token};
}
export function authenticateWorker(id,token){const row=controlDb().prepare('SELECT * FROM load_workers WHERE id=?').get(id);if(!row||!safeEqual(row.token_hash,hash(token)))throw new HttpError(401,'Worker authentication failed');return row;}
export function heartbeatWorker(id,token,input={}){
  authenticateWorker(id,token);const status=['idle','reserved','preparing','ready','running','pausing','paused','draining','uploading','failed'].includes(input.status)?input.status:'idle',db=controlDb();
  db.prepare('UPDATE load_workers SET status=?,last_seen=?,version=COALESCE(?,version) WHERE id=?').run(status,Date.now(),input.version||null,id);
  if(input.runId){
    const shardStatus=['reserved','preparing','ready','running','pausing','paused','draining','uploading','complete','failed'].includes(input.runStatus)?input.runStatus:null;
    if(shardStatus)db.prepare('UPDATE run_shards SET status=?,error=?,progress=?,updated_at=? WHERE run_id=? AND worker_id=?').run(shardStatus,input.error?String(input.error).slice(0,1000):null,JSON.stringify(input.progress||{}),Date.now(),input.runId,id);
  }
  return {ok:true,serverTime:Date.now()};
}
export function queueWorkerCommand(workerId,{runId=null,kind,payload={}}){
  const db=controlDb();if(!db.prepare('SELECT 1 FROM load_workers WHERE id=?').get(workerId))throw new Error('Worker does not exist');
  const revision=(db.prepare('SELECT COALESCE(MAX(revision),0)+1 AS revision FROM worker_commands WHERE worker_id=?').get(workerId).revision),id=randomUUID();
  db.prepare("INSERT INTO worker_commands(id,worker_id,run_id,revision,kind,payload,status,created_at) VALUES(?,?,?,?,?,?,'pending',?)").run(id,workerId,runId,revision,kind,JSON.stringify(payload),Date.now());
  return {id,runId,revision,kind,payload};
}
export function pendingWorkerCommands(id,token,after=0){authenticateWorker(id,token);return controlDb().prepare("SELECT id,run_id,revision,kind,payload,created_at FROM worker_commands WHERE worker_id=? AND revision>? AND status='pending' ORDER BY revision LIMIT 20").all(id,after).map(row=>({id:row.id,runId:row.run_id,revision:row.revision,kind:row.kind,payload:parsed(row.payload),createdAt:row.created_at}));}
export function acknowledgeWorkerCommand(id,token,commandId,input={}){
  authenticateWorker(id,token);const db=controlDb(),command=db.prepare("SELECT run_id,kind FROM worker_commands WHERE id=? AND worker_id=? AND status='pending'").get(commandId,id);
  if(!command)throw new HttpError(409,'Worker command is missing or already acknowledged');
  const failed=input.ok===false,result=db.prepare("UPDATE worker_commands SET status=?,acked_at=?,result=? WHERE id=? AND worker_id=? AND status='pending'").run(failed?'failed':'acknowledged',Date.now(),JSON.stringify(input.result||{}),commandId,id);
  if(!result.changes)throw new HttpError(409,'Worker command is missing or already acknowledged');
  if(failed&&command.run_id){
    const message=String(input.result?.error||`${command.kind} command failed`).slice(0,1000);
    db.prepare("UPDATE run_shards SET status='failed',error=?,updated_at=? WHERE run_id=? AND worker_id=? AND status NOT IN ('complete','failed')").run(message,Date.now(),command.run_id,id);
  }
  return {ok:true};
}
export function removeWorker(id){if(id==='local')throw new Error('The local worker cannot be removed');const db=controlDb();if(db.prepare("SELECT 1 FROM run_shards WHERE worker_id=? AND status IN ('reserved','preparing','ready','running','pausing','paused','draining','uploading')").get(id))throw new HttpError(409,'Drain the worker before removing it');db.prepare('DELETE FROM worker_commands WHERE worker_id=?').run(id);db.prepare('DELETE FROM load_workers WHERE id=?').run(id);return {deleted:true};}
