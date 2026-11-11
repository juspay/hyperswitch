import {DatabaseSync} from 'node:sqlite';
import {readFile,writeFile,rename} from 'node:fs/promises';
import {createHash,randomUUID} from 'node:crypto';
import path from 'node:path';
import {loadTimeline,loadShape} from '../src/load-shape.mjs';
import {defaultResultsLimits,validateResultsLimits} from '../src/results-limits.mjs';
import {executionClock,stageIntervals} from './execution-clock.mjs';
import {latencyPercentile,settledLatencyWindows,containedLatencySample,mergeLatencyWindows} from './successful-latency.mjs';
import {matchedLatencyDrift} from './latency-drift.mjs';
import {testCasePlan} from '../src/result-test-cases.mjs';

export const scenarioChecksVersion=16;
const terminal=new Set(['complete','failed','cancelled','interrupted']);
const successful=row=>row.api_status_code>=200&&row.api_status_code<300&&['succeeded','requires_capture','processing'].includes(row.status);
const inWindow=(at,windows)=>windows.some(w=>at>=w.from&&at<w.to);
const overlaps=(row,windows)=>windows.some(w=>row.observed_at>=w.from&&row.observed_at-Math.max(0,row.latency_ms||0)<w.to);
const seconds=windows=>windows.reduce((sum,w)=>sum+(w.to-w.from)/1000,0);
function clipInterval(window,from,to){
  const active=at=>window.activeFrom+(at-window.from)/(window.to-window.from)*(window.activeTo-window.activeFrom);
  return {...window,from,to,activeFrom:active(from),activeTo:active(to)};
}
function withoutOutages(windows,outages){
  return outages.reduce((parts,outage)=>parts.flatMap(w=>{
    if(outage.to<=w.from||outage.from>=w.to)return [w];
    const result=[];if(outage.from>w.from)result.push(clipInterval(w,w.from,outage.from));if(outage.to<w.to)result.push(clipInterval(w,outage.to,w.to));return result;
  }),windows);
}
function offeredWork(stage,windows){
  const rate=a=>stage.from+(stage.target-stage.from)*(a-stage.start)/(stage.end-stage.start);
  return windows.reduce((sum,w)=>sum+(rate(w.activeFrom)+rate(w.activeTo))/2*(w.activeTo-w.activeFrom),0);
}
const percentile=(values,p)=>latencyPercentile([...values],p);
const readJson=async file=>{try{return JSON.parse(await readFile(file,'utf8'));}catch(e){if(e.code==='ENOENT')return null;throw e;}};
async function lines(file){try{return (await readFile(file,'utf8')).split('\n').filter(Boolean).flatMap(line=>{try{return [JSON.parse(line)];}catch{return [];}});}catch(e){if(e.code==='ENOENT')return [];throw e;}}

// Fault occurrences are bound to the scheduler's stable stage index, rather
// than to a service label (the same DB can fail over more than once).
export function faultOccurrences(events,stages,end){
  const result=[];let stageIndex=null,current=null;
  for(const event of [...events].sort((a,b)=>a.at-b.at)){
    if(event.kind==='scheduled_fault'){stageIndex=Number(event.message?.match(/scheduled step (\d+)/)?.[1])-1;continue;}
    if(event.kind==='fault_requested'||(event.kind==='fault_started'&&!current)){
      const stage=stages.find(s=>s.index===stageIndex&&s.fault)||stages.filter(s=>s.fault)[result.length];
      if(!stage)continue;
      current={stageIndex:stage.index,from:event.at,to:end,restoreAt:null,recoveredAt:null,unavailable:false,simulated:event.message?.startsWith('SIMULATED')||false};result.push(current);
    }
    if(!current)continue;
    if(event.kind==='fault_started'){current.startedAt??=event.at;current.simulated||=event.message?.startsWith('SIMULATED')||false;}
    if(event.kind==='fault_restore')current.restoreAt=event.at;
    if(event.kind==='database_unavailable'){current.unavailable=true;current.unavailableAt??=event.at;}
    if(event.kind==='database_available'&&current.unavailable){current.recoveredAt??=event.at;current.to=event.at;}
    if(event.kind==='fault_restored'){
      current.recoveredAt??=event.at;current.to=current.recoveredAt;current=null;
    }
    if(event.kind==='fault_skipped'){current.skipped=true;current.to=event.at;current=null;}
  }
  return result;
}

