import {DatabaseSync} from 'node:sqlite';
import {copyFile,readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';

const quote=value=>`'${String(value).replaceAll("'","''")}'`;
function quantile(db,runId,flowName,percentile,column='latency_ms'){
  const filter=flowName?' AND flow_id LIKE ?':'',args=flowName?[runId,`%:${flowName}:%`]:[runId];
  const count=Number(db.prepare(`SELECT count(*) n FROM request_events WHERE run_id=? AND role='measured' AND operation='payment_confirm' AND ${column} IS NOT NULL${filter}`).get(...args).n);if(!count)return null;
  const offset=Math.max(0,Math.ceil(count*percentile)-1);return Number(db.prepare(`SELECT ${column} value FROM request_events WHERE run_id=? AND role='measured' AND operation='payment_confirm' AND ${column} IS NOT NULL${filter} ORDER BY ${column} LIMIT 1 OFFSET ?`).get(...args,offset).value);
}
function mergeMetric(target,metric){
  const output=target||{type:metric.type,contains:metric.contains,values:{},...(metric.thresholds?{thresholds:structuredClone(metric.thresholds)}:{})};for(const [key,value] of Object.entries(metric.values||{})){
    if(['count','passes','fails','value'].includes(key))output.values[key]=Number(output.values[key]||0)+Number(value||0);
  }
  for(const [expression,result] of Object.entries(metric.thresholds||{})){const previous=output.thresholds?.[expression];output.thresholds||={};output.thresholds[expression]={...previous,...result,ok:(previous?.ok??true)&&result.ok!==false};}
  return output;
}
export async function mergeWorkerShards({runPath,runId,shards,config}){
  if(!shards.length)throw new Error('No worker result shards were uploaded');const canonical=path.join(runPath,'results.sqlite');await copyFile(shards[0].database,canonical);
  const db=new DatabaseSync(canonical),base=db.prepare('SELECT * FROM runs LIMIT 1').get();if(!base){db.close();throw new Error('First worker shard has no run record');}
  if(!db.prepare('PRAGMA table_info(request_events)').all().some(c=>c.name==='server_latency_ms'))db.exec('ALTER TABLE request_events ADD COLUMN server_latency_ms REAL');
  const original=base.run_id;for(const table of ['request_events','customers','payments'])db.prepare(`UPDATE ${table} SET run_id=? WHERE run_id=?`).run(runId,original);db.prepare('UPDATE runs SET run_id=? WHERE run_id=?').run(runId,original);
  let enqueued=Number(base.enqueued),persisted=Number(base.persisted),started=Number(base.started_at),ended=Number(base.ended_at||0),complete=base.recording_state==='complete',error=base.error||null;
  for(let index=1;index<shards.length;index++){
    const alias=`shard${index}`,file=shards[index].database;db.exec(`ATTACH DATABASE ${quote(file)} AS ${alias}`);const run=db.prepare(`SELECT * FROM ${alias}.runs LIMIT 1`).get();if(!run){db.exec(`DETACH DATABASE ${alias}`);throw new Error(`Worker shard ${index+1} has no run record`);}
    const offset=Number(db.prepare('SELECT COALESCE(MAX(event_id),0) n FROM request_events').get().n),source=quote(run.run_id),target=quote(runId);
    const serverColumn=db.prepare(`PRAGMA ${alias}.table_info(request_events)`).all().some(c=>c.name==='server_latency_ms')?'server_latency_ms':'NULL';
    db.exec('BEGIN');try{
      db.exec(`INSERT INTO request_events SELECT event_id+${offset},${target},attempt_id,flow_id,merchant_id,operation,role,observed_at,method,url,customer_id,merchant_reference_id,payment_id,amount,status,api_status_code,request_id,latency_ms,message,error_body,entity_type,${serverColumn} FROM ${alias}.request_events WHERE run_id=${source}`);
      db.exec(`INSERT OR IGNORE INTO customers SELECT ${target},customer_id,merchant_id,merchant_reference_id,api_status_code,request_id,message,observed_at,response_event_id+${offset} FROM ${alias}.customers WHERE run_id=${source}`);
      db.exec(`INSERT OR IGNORE INTO payments SELECT ${target},payment_id,merchant_id,customer_id,amount,status,role,api_status_code,request_id,message,observed_at,response_event_id+${offset} FROM ${alias}.payments WHERE run_id=${source}`);
      db.exec('COMMIT');
    }catch(e){db.exec('ROLLBACK');throw e;}finally{db.exec(`DETACH DATABASE ${alias}`);}
    enqueued+=Number(run.enqueued);persisted+=Number(run.persisted);started=Math.min(started,Number(run.started_at));ended=Math.max(ended,Number(run.ended_at||0));complete&&=run.recording_state==='complete';error||=run.error;
  }
  const metadata={...JSON.parse(String(base.metadata||'{}')),distributed:{workers:shards.map(shard=>shard.workerId)}};
  db.prepare('UPDATE runs SET recording_state=?,started_at=?,ended_at=?,enqueued=?,persisted=?,error=?,metadata=? WHERE run_id=?').run(complete&&enqueued===persisted?'complete':'incomplete',started,ended||null,enqueued,persisted,error,JSON.stringify(metadata),runId);
  const aggregate={metrics:{}};for(const shard of shards){let summary={};try{summary=JSON.parse(await readFile(shard.summary,'utf8'));}catch{}for(const [name,metric] of Object.entries(summary.metrics||{}))aggregate.metrics[name]=mergeMetric(aggregate.metrics[name],metric);}
  for(const metric of Object.values(aggregate.metrics)){const values=metric.values;if(values.passes!=null||values.fails!=null)values.rate=Number(values.passes||0)/(Number(values.passes||0)+Number(values.fails||0)||1);}
  const addTrend=(name,flow=null,column='latency_ms')=>{const p90=quantile(db,runId,flow,.9,column),p99=quantile(db,runId,flow,.99,column);if(p99!=null)aggregate.metrics[name]={type:'trend',contains:'time',values:{'p(90)':p90,'p(99)':p99}};};
  addTrend('payment_confirm_latency_ms');for(const scenario of config.scenarios||[])addTrend(`payment_confirm_ms_${scenario.name}`,scenario.name);
  addTrend('payment_confirm_server_latency_ms',null,'server_latency_ms');for(const scenario of config.scenarios||[])addTrend(`payment_confirm_server_ms_${scenario.name}`,scenario.name,'server_latency_ms');
  // Successful TPS is additive across workers. k6 exposes these as gauges.
  for(const scenario of config.scenarios||[]){const metric=aggregate.metrics[`achieved_tps_${scenario.name}`];if(metric)metric.values.value=Number(metric.values.value||0);}
  await writeFile(path.join(runPath,'summary.json'),JSON.stringify(aggregate,null,2)+'\n',{mode:0o600});db.close();return {database:canonical,summary:path.join(runPath,'summary.json'),workers:shards.length,enqueued,persisted};
}
