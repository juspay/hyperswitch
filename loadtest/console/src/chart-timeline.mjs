export const activeTime=p=>Number.isFinite(p.activeSeconds)?p.activeSeconds:p.t;
// Keep wall-clock recording intact, but stop extending the visible graph once
// the current pause has drained and its first zero-throughput sample arrives.
export function pausedChartSamples(points,state){
 if(state!=='paused'||!points.length)return points;
 let start=points.length-1;
 while(start>0&&points[start-1].state==='paused')start--;
 if(points[start].state!=='paused')return points;
 const zero=points.findIndex((p,i)=>i>=start&&p.successfulTps===0&&(p.inFlight??0)===0);
 return points.slice(0,(zero<0?start:zero)+1);
}
export function activeSamples(points){const result=[];for(const p of points){const t=activeTime(p);if(!Number.isFinite(t))continue;const row={...p,t};if(result.at(-1)?.t===t)result[result.length-1]=row;else result.push(row);}return result;}
export function eventActiveTime(event,startedAt,points){
 const wall=Math.max(0,(event.at-startedAt)/1000);
 if(!points.length)return wall;
 const next=points.findIndex(p=>p.t>=wall);
 if(next===0)return Math.min(wall,activeTime(points[0]));
 if(next<0)return activeTime(points.at(-1));
 const a=points[next-1],b=points[next],fraction=(wall-a.t)/(b.t-a.t||1);
 return activeTime(a)+(activeTime(b)-activeTime(a))*fraction;
}
export function activeToWall(seconds,points,edge='start'){
 const samples=(points || []).filter(point=>Number.isFinite(point?.t)&&Number.isFinite(activeTime(point)));
 if(!samples.length)return seconds;
 const exact=samples.filter(point=>Math.abs(activeTime(point)-seconds)<.01);
 if(exact.length)return edge==='end'?exact.at(-1).t:exact[0].t;
 const next=samples.find(point=>activeTime(point)>seconds);
 if(!next){const last=samples.at(-1);return last.t+Math.max(0,seconds-activeTime(last));}
 const index=samples.indexOf(next),previous=samples[index-1];
 if(!previous)return Math.max(0,next.t-(activeTime(next)-seconds));
 const fraction=(seconds-activeTime(previous))/(activeTime(next)-activeTime(previous));
 return previous.t+(next.t-previous.t)*fraction;
}
export function wallToActive(seconds,points){
 const samples=(points || []).filter(point=>Number.isFinite(point?.t)&&Number.isFinite(activeTime(point)));
 if(!samples.length)return seconds;
 const next=samples.find(point=>point.t>=seconds);
 if(!next)return activeTime(samples.at(-1));
 const index=samples.indexOf(next),previous=samples[index-1];
 if(!previous)return activeTime(next);
 const fraction=(seconds-previous.t)/(next.t-previous.t||1);
 return activeTime(previous)+(activeTime(next)-activeTime(previous))*fraction;
}
export function plannedEvents(timeline){return timeline.flatMap((s,index)=>[
 {t:s.start,edge:index>0&&timeline[index-1].fault?'end':'start',label:s.fault?`${s.fault.target} · fault planned`:s.kind==='ramp'?`Ramp ${s.from} → ${s.target} TPS`:`Hold ${s.target} TPS`,kind:s.kind},
 ...(s.fault?.action==='outage'?[{t:s.end,edge:'start',label:`${s.fault.target} · recovery planned`,kind:'recovery'}]:[])
]);}
function eventTarget(event){
 const parts=(event.message||'').split(' · ');
 while(parts.length>1&&['SIMULATED','Local','GCP','AWS','Azure','Custom'].includes(parts[0]))parts.shift();
 return event.target||parts[0]||'Service';
}
function failoverTargets(timeline,activity){
 return new Set([...timeline.filter(s=>s.fault?.action==='failover').map(s=>s.fault.target),...activity.filter(e=>/ · failover(?: requested)?$/.test(e.message||'')).map(eventTarget)]);
}
export function availabilityGraphEvents(activity,timeline=[]){
 const failovers=failoverTargets(timeline,activity);
 return activity.flatMap(e=>{
  const failover=failovers.has(eventTarget(e));
  if(failover&&e.kind==='fault_requested')return [{...e,kind:'failover_requested'}];
  return ['pause','resume','database_unavailable','database_available'].includes(e.kind)||(['fault_requested','fault_started','fault_restore','fault_restored'].includes(e.kind)&&!failover)?[e]:[];
 });
}
// Fault recovery pauses the active clock. The following stage therefore starts
// at the verified-recovery event, rather than at the recovery request or its
// nominal active-time boundary.
export function stageWallTimes(timeline,points,activity=[],startedAt=0){
 const recovered=activity.filter(event=>['fault_restored','fault_skipped'].includes(event.kind)&&Number.isFinite(event?.at)).sort((a,b)=>a.at-b.at),stages=[];
 let recoveryIndex=0;
 for(const [index,stage] of timeline.entries()){
  const previous=timeline[index-1];
  const verified=previous?.fault?recovered[recoveryIndex++]:null;
  const start=verified?Math.max(0,(verified.at-startedAt)/1000):activeToWall(stage.start,points,index>0&&previous?.fault?'end':'start');
  stages.push({...stage,wallStart:start});
 }
 return stages.map((stage,index)=>({...stage,wallEnd:stage.fault?(stages[index+1]?.wallStart??activeToWall(stage.end,points,'end')):activeToWall(stage.end,points,'start')}));
}
// Execution phases use wall time. An outage spans verified interruption through
// verified recovery, including the time spent restoring the service.
export function executionPhases(timeline,points,activity=[],startedAt=0,elapsedSeconds=0){
 const stages=stageWallTimes(timeline,points,activity,startedAt),end=Math.max(elapsedSeconds,points.at(-1)?.t||0,stages.at(-1)?.wallEnd||0);
 const outages=[],open=new Map(),target=eventTarget;
 const failovers=failoverTargets(timeline,activity);
 for(const event of [...activity].sort((a,b)=>a.at-b.at)){
  if(event.kind==='database_unavailable'||(event.kind==='fault_started'&&!failovers.has(target(event))))open.set(target(event),{start:Math.max(0,(event.at-startedAt)/1000),target:target(event)});
  if((event.kind==='database_available'||(event.kind==='fault_restored'&&!failovers.has(target(event))))&&open.has(target(event))){const fault=open.get(target(event));outages.push({...fault,end:Math.max(fault.start,(event.at-startedAt)/1000)});open.delete(target(event));}
 }
 for(const fault of open.values())outages.push({...fault,end});
 const boundaries=[...new Set([0,end,...stages.flatMap(s=>[s.wallStart,s.wallEnd]),...outages.flatMap(f=>[f.start,f.end])])].filter(t=>t>=0&&t<=end).sort((a,b)=>a-b),phases=[];
 for(let i=1;i<boundaries.length;i++){
  const start=boundaries[i-1],finish=boundaries[i],mid=(start+finish)/2,fault=outages.find(f=>mid>=f.start&&mid<f.end),stage=stages.find(s=>mid>=s.wallStart&&mid<s.wallEnd)||stages.at(-1);
  if(!stage&&!fault)continue;
  const kind=fault?'outage':stage?.kind==='ramp'?'ramp':'hold',label=fault?`${fault.target==='PostgreSQL'?'DB':fault.target} outage`:kind==='ramp'?'Ramp':'Hold';
  const previous=phases.at(-1);if(previous?.label===label&&previous.end===start)previous.end=finish;else phases.push({start,end:finish,kind,label});
 }
 return phases;
}
export function targetAt(t,timeline){const stage=timeline.find(s=>t>=s.start&&t<s.end)||timeline.at(-1);if(!stage)return null;return stage.kind==='ramp'?stage.from+(stage.target-stage.from)*Math.max(0,Math.min(1,(t-stage.start)/(stage.end-stage.start||1))):stage.target;}
export function compactSamples(points,limit=1800){return points.length>limit?points.filter((_,i)=>i%2===0||i===points.length-1):points;}