function bucket(){return {requests:0,errors:0,paymentRequests:0,paymentErrors:0,failedCreates:0,completed:0,successes:0,latencies:[],driftSamples:[]};}
function check(id,label,actual,limit,unit,{minimum=false,state='complete',record=false}={}){
  const outcome=state==='not_run'?'not_run':state==='in_progress'?'in_progress':state==='partial'?'not_measured':record||limit==null?'recorded':actual==null?'not_measured':(minimum?actual>=limit:actual<=limit)?'pass':'fail';
  return {id,label,actual,limit:record?null:limit,unit,comparison:minimum?'≥':'≤',outcome};
}
const groupVerdict=checks=>checks.some(c=>c.outcome==='fail')?'fail':checks.some(c=>c.outcome==='in_progress')?'in_progress':checks.every(c=>c.outcome==='not_run')?'not_run':checks.some(c=>['not_measured','not_run'].includes(c.outcome))?'incomplete':checks.some(c=>c.outcome==='pass')?'pass':'recorded';

export function evaluateScenarioChecks({config,recording,events=[],samples=[],requests=[],flows=[],reconciliation=null,jobStatus=null,now=Date.now(),measurementsAvailable=true,control=null}){
  const limits=config.resultsLimits?validateResultsLimits(config.resultsLimits):{...defaultResultsLimits};
  // Only limits stored with the run can produce historical pass/fail claims.
  const saved=Boolean(config.resultsLimits);
  const legacy=config.acceptance||{};
  const throughputLimit=saved?limits.minimumThroughputPercent:legacy.minimumSuccessRatio==null?null:legacy.minimumSuccessRatio*100;
  const errorLimit=saved?limits.maximumHttpErrorPercent:legacy.maximumErrorPercent??null;
  const finished=terminal.has(jobStatus)||Boolean(recording.ended_at&&!jobStatus);
  const end=Number(recording.ended_at)||now,clock=executionClock(samples,{...recording,ended_at:end});
  const stages=loadTimeline(config.load,config.faultPlan),faults=faultOccurrences(events,stages,end),outages=faults.filter(f=>!f.skipped).map(f=>({from:f.from,to:f.to}));
  const groups=[];
  for(const stage of stages){
    const execution=stageIntervals(clock,stage),windows=withoutOutages(execution,outages),reached=clock?.active??0;
    const state=!clock?(finished?'partial':'in_progress'):!execution.length?'not_run':reached>=stage.end-0.05?'complete':finished?'partial':'in_progress';
    const kind=stage.from===stage.target?'hold':'ramp';
    if(!stage.fault)groups.push({id:`stage-${stage.index}`,stageIndex:stage.index,kind,title:`${stage.index+1}. ${kind==='hold'?'Hold':'Ramp'} · ${stage.from===stage.target?stage.target:`${stage.from} → ${stage.target}`} TPS`,configuredSeconds:stage.end-stage.start,state,windows,stage,bucket:bucket()});
    const fault=faults.find(f=>f.stageIndex===stage.index);
    if(stage.fault){
      const windows=fault&&!fault.skipped?[{from:fault.from,to:fault.to}]:[];
      groups.push({id:`fault-${stage.index}`,stageIndex:stage.index,kind:'fault',target:stage.fault.target,title:`${stage.index+1}. ${stage.fault.target} · ${stage.fault.action==='failover'?'Failover':'Outage'} · ${stage.from===stage.target?stage.target:`${stage.from} → ${stage.target}`} TPS`,state:!fault?'not_run':fault.skipped?'partial':fault.recoveredAt?'complete':finished?'partial':'in_progress',windows,fault,bucket:bucket()});
      {
        const recoveredAt=fault?.recoveredAt,until=(recoveredAt??end)+limits.recoveryWindowSeconds*1000;
        const next=faults.find(f=>recoveredAt!=null&&f.from>recoveredAt);
        const to=Math.min(until,end,next?.from??Infinity);
        groups.push({id:`recovery-${stage.index}`,stageIndex:stage.index,kind:'recovery',title:`After ${stage.fault.target} recovery · first ${limits.recoveryWindowSeconds}s`,state:recoveredAt==null?'not_run':to>=until?'complete':finished?'partial':'in_progress',windows:recoveredAt!=null&&to>recoveredAt?[{from:recoveredAt,to}]:[],bucket:bucket()});
      }
    }
  }
  const recoveryWindows=groups.filter(g=>g.kind==='recovery').flatMap(g=>g.windows);
  for(const group of groups)if(['hold','ramp'].includes(group.kind))group.windows=withoutOutages(group.windows,recoveryWindows);
  const normalWindows=groups.filter(g=>['hold','ramp'].includes(g.kind)).flatMap(g=>g.windows);
  const steadyIds=new Set(testCasePlan(config,groups).find(p=>p.id==='TC-02')?.groupIds||[]);
  const latencyWindows=settledLatencyWindows({normalWindows,faults,startedAt:recording.started_at,end,settlingSeconds:limits.recoveryWindowSeconds});
  for(const group of groups)group.latencyWindows=group.kind==='fault'?latencyWindows.outage.filter(w=>w.stageIndex===group.stageIndex):group.kind==='recovery'?latencyWindows.recovery.filter(w=>w.stageIndex===group.stageIndex):mergeLatencyWindows(group.windows.flatMap(w=>[...latencyWindows.normal,...latencyWindows.recovery].flatMap(s=>{
   const from=Math.max(w.from,s.from),to=Math.min(w.to,s.to);return to>from?[{from,to}]:[];
  })));
  const steadyLatencyWindows=mergeLatencyWindows(groups.filter(g=>steadyIds.has(g.id)).flatMap(g=>g.latencyWindows.flatMap(w=>latencyWindows.normal.flatMap(n=>{const from=Math.max(w.from,n.from),to=Math.min(w.to,n.to);return to>from?[{from,to}]:[];}))));

  const steadyDrift=[],steadyLatencies=[];
  const stateBuckets={normal:bucket(),outage:bucket(),recovery:bucket()};
  const stateFor=(at,faultOverlap=false,recoveryOverlap=false)=>faultOverlap||inWindow(at,outages)?'outage':recoveryOverlap||inWindow(at,recoveryWindows)?'recovery':inWindow(at,normalWindows)?'normal':null;
  // Request intervals overlapping an outage never pollute normal-stage limits,
  // including a timeout returned after recovery. All raw failures remain in the
  // fault group and run-wide recording; no Performance display filter is used.
  const confirmations={attempts:0,successful:0,outageAttempts:0,outageFailures:0};
  for(const row of requests){
    const faultOverlap=overlaps(row,outages),recoveryOverlap=overlaps(row,recoveryWindows);
    const paymentRequest=['payment_create','payment_confirm'].includes(row.operation),httpFailed=row.api_status_code===0||row.api_status_code>=400;
    if(row.operation==='payment_confirm'){
      if(faultOverlap||inWindow(row.observed_at,outages)){confirmations.outageAttempts++;if(!successful(row))confirmations.outageFailures++;}
      else {confirmations.attempts++;if(successful(row))confirmations.successful++;}
    }
    const state=stateFor(row.observed_at,faultOverlap,recoveryOverlap);
    if(state){const b=stateBuckets[state];b.requests++;if(httpFailed)b.errors++;if(paymentRequest){b.paymentRequests++;if(httpFailed)b.paymentErrors++;}
    }
    const hasLatency=row.operation==='payment_confirm'&&successful(row)&&Number.isFinite(row.server_latency_ms)&&row.server_latency_ms>=0;
    if(hasLatency&&containedLatencySample(row,steadyLatencyWindows)){
      steadyLatencies.push(row.server_latency_ms);
      steadyDrift.push(row);
    }
    if(hasLatency)for(const [phase,windows] of Object.entries(latencyWindows))if(containedLatencySample(row,windows))stateBuckets[phase].latencies.push(row.server_latency_ms);
    for(const group of groups){
      // Recovery latency uses the settled post-recovery interval; its other
      // observations retain their original first-window measurements.
      if(hasLatency&&containedLatencySample(row,group.latencyWindows))group.bucket.latencies.push(row.server_latency_ms);
      if(!inWindow(row.observed_at,group.windows)&&!(group.kind==='fault'&&overlaps(row,group.windows)))continue;
      if(group.kind!=='fault'&&faultOverlap)continue;
      if(['hold','ramp'].includes(group.kind)&&recoveryOverlap)continue;
      const b=group.bucket;if(paymentRequest){b.paymentRequests++;if(httpFailed)b.paymentErrors++;}b.requests++;if(row.api_status_code===0||row.api_status_code>=400){b.errors++;if(row.operation==='payment_create')b.failedCreates++;}
      if(row.operation==='payment_confirm'&&successful(row)&&Number.isFinite(row.server_latency_ms)&&row.server_latency_ms>=0){
        if(group.kind==='hold'&&clock&&containedLatencySample(row,group.latencyWindows)){
          b.driftSamples.push(row);
        }
      }
    }
  }
  let runFlows=0,successfulFlows=0;
  for(const row of flows){
    runFlows++;if(row.success)successfulFlows++;
    const interval={observed_at:row.at,latency_ms:Math.max(0,row.at-(row.startedAt??row.at))};
    const affected=overlaps(interval,outages)||overlaps(interval,recoveryWindows);
    const state=stateFor(row.at,overlaps(interval,outages),overlaps(interval,recoveryWindows));if(state){stateBuckets[state].completed++;if(row.success)stateBuckets[state].successes++;}
    for(const group of groups){
      if(!inWindow(row.at,group.windows))continue;
      if(['hold','ramp'].includes(group.kind)&&affected)continue;
      group.bucket.completed++;if(row.success)group.bucket.successes++;
    }
  }
  const steadyBucket=bucket();let steadySeconds=0,steadyExpected=0;
  for(const group of groups){
    const {bucket:b,state,kind}=group,opts={state:!measurementsAvailable&&!finished&&state!=='not_run'?'in_progress':state};
    const duration=seconds(group.windows);
    const expected=group.stage?offeredWork(group.stage,group.windows):null;
    if(kind==='hold'&&steadyIds.has(group.id)){
      steadySeconds+=duration;steadyExpected+=expected;
      for(const key of ['requests','errors','paymentRequests','paymentErrors','completed','successes'])steadyBucket[key]+=b[key];
      for(const value of b.latencies)steadyBucket.latencies.push(value);
    }
    const normal=kind==='hold'||kind==='ramp';
    const tps=measurementsAvailable&&duration>0?b.successes/duration:null;
    const throughput=measurementsAvailable&&normal&&expected>0?100*b.successes/expected:null;
    const recoveryExpected=kind==='recovery'?stages.reduce((sum,s)=>sum+offeredWork(s,stageIntervals(clock,s).flatMap(w=>group.windows.flatMap(r=>{
      const from=Math.max(w.from,r.from),to=Math.min(w.to,r.to);return to>from?[clipInterval(w,from,to)]:[];
    }))),0):null;
    const rows=[];
    if(normal)rows.push(check('throughput','Window throughput vs offered',throughput,throughputLimit,'%',{...opts,minimum:true}));
    else if(kind==='recovery')rows.push(check('throughput','Throughput restored',recoveryExpected>0&&tps!=null?100*b.successes/recoveryExpected:null,throughputLimit,'%',{...opts,minimum:true}));
    rows.push(check('tps','Successful TPS',tps,null,'TPS',{...opts,record:true}));
    rows.push(check('errors','HTTP errors',b.paymentRequests?100*b.paymentErrors/b.paymentRequests:null,normal?errorLimit:null,'%',opts));
    if(kind==='fault'){
      rows.push(check('failed','Failed requests',measurementsAvailable?b.errors:null,null,'requests',{...opts,record:true}));
      rows.push(check('failed-creation','Failed payment creation during outage',measurementsAvailable?b.failedCreates:null,null,'requests',{...opts,record:true}));
    }
    rows.push(check('completed','Flows completed in window',measurementsAvailable&&group.windows.length?b.successes:null,null,'flows',{...opts,record:true}));
    for(const p of [.5,.95,.99])rows.push(check(`p${p*100}`,`Server-side latency · p${p*100}`,percentile(b.latencies,p),saved&&p===.99&&kind!=='fault'?(kind==='recovery'||(group.latencyWindows.length&&group.latencyWindows.every(w=>latencyWindows.recovery.some(r=>w.from>=r.from&&w.to<=r.to)))?limits.maximumRecoveryP99Ms:kind==='ramp'?limits.maximumRampP99Ms:limits.maximumHoldP99Ms):null,'ms',opts));
    if(kind==='hold'){
      const matched=matchedLatencyDrift(b.driftSamples,group.latencyWindows);
      rows.push({...check('drift','Latency drift',matched.percent,saved?limits.maximumLatencyDriftPercent:null,'%',opts),matched});
    }
    if(kind==='fault'){
      const f=group.fault,recoveryStart=f?.unavailableAt??f?.restoreAt,recovered=f?.recoveredAt!=null&&recoveryStart!=null;
      const elapsed=recoveryStart!=null?(end-recoveryStart)/1000:null,expired=!recovered&&elapsed>limits.maximumRecoverySeconds;
      rows.unshift({...check('recovery',f?.unavailableAt!=null?'Database availability restored after outage':'Recovery after restoration starts',recovered?Math.max(0,(f.recoveredAt-recoveryStart)/1000):expired?elapsed:null,saved?limits.maximumRecoverySeconds:null,'s',{...opts,state:expired?'complete':state,record:f?.simulated}),lowerBound:expired});
      if(f?.simulated)group.title=`Simulation · ${group.title}`;
    }
    group.checks=rows;group.verdict=groupVerdict(rows);group.durationSeconds=duration;group.successfulFlows=b.successes;group.latencySamples=b.latencies.length;
    delete group.bucket;delete group.stage;delete group.fault;delete group.windows;delete group.latencyWindows;
  }
  if(stages.some(s=>s.fault)){
    const verified=reconciliation?.recordingState==='complete'&&Number.isFinite(reconciliation.missing)&&reconciliation.total===reconciliation.expected;
    const checks=[check('lost','Acknowledged payments lost',verified?reconciliation.missing:null,saved?limits.maximumLostPayments:null,'payments',{state:finished?'complete':'in_progress'})];
    groups.push({id:'integrity',kind:'integrity',title:'Data integrity',checks,verdict:groupVerdict(checks)});
  }
  const normalSeconds=seconds(withoutOutages(normalWindows,recoveryWindows));
  const states=Object.entries(stateBuckets).map(([state,b])=>({state,count:b.latencies.length,p50:percentile(b.latencies,.5),p95:percentile(b.latencies,.95),p99:percentile(b.latencies,.99),windowExpected:state==='normal'?normalWindows.length>0:state==='outage'?faults.some(f=>f.startedAt!=null||f.unavailableAt!=null):faults.some(f=>f.recoveredAt!=null),windowSeconds:seconds(latencyWindows[state]),errors:measurementsAvailable?b.paymentErrors:null,requests:measurementsAvailable?b.paymentRequests:null,tps:state==='normal'&&normalSeconds>0&&measurementsAvailable?b.successes/normalSeconds:null}));
  const steadyOpts={state:!finished?'in_progress':!measurementsAvailable||groups.some(g=>steadyIds.has(g.id)&&g.state!=='complete')?'partial':'complete'};
  const steadyChecks=[
    check('throughput','Window throughput vs offered',measurementsAvailable&&steadyExpected>0?100*steadyBucket.successes/steadyExpected:null,throughputLimit,'%',{...steadyOpts,minimum:true}),
    check('tps','Successful TPS',measurementsAvailable&&steadySeconds>0?steadyBucket.successes/steadySeconds:null,null,'TPS',{...steadyOpts,record:true}),
    check('errors','HTTP errors',steadyBucket.paymentRequests?100*steadyBucket.paymentErrors/steadyBucket.paymentRequests:null,errorLimit,'%',steadyOpts),
    check('completed','Flows completed in window',measurementsAvailable&&steadySeconds>0?steadyBucket.successes:null,null,'flows',{...steadyOpts,record:true}),
    ...[.5,.95,.99].map(p=>check(`p${p*100}`,`Server-side latency · p${p*100}`,percentile(steadyLatencies,p),saved&&p===.99?limits.maximumHoldP99Ms:null,'ms',steadyOpts))
  ];
  const matched=matchedLatencyDrift(steadyDrift,steadyLatencyWindows);
  steadyChecks.push({...check('drift','Latency drift',matched.percent,saved?limits.maximumLatencyDriftPercent:null,'%',steadyOpts),matched});
  const steady={checks:steadyChecks,durationSeconds:steadySeconds,successfulFlows:steadyBucket.successes};
  // Confirmations include drain responses. Setup failures and expected outage
  // impact are recorded separately, never counted as failed confirmations.
  // Window throughput remains a time-based diagnostic, never a completion ratio.
  const shape=loadShape(config.load),planned=Math.round(shape.average*shape.duration);
  const dropped=Number.isFinite(control?.dropped)?Math.max(0,control.dropped):0;
  const admitted=Number.isFinite(control?.started_flows)?Math.max(0,control.started_flows):runFlows;
  const expected=confirmations.attempts;
  const completionState=!finished?'in_progress':!measurementsAvailable||['cancelled','interrupted'].includes(jobStatus)||recording.recording_state!=='complete'||!clock||Math.max(clock.active,control?.active_seconds||0)<shape.duration-.05?'partial':'complete';
  const completion={expected,planned,started:admitted,successful:confirmations.successful,dropped,scope:'payment_confirm_outside_outage',outageAttempts:confirmations.outageAttempts,outageFailures:confirmations.outageFailures,
    check:check('completion','Payment confirmations succeeded outside outage',measurementsAvailable&&expected>0?100*confirmations.successful/expected:null,saved?100:null,'%',{minimum:true,state:completionState})};
  const checks=[...groups.filter(g=>!steadyIds.has(g.id)).flatMap(g=>g.checks),...(steadyIds.size?steadyChecks:[]),completion.check],passed=checks.filter(c=>c.outcome==='pass').length,evaluated=checks.filter(c=>['pass','fail'].includes(c.outcome)).length,total=checks.filter(c=>c.limit!=null).length;
  const measuredVerdict=groupVerdict(checks);
  // Legacy snapshots only cover part of this checklist. Keep their measured
  // failures, but do not promote a historical run to a new overall pass.
  const verdict=!finished?'in_progress':['cancelled','interrupted'].includes(jobStatus)||recording.recording_state!=='complete'?'incomplete':!saved&&measuredVerdict==='pass'?'recorded':measuredVerdict;
  return {version:scenarioChecksVersion,httpErrorScope:'payment_create_and_confirm',latencySettlingSeconds:limits.recoveryWindowSeconds,steady,completion,limits,limitsSaved:saved,groups,states,testCasePlan:testCasePlan(config,groups),passed,evaluated,total,verdict};
}

