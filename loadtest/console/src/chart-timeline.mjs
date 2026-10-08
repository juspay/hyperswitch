export const activeTime=p=>Number.isFinite(p.activeSeconds)?p.activeSeconds:p.t;
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
export function plannedEvents(timeline){return timeline.flatMap(s=>[
 {t:s.start,label:s.fault?`${s.fault.target} · ${s.fault.action}`:s.kind==='ramp'?`Ramp ${s.from} → ${s.target} TPS`:`Hold ${s.target} TPS`,kind:s.kind},
 ...(s.fault?.action==='outage'?[{t:s.end,label:`${s.fault.target} · recovery begins (planned)`,kind:'recovery'}]:[])
]);}
export function targetAt(t,timeline){const stage=timeline.find(s=>t>=s.start&&t<s.end)||timeline.at(-1);if(!stage)return null;return stage.kind==='ramp'?stage.from+(stage.target-stage.from)*Math.max(0,Math.min(1,(t-stage.start)/(stage.end-stage.start||1))):stage.target;}
export function compactSamples(points,limit=1800){return points.length>limit?points.filter((_,i)=>i%2===0||i===points.length-1):points;}
