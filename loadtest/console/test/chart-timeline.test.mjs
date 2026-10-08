import test from 'node:test';import assert from 'node:assert/strict';
import {loadTimeline} from '../src/load-shape.mjs';
import {activeSamples,eventActiveTime,plannedEvents,targetAt} from '../src/chart-timeline.mjs';
test('timeline preserves full plan and exposes only display-safe fault fields',()=>{
 const plan=loadTimeline({start_rps:0,stages:[{duration_seconds:10,target_rps:10},{duration_seconds:5,target_rps:10,fault:{targetId:'pg',action:'outage'}},{duration_seconds:10,target_rps:20}]},[{index:1,target:{label:'PostgreSQL',secret:'private'}}]);
 assert.equal(plan.at(-1).end,25);assert.equal(targetAt(5,plan),5);assert.equal(targetAt(12,plan),10);assert.equal(targetAt(20,plan),15);
 assert.deepEqual(plannedEvents(plan).map(e=>e.t),[0,10,15,15]);assert.match(plannedEvents(plan)[2].label,/recovery begins \(planned\)/);assert.doesNotMatch(JSON.stringify(plan),/private|secret/);
 const restart=loadTimeline({start_rps:10,stages:[{duration_seconds:5,target_rps:10,fault:{targetId:'vault',action:'restart'}}]});assert.equal(plannedEvents(restart).length,1);
});
test('paused samples and wall-time events align to active time without shifting future stages',()=>{
 const points=[{t:5,activeSeconds:5},{t:10,activeSeconds:10},{t:20,activeSeconds:10},{t:25,activeSeconds:15}];
 assert.deepEqual(activeSamples(points).map(p=>p.t),[5,10,15]);
 assert.equal(eventActiveTime({at:17000},0,points),10);assert.equal(eventActiveTime({at:22000},0,points),12);assert.equal(eventActiveTime({at:30000},0,points),15);
 assert.equal(loadTimeline({total_rps:10,duration_seconds:60})[0].end,60);
});
import {compactSamples} from '../src/chart-timeline.mjs';
test('bounded history keeps the beginning and latest sample',()=>{const points=Array.from({length:1801},(_,t)=>({t}));const compact=compactSamples(points);assert.equal(compact[0].t,0);assert.equal(compact.at(-1).t,1800);assert.ok(compact.length<1800);});
