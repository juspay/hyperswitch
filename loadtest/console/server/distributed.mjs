import {controlDb} from './state.mjs';
import {queueWorkerCommand} from './worker-registry.mjs';

export function planWorkerShards(totalTps,requiredVus,workers,{now=Date.now()}={}){
  totalTps=Number(totalTps);requiredVus=Number(requiredVus);
  if(!Number.isFinite(totalTps)||totalTps<=0||!Number.isFinite(requiredVus)||requiredVus<=0)throw new Error('Load capacity must be positive');
  const vusPerTps=requiredVus/totalTps,available=workers.filter(worker=>worker.status==='idle'&&now-Number(worker.lastSeen??worker.last_seen??0)<=15000)
    .map(worker=>{const capacityTps=Number(worker.capacityTps??worker.capacity_tps),capacityVus=Number(worker.capacityVus??worker.capacity_vus);return {...worker,capacityTps,capacityVus,effectiveTps:Math.floor(Math.min(capacityTps,capacityVus/vusPerTps))};})
    .filter(worker=>worker.effectiveTps>0)
    .sort((a,b)=>b.effectiveTps-a.effectiveTps||String(a.id).localeCompare(String(b.id)));
  const selected=[];let tps=0,vus=0;
  for(const worker of available){selected.push(worker);tps+=worker.effectiveTps;vus+=worker.capacityVus;if(tps>=totalTps&&vus>=requiredVus)break;}
  if(tps<totalTps||vus<requiredVus)throw new Error(`Insufficient worker capacity: ${tps}/${totalTps} TPS and ${vus}/${requiredVus} VUs available`);
  // Divide by TPS capacity while capping the last worker so the sum is exact.
  let remaining=totalTps,start=0;
  return selected.map((worker,index)=>{
    const workersLeft=selected.length-index,share=index===selected.length-1?remaining:Math.min(worker.effectiveTps,Math.max(1,Math.round(remaining/workersLeft)));
    const end=Math.min(1,start+share/totalTps),shard={workerId:worker.id,workerName:worker.name,offeredTps:share,requiredVus:Math.ceil(requiredVus*share/totalTps),segmentStart:start,segmentEnd:index===selected.length-1?1:end};
    remaining-=share;start=shard.segmentEnd;return shard;
  });
}

export function reserveRunShards(runId,plan,payload={}){
  const db=controlDb(),now=Date.now();
  const insert=db.prepare("INSERT INTO run_shards(id,run_id,worker_id,status,offered_tps,segment_start,segment_end,updated_at) VALUES(?,?,?,'reserved',?,?,?,?)");
  db.exec('BEGIN IMMEDIATE');
  try{const result=plan.map((shard,index)=>{
      const id=`${runId}:${index+1}`;insert.run(id,runId,shard.workerId,shard.offeredTps,shard.segmentStart,shard.segmentEnd,now);
      const common=Array.isArray(payload)?payload[index]:payload;
      const command=queueWorkerCommand(shard.workerId,{runId,kind:'prepare',payload:{...common,shardId:id,offeredTps:shard.offeredTps,requiredVus:shard.requiredVus,segment:{start:shard.segmentStart,end:shard.segmentEnd}}});
      return {...shard,id,command};
    });db.exec('COMMIT');return result;
  }catch(error){db.exec('ROLLBACK');throw error;}
}
