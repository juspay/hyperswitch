import test from 'node:test';import assert from 'node:assert/strict';
import {loadTimeline} from '../src/load-shape.mjs';
import {activeSamples,activeToWall,eventActiveTime,plannedEvents,stageWallTimes,executionPhases,targetAt,wallToActive} from '../src/chart-timeline.mjs';
test('timeline preserves full plan and exposes only display-safe fault fields',()=>{
 const plan=loadTimeline({start_rps:0,stages:[{duration_seconds:10,target_rps:10},{duration_seconds:5,target_rps:10,fault:{targetId:'pg',action:'outage'}},{duration_seconds:10,target_rps:20}]},[{index:1,target:{label:'PostgreSQL',secret:'private'}}]);
 assert.equal(plan.at(-1).end,25);assert.equal(targetAt(5,plan),5);assert.equal(targetAt(12,plan),10);assert.equal(targetAt(20,plan),15);
 assert.deepEqual(plannedEvents(plan).map(e=>e.t),[0,10,15,15]);assert.match(plannedEvents(plan)[2].label,/recovery planned/);assert.doesNotMatch(JSON.stringify(plan),/private|secret/);
 const restart=loadTimeline({start_rps:10,stages:[{duration_seconds:5,target_rps:10,fault:{targetId:'vault',action:'restart'}}]});assert.equal(plannedEvents(restart).length,1);
});
test('paused samples and wall-time events align to active time without shifting future stages',()=>{
 const points=[{t:5,activeSeconds:5},{t:10,activeSeconds:10},{t:20,activeSeconds:10},{t:25,activeSeconds:15}];
 assert.deepEqual(activeSamples(points).map(p=>p.t),[5,10,15]);
 assert.equal(eventActiveTime({at:17000},0,points),10);assert.equal(eventActiveTime({at:22000},0,points),12);assert.equal(eventActiveTime({at:30000},0,points),15);
 assert.equal(activeToWall(10,points),10);assert.equal(activeToWall(10,points,'end'),20);assert.equal(activeToWall(12,points),22);assert.equal(wallToActive(17,points),10);
 assert.equal(loadTimeline({total_rps:10,duration_seconds:60})[0].end,60);
});
test('the stage after an outage begins only after recovery verification',()=>{
 const timeline=[{start:0,end:10,from:1,target:1,kind:'hold'},{start:10,end:20,from:1,target:1,kind:'fault',fault:{target:'Card vault',action:'outage'}},{start:20,end:25,from:1,target:5,kind:'ramp'}];
 const points=[{t:0,activeSeconds:0},{t:10,activeSeconds:10},{t:20,activeSeconds:20},{t:30,activeSeconds:20},{t:35,activeSeconds:25}];
 const stages=stageWallTimes(timeline,points,[{kind:'fault_restored',at:32_000}],0);
 assert.equal(stages[1].wallEnd,32);assert.equal(stages[2].wallStart,32);assert.equal(stages[2].wallEnd,35);
});
test('future stages include accumulated fault delays instead of collapsing at the latest sample',()=>{
 const timeline=[{start:0,end:20,from:1,target:1,kind:'hold'},{start:20,end:40,from:1,target:1,kind:'fault',fault:{target:'Card vault',action:'outage'}},{start:40,end:45,from:1,target:5,kind:'ramp'},{start:45,end:65,from:5,target:5,kind:'hold'}];
 const points=[{t:0,activeSeconds:0},{t:20,activeSeconds:20},{t:30,activeSeconds:20},{t:50,activeSeconds:40},{t:60,activeSeconds:40}];
 const stages=stageWallTimes(timeline,points,[{kind:'fault_restored',at:60_000}],0);
 assert.equal(stages[2].wallStart,60);assert.equal(stages[2].wallEnd,65);assert.equal(stages[3].wallEnd,85);
 assert.equal(activeToWall(65,points),85);
});
import {compactSamples} from '../src/chart-timeline.mjs';
import {availabilityGraphEvents} from '../src/chart-timeline.mjs';
test('DB shading and markers use observed availability rather than provider completion',()=>{
 const timeline=[{start:0,end:10,kind:'hold',target:5},{start:10,end:11,kind:'fault',target:5,fault:{target:'PostgreSQL',action:'failover'}},{start:11,end:21,kind:'hold',target:5}];
 const points=[{t:0,activeSeconds:0},{t:10,activeSeconds:10},{t:11,activeSeconds:11},{t:100,activeSeconds:11},{t:110,activeSeconds:21}];
 const events=[{at:10000,kind:'fault_requested',message:'PostgreSQL · failover requested'},{at:11000,kind:'fault_started',message:'PostgreSQL · failover'},{at:15000,kind:'database_unavailable',message:'DB · unavailable'},{at:30000,kind:'database_available',message:'DB · available'},{at:100000,kind:'fault_restored',message:'PostgreSQL · recovery verified'}];
 assert.deepEqual(availabilityGraphEvents(events,timeline).map(e=>e.kind),['failover_requested','database_unavailable','database_available']);
 assert.deepEqual(executionPhases(timeline,points,events,0,110).filter(p=>p.kind==='outage'),[{start:15,end:30,kind:'outage',label:'DB outage'}]);
 assert.equal(executionPhases(timeline,points,events.filter(e=>!e.kind.startsWith('database_')),0,110).some(p=>p.kind==='outage'),false);
 assert.equal(stageWallTimes(timeline,points,events,0)[2].wallStart,100);
});
test('a rejected failover advances the following stage without a DB outage band',()=>{
 const timeline=[{start:0,end:10,kind:'hold',target:5},{start:10,end:70,kind:'fault',target:5,fault:{target:'PostgreSQL',action:'failover'}},{start:70,end:80,kind:'hold',target:5}];
 const points=[{t:0,activeSeconds:0},{t:10,activeSeconds:10},{t:12,activeSeconds:70},{t:22,activeSeconds:80}];
 const events=[{at:10000,kind:'fault_requested',message:'PostgreSQL · failover requested'},{at:12000,kind:'fault_error',message:'ABORTED'},{at:12000,kind:'fault_skipped',message:'PostgreSQL · failover rejected · continuing'}];
 assert.equal(stageWallTimes(timeline,points,events,0)[2].wallStart,12);
 assert.equal(executionPhases(timeline,points,events,0,22).some(p=>p.kind==='outage'),false);
 assert.deepEqual(availabilityGraphEvents(events,timeline).map(e=>e.kind),['failover_requested']);
});
test('execution bands start at verified outage and keep recovery delay out of the ramp',()=>{
 const timeline=[{start:0,end:20,from:1,target:1,kind:'hold'},{start:20,end:40,from:1,target:1,kind:'fault',fault:{target:'Card vault',action:'outage'}},{start:40,end:45,from:1,target:5,kind:'ramp'},{start:45,end:65,from:5,target:5,kind:'hold'}];
 const points=[{t:0,activeSeconds:0},{t:20,activeSeconds:20},{t:22,activeSeconds:20},{t:42,activeSeconds:40},{t:50,activeSeconds:40},{t:55,activeSeconds:45},{t:75,activeSeconds:65}];
 const activity=[{kind:'fault_requested',at:20_000,message:'Card vault · requested'},{kind:'fault_started',at:22_000,message:'Card vault · outage'},{kind:'fault_restore',at:42_000,message:'Card vault · restoring'},{kind:'fault_restored',at:50_000,message:'Card vault · recovery verified'}];
 assert.deepEqual(executionPhases(timeline,points,activity,0),[{start:0,end:22,kind:'hold',label:'Hold'},{start:22,end:50,kind:'outage',label:'Card vault outage'},{start:50,end:55,kind:'ramp',label:'Ramp'},{start:55,end:75,kind:'hold',label:'Hold'}]);
 const unrecovered=executionPhases(timeline,points.slice(0,4),activity.slice(0,3),0,80);assert.equal(unrecovered.at(-1).kind,'outage');assert.equal(unrecovered.at(-1).end,80);
});
test('bounded history keeps the beginning and latest sample',()=>{const points=Array.from({length:1801},(_,t)=>({t}));const compact=compactSamples(points);assert.equal(compact[0].t,0);assert.equal(compact.at(-1).t,1800);assert.ok(compact.length<1800);});