export async function scenarioChecks(directory,{config,recording,events=[],reconciliation=null,jobStatus=null}){
  const progress=await readJson(path.join(directory,'progress.json'));
  const control=progress?.control?{started_flows:progress.control.started_flows,dropped:progress.control.dropped,active_seconds:progress.control.active_seconds}:null;
  const fingerprint=createHash('sha256').update(JSON.stringify({version:scenarioChecksVersion,config,recording,events,reconciliation,jobStatus,control})).digest('hex');
  const file=path.join(directory,'scenario-checks.json');
  const finished=terminal.has(jobStatus)||Boolean(recording.ended_at&&!jobStatus);
  if(finished){const saved=await readJson(file);if(saved?.fingerprint===fingerprint)return saved.summary;}
  let samples=await lines(path.join(directory,'execution.jsonl'));
  if(!samples.length)samples=(await lines(path.join(directory,'telemetry.jsonl'))).map(p=>({at:recording.started_at+p.t*1000,active:p.activeSeconds??p.t,state:p.state}));
  const args={config,recording,events,samples,reconciliation,jobStatus,control};let db;
  // Results can be opened while load is running. Do not repeatedly scan a
  // growing recorder DB; exact checks are evaluated once it has drained.
  if(!finished)return evaluateScenarioChecks({...args,measurementsAvailable:false});
  try{
    db=new DatabaseSync(path.join(directory,'results.sqlite'),{readOnly:true});db.exec('BEGIN');
    const cols=new Set(db.prepare('PRAGMA table_info(request_events)').all().map(c=>c.name));
    const optional=['server_latency_ms','latency_ms','status'].map(c=>cols.has(c)?c:`NULL AS ${c}`).join(',');
    const requestStart=cols.has('latency_ms')?'observed_at-COALESCE(latency_ms,0)':'observed_at';
    const requests=db.prepare(`SELECT flow_id,operation,api_status_code,observed_at,${optional},CASE WHEN flow_id IS NULL THEN ${requestStart} ELSE MIN(${requestStart}) OVER (PARTITION BY flow_id) END AS flow_started_at FROM request_events WHERE run_id=? AND role='measured'`).iterate(recording.run_id);
    // Iterate request rows once; aggregate flow completions in SQLite so a
    // multi-request payment is never counted as several completed flows.
    const flows=db.prepare(`SELECT MIN(observed_at-COALESCE(latency_ms,0)) AS startedAt,MAX(observed_at) AS at,MAX(CASE WHEN operation='payment_confirm' AND api_status_code>=200 AND api_status_code<300 AND status IN ('succeeded','requires_capture','processing') THEN 1 ELSE 0 END) AS success FROM request_events WHERE run_id=? AND role='measured' AND flow_id IS NOT NULL GROUP BY flow_id`).iterate(recording.run_id);
    const summary=evaluateScenarioChecks({...args,requests,flows});
    if(finished){const temp=`${file}.${randomUUID()}.tmp`;await writeFile(temp,JSON.stringify({fingerprint,summary})+'\n',{mode:0o600});await rename(temp,file);}
    return summary;
  }catch(error){if(error.code==='ERR_SQLITE_ERROR')return evaluateScenarioChecks({...args,measurementsAvailable:false});throw error;}finally{db?.close();}
}
