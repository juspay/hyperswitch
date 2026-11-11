import {loadTimeline} from '../src/load-shape.mjs';

// Preserve the wall/active clock separately from compacted chart samples. A
// plateau is a pause or recovery gate, not an extension of configured load time.
export function executionSample(progress,load,at=Date.now()){
  const active=progress.control?.active_seconds;
  if(!Number.isFinite(active))return null;
  const stages=loadTimeline(load),stage=stages.find(s=>active>=s.start&&active<s.end);
  return {at,active,stageIndex:stage?.index??null,state:progress.control.state};
}
export function executionClock(samples,recording){
  const start=Number(recording.started_at),end=Number(recording.ended_at)||Date.now();
  const points=samples.filter(p=>Number.isFinite(p.at)&&Number.isFinite(p.active)).sort((a,b)=>a.at-b.at);
  // Older runs retain wall time and active time in their telemetry.
  if(!points.length)return null;
  const first=points[0];
  if(first.active>0)points.unshift({at:Math.max(start,first.at-first.active*1000),active:0});
  const intervals=[];
  for(let i=1;i<points.length;i++){
    const a=points[i-1],b=points[i];
    if(b.at>a.at&&a.at<end&&b.active>a.active)intervals.push({from:a.at,to:Math.min(end,b.at),activeFrom:a.active,activeTo:b.active});
  }
  return {points,intervals,end,active:points.at(-1).active};
}
export function stageIntervals(clock,stage){
  if(!clock)return [];
  return clock.intervals.flatMap(p=>{
    const from=Math.max(stage.start,p.activeFrom),to=Math.min(stage.end,p.activeTo);
    if(to<=from)return [];
    const wall=a=>p.from+(a-p.activeFrom)/(p.activeTo-p.activeFrom)*(p.to-p.from);
    return [{from:wall(from),to:wall(to),activeFrom:from,activeTo:to}];
  });
}