// Cards and the main chart share metric selection, interval edges and wall time.
export function metricValue(point,metric,flow='all'){
 const key=metric==='tps'?'successfulTps':metric;
 const source=flow==='all'||['errors','backlog'].includes(metric)?point:point?.flows?.[flow];
 // Empty observed latency intervals are zero on charts; never bridge them.
 if(['p99','serverP99'].includes(metric)&&(source?.successfulTps===0||source?.[key]===null))return 0;
 return source?.[key];
}
export function metricSeries(points,metric,flow='all'){
 return (points||[]).filter(p=>Number.isFinite(p?.t)).map(p=>({t:p.t,intervalStart:p.intervalStart,value:metricValue(p,metric,flow)}));
}
export function rateIntervals(points){
 return points.flatMap((point,index)=>[{t:Number.isFinite(point.intervalStart)?point.intervalStart:index?points[index-1].t:0,value:point.value},point]);
}
export function chartHorizon(points,durationSeconds,timeline=[],activity=[],startedAt=0){
 const wall=stageWallTimes(timeline,points,activity,startedAt),planned=plannedEvents(timeline);
 return Math.max(1,Number(durationSeconds)||0,points.at(-1)?.t||0,wall.at(-1)?.wallEnd||0,planned.at(-1)?.t||0);
}
