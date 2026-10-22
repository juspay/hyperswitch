import path from 'node:path';
import {controlDb} from './state.mjs';
import {listWorkers,queueWorkerCommand} from './worker-registry.mjs';
import {planWorkerShards,reserveRunShards} from './distributed.mjs';
import {createWorkerBundle,artifactPath} from './worker-artifacts.mjs';
import {mergeWorkerShards} from './merge-shards.mjs';

const parsed=value=>{try{return JSON.parse(value||'{}');}catch{return {};}};
const workerOfflineAfterMs=Number(process.env.LOADTEST_WORKER_OFFLINE_AFTER_MS||15000);
const workerPrepareTimeoutMs=Number(process.env.LOADTEST_WORKER_PREPARE_TIMEOUT_MS||120000);
export function aggregate(shards){
  const progress=shards.map(shard=>parsed(shard.progress)),active=progress.map(p=>Number(p.control?.active_seconds??p.elapsed_seconds??0));
  const result={elapsed_seconds:active.length?Math.min(...active):0,iterations:0,successful_tps:0,http_attempts:0,http_errors:0,dropped:0,recorder:{enqueued:0,persisted:0},flows:{},control:{state:shards.every(s=>s.status==='paused')?'paused':shards.some(s=>['pausing','draining'].includes(s.status))?'pausing':shards.every(s=>s.status==='ready')?'idle':'running',active_seconds:active.length?Math.min(...active):0,in_flight:0}};
  for(const point of progress){result.iterations+=Number(point.iterations||0);result.successful_tps+=Number(point.successful_tps||0);result.http_attempts+=Number(point.http_attempts||0);result.http_errors+=Number(point.http_errors||0);result.dropped+=Number(point.dropped||0);result.recorder.enqueued+=Number(point.recorder?.enqueued||0);result.recorder.persisted+=Number(point.recorder?.persisted||0);result.control.in_flight+=Number(point.control?.in_flight||0);for(const [name,flow] of Object.entries(point.flows||{})){const target=result.flows[name]||={successes:0,failures:0,tps:0,p90_ms:null,p99_ms:null};target.successes+=Number(flow.successes||0);target.failures+=Number(flow.failures||0);target.tps+=Number(flow.tps||0);target.p90_ms=Math.max(target.p90_ms||0,Number(flow.p90_ms||0))||null;target.p99_ms=Math.max(target.p99_ms||0,Number(flow.p99_ms||0))||null;}}
  if(progress.length&&progress.every(p=>Number.isFinite(p.control?.started_flows))){
    result.control.started_flows=progress.reduce((sum,p)=>sum+p.control.started_flows,0);
    result.control.flow_starts={};for(const p of progress)for(const [name,count] of Object.entries(p.control.flow_starts||{}))result.control.flow_starts[name]=(result.control.flow_starts[name]||0)+count;
  }
  if(progress.length&&progress.every(p=>Number.isFinite(p.confirm_http_attempts)&&Number.isFinite(p.confirm_http_errors))){result.confirm_http_attempts=progress.reduce((sum,p)=>sum+p.confirm_http_attempts,0);result.confirm_http_errors=progress.reduce((sum,p)=>sum+p.confirm_http_errors,0);}
  const percentile=values=>{const finite=values.filter(v=>Number.isFinite(v)&&v>=0).sort((a,b)=>a-b);return finite.length?finite[Math.floor((finite.length-1)*.99)]:null;};
  result.server_p99_ms=percentile(progress.flatMap(p=>p.server_samples_ms||[]));
  for(const [name,flow] of Object.entries(result.flows))flow.server_p99_ms=percentile(progress.flatMap(p=>p.flows?.[name]?.server_samples_ms||[]));
  return result;
}
export async function launchDistributedRun({runId,runPath,config,fixturePath,onProgress=(..._args)=>{},onEvent=(..._args)=>{},onDone=(..._args)=>{}}){
  const requiredVus=Math.ceil(Number(config.load.max_vus_multiplier||10)*(config.load.total_rps??Math.max(...config.load.stages.map(stage=>stage.target_rps)))),workers=listWorkers().filter(worker=>worker.id!=='local'),totalTps=config.load.total_rps??Math.max(...config.load.stages.map(stage=>stage.target_rps));
  const plan=planWorkerShards(totalTps,requiredVus,workers),bundles=[];
  for(const shard of plan)bundles.push(await createWorkerBundle({runId,config,fixturePath,workerId:shard.workerId,segmentStart:shard.segmentStart,segmentEnd:shard.segmentEnd}));
  reserveRunShards(runId,plan,bundles.map(bundle=>({bundleId:bundle.id,sha256:bundle.sha256,totalTps})));
  let phase='preparing',settled=false,timer,startAt;const prepareStarted=Date.now();
  const done=new Promise(resolve=>{const poll=async()=>{
    if(settled)return;try{
      const db=controlDb(),shards=db.prepare('SELECT s.*,w.last_seen FROM run_shards s JOIN load_workers w ON w.id=s.worker_id WHERE s.run_id=? ORDER BY s.id').all(runId),now=Date.now();
      for(const shard of shards){
        if(!['complete','failed'].includes(shard.status)&&now-Number(shard.last_seen)>workerOfflineAfterMs){shard.status='failed';shard.error='Worker heartbeat timed out';db.prepare("UPDATE run_shards SET status='failed',error=?,updated_at=? WHERE id=?").run(shard.error,now,shard.id);}
        else if(phase==='preparing'&&!['ready','complete','failed'].includes(shard.status)&&now-prepareStarted>workerPrepareTimeoutMs){shard.status='failed';shard.error='Worker preparation timed out';db.prepare("UPDATE run_shards SET status='failed',error=?,updated_at=? WHERE id=?").run(shard.error,now,shard.id);}
      }
      const progress=aggregate(shards);onProgress(progress,shards);
      if(shards.some(shard=>shard.status==='failed')){settled=true;onDone(new Error(shards.find(shard=>shard.error)?.error||'A load worker failed'),progress);resolve();return;}
      if(phase==='preparing'&&shards.every(shard=>shard.status==='ready')){startAt=Date.now()+3000;for(const shard of shards)queueWorkerCommand(shard.worker_id,{runId,kind:'start',payload:{startAt}});phase='running';onEvent('workers_ready',`${shards.length} workers ready · synchronized start scheduled`);}
      if(phase==='running'&&shards.every(shard=>shard.status==='complete')){
        phase='merging';onEvent('merging','All workers complete · merging result shards');const artifacts=shards.map(shard=>({workerId:shard.worker_id,database:artifactPath(runId,shard.worker_id,'results.sqlite'),summary:artifactPath(runId,shard.worker_id,'summary.json')}));
        const merged=await mergeWorkerShards({runPath,runId,shards:artifacts,config}),exitCode=shards.some(shard=>Number(parsed(shard.progress).exitCode)===99)?99:0;settled=true;await onDone(null,{...progress,merged,exitCode});resolve();return;
      }
    }catch(error){settled=true;await onDone(error,{});resolve();return;}timer=setTimeout(poll,1000);};poll();});
  return {plan,done,async control(action,payload={}){
    const shards=controlDb().prepare('SELECT worker_id FROM run_shards WHERE run_id=?').all(runId),effectiveAt=['pause','resume'].includes(action)?Date.now()+3000:Date.now();
    const commands=shards.map(shard=>queueWorkerCommand(shard.worker_id,{runId,kind:action,payload:{effectiveAt,...payload}}));onEvent(action,`${action[0].toUpperCase()+action.slice(1)} scheduled on ${shards.length} workers`);
    if(['gate','release','recover'].includes(action)){
      const deadline=Date.now()+15000;
      for(;;){
        const states=commands.map(command=>controlDb().prepare('SELECT status,result FROM worker_commands WHERE id=?').get(command.id));
        const failed=states.find(state=>state?.status==='failed');if(failed)throw new Error(parsed(failed.result).error||`${action} failed on a load worker`);
        if(states.every(state=>state?.status==='acknowledged'))break;
        if(Date.now()>=deadline)throw new Error(`Workers did not acknowledge ${action} in time`);
        await new Promise(resolve=>setTimeout(resolve,100));
      }
    }
  },cancel(){if(settled)return;for(const shard of controlDb().prepare('SELECT worker_id FROM run_shards WHERE run_id=?').all(runId))queueWorkerCommand(shard.worker_id,{runId,kind:'stop'});phase='stopping';},get phase(){return phase;}};
}
