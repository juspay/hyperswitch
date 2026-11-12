import path from 'node:path';
import {controlDb} from './state.mjs';
import {listWorkers,queueWorkerCommand} from './worker-registry.mjs';
import {planWorkerShards,reserveRunShards} from './distributed.mjs';
import {createWorkerBundle,artifactPath} from './worker-artifacts.mjs';
import {mergeWorkerShards} from './merge-shards.mjs';

const parsed=value=>{try{return JSON.parse(value||'{}');}catch{return {};}};
const workerOfflineAfterMs=Number(process.env.LOADTEST_WORKER_OFFLINE_AFTER_MS||15000);
const workerPrepareTimeoutMs=Number(process.env.LOADTEST_WORKER_PREPARE_TIMEOUT_MS||120000);
export function aggregate(shards,{wallElapsedSeconds=null}={}){
  const progress=shards.map(shard=>parsed(shard.progress)),active=progress.map(p=>Number(p.control?.active_seconds??p.elapsed_seconds??0));
  const wall=progress.map(p=>Number(p.elapsed_seconds||0));
  const result={elapsed_seconds:wallElapsedSeconds??(wall.length?Math.min(...wall):0),iterations:0,successful_tps:0,http_attempts:0,http_errors:0,dropped:0,recorder:{enqueued:0,persisted:0},flows:{},connectors:{},control:{state:shards.every(s=>s.status==='paused')?'paused':shards.some(s=>['pausing','draining'].includes(s.status))?'pausing':shards.every(s=>s.status==='ready')?'idle':'running',active_seconds:active.length?Math.min(...active):0,in_flight:0}};
  for(const point of progress){result.iterations+=Number(point.iterations||0);result.successful_tps+=Number(point.successful_tps||0);result.http_attempts+=Number(point.http_attempts||0);result.http_errors+=Number(point.http_errors||0);result.dropped+=Number(point.dropped||0);result.recorder.enqueued+=Number(point.recorder?.enqueued||0);result.recorder.persisted+=Number(point.recorder?.persisted||0);result.control.in_flight+=Number(point.control?.in_flight||0);for(const [name,flow] of Object.entries(point.flows||{})){const target=result.flows[name]||={successes:0,failures:0,tps:0,p90_ms:null,p99_ms:null};target.successes+=Number(flow.successes||0);target.failures+=Number(flow.failures||0);target.tps+=Number(flow.tps||0);target.p90_ms=Math.max(target.p90_ms||0,Number(flow.p90_ms||0))||null;target.p99_ms=Math.max(target.p99_ms||0,Number(flow.p99_ms||0))||null;}}
  for(const point of progress)for(const [name,c] of Object.entries(point.connectors||{})){const target=result.connectors[name]||={assigned:0,confirmed:0,successes:0};for(const key of ['assigned','confirmed','successes'])target[key]+=Number(c[key]||0);}
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
  let phase='preparing',settled=false,timer,stopStarted=0,failure=null,wallStartedAt=null;
  const prepareStarted=Date.now(),stopTimeoutMs=Number(process.env.LOADTEST_WORKER_STOP_TIMEOUT_MS||60000);
  const shardsNow=()=>controlDb().prepare('SELECT s.*,w.last_seen FROM run_shards s JOIN load_workers w ON w.id=s.worker_id WHERE s.run_id=? ORDER BY s.id').all(runId);
  function stop(error=null){
    if(settled||phase==='stopping')return;
    failure=error;phase='stopping';stopStarted=Date.now();
    // A prepared-but-not-started worker must not receive a stale start after Stop.
    controlDb().prepare("UPDATE worker_commands SET status='cancelled',acked_at=? WHERE run_id=? AND status='pending'").run(stopStarted,runId);
    for(const shard of shardsNow())if(shard.status!=='complete')queueWorkerCommand(shard.worker_id,{runId,kind:'stop'});
    onEvent('draining_workers',error?'A worker failed · stopping all load workers':'Stopping all load workers');
  }
  let resolveDone;
  const done=new Promise(resolve=>{resolveDone=resolve;});
  async function finish(error,result){
    if(settled)return;settled=true;clearTimeout(timer);
    try{await onDone(error,result);}finally{resolveDone();}
  }
  async function poll(){
    if(settled)return;
    try{
      const db=controlDb(),shards=shardsNow(),now=Date.now();
      for(const shard of shards){
        if(!['complete','failed'].includes(shard.status)&&now-Number(shard.last_seen)>workerOfflineAfterMs){shard.status='failed';shard.error='Worker heartbeat timed out';db.prepare("UPDATE run_shards SET status='failed',error=?,updated_at=? WHERE id=?").run(shard.error,now,shard.id);}
        else if(phase==='preparing'&&!['ready','complete','failed'].includes(shard.status)&&now-prepareStarted>workerPrepareTimeoutMs){shard.status='failed';shard.error='Worker preparation timed out';db.prepare("UPDATE run_shards SET status='failed',error=?,updated_at=? WHERE id=?").run(shard.error,now,shard.id);}
      }
      // Counter updates from different workers must share the coordinator's
      // wall interval. A minimum worker clock can advance only microseconds
      // while another worker contributes a full second of completed flows.
      const reportedWall=Math.max(0,...shards.map(s=>Number(parsed(s.progress).elapsed_seconds||0)));
      if(wallStartedAt===null&&reportedWall>0)wallStartedAt=now-reportedWall*1000;
      const progress=aggregate(shards,{wallElapsedSeconds:wallStartedAt===null?0:Math.max(0,(now-wallStartedAt)/1000)});await onProgress(progress,shards);
      if(phase!=='stopping'&&shards.some(s=>s.status==='failed'))stop(new Error(shards.find(s=>s.error)?.error||'A load worker failed'));
      if(phase==='stopping'){
        const drained=shards.every(s=>{
          if(!['complete','failed'].includes(s.status))return false;
          // An offline worker enforces a local 30s control-plane lease. Give it
          // time to stop instead of treating a missed heartbeat as a stopped load.
          return s.error!=='Worker heartbeat timed out'||now-Number(s.last_seen)>=45000;
        });
        if(drained){await finish(failure,{...progress,exitCode:failure?1:0});return;}
        if(now-stopStarted>=stopTimeoutMs){
          db.prepare("UPDATE run_shards SET status='failed',error=COALESCE(error,'Worker stop was not verified'),updated_at=? WHERE run_id=? AND status NOT IN ('complete','failed')").run(now,runId);
          await finish(new Error('Worker stop was not verified before the drain timeout'),{...progress,exitCode:1});return;
        }
      }else if(phase==='preparing'&&shards.every(s=>s.status==='ready')){
        const startAt=Date.now()+3000;for(const shard of shards)queueWorkerCommand(shard.worker_id,{runId,kind:'start',payload:{startAt}});
        phase='running';onEvent('workers_ready',`${shards.length} workers ready · synchronized start scheduled`);
      }else if(phase==='running'&&shards.every(s=>s.status==='complete')){
        phase='merging';onEvent('merging','All workers complete · merging result shards');
        const artifacts=shards.map(shard=>({workerId:shard.worker_id,database:artifactPath(runId,shard.worker_id,'results.sqlite'),summary:artifactPath(runId,shard.worker_id,'summary.json')}));
        const merged=await mergeWorkerShards({runPath,runId,shards:artifacts,config}),exitCode=shards.some(s=>Number(parsed(s.progress).exitCode)===99)?99:0;
        await finish(null,{...progress,merged,exitCode});return;
      }
    }catch(error){
      if(settled)return;
      if(phase==='stopping'){failure||=error;if(Date.now()-stopStarted>=stopTimeoutMs){await finish(failure,{});return;}}
      else stop(error);
    }
    if(!settled)timer=setTimeout(poll,1000);
  }
  await poll();
  return {plan,done,async control(action,payload={}){
    if(settled||phase==='stopping')throw new Error('Run is stopping or finished');
    const shards=shardsNow(),effectiveAt=['pause','resume'].includes(action)?Date.now()+3000:Date.now();
    const commands=shards.map(shard=>queueWorkerCommand(shard.worker_id,{runId,kind:action,payload:{effectiveAt,...payload}}));onEvent(action,`${action[0].toUpperCase()+action.slice(1)} scheduled on ${shards.length} workers`);
    if(['gate','release','recover'].includes(action)){
      const deadline=Date.now()+15000;
      for(;;){
        const states=commands.map(command=>controlDb().prepare('SELECT status,result FROM worker_commands WHERE id=?').get(command.id));
        const failed=states.find(state=>state?.status==='failed');if(failed)throw new Error(parsed(failed.result).error||`${action} failed on a load worker`);
        if(states.every(state=>state?.status==='acknowledged'))break;
        if(Date.now()>=deadline||phase==='stopping')throw new Error(`Workers did not acknowledge ${action} in time`);
        await new Promise(resolve=>setTimeout(resolve,100));
      }
    }
  },cancel(){stop();},get phase(){return phase;}};
}
