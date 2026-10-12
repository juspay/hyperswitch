import test from 'node:test';import assert from 'node:assert/strict';
import {decodeStages,encodeStages} from '../src/ramp-editor.mjs';
import {snapshotFaultPlan,faultScheduler} from '../server/fault-plan.mjs';
const targets=[{id:'redis',actions:['outage'],port:6379,adapter:'process-compose'}];
const stages=encodeStages([{kind:'ramp',target:20,duration:2},{kind:'fault',duration:3,fault:{targetId:'redis',action:'outage'}},{kind:'hold',duration:2}],10);
test('fault steps preserve traffic, order and hidden target identity',()=>{
 assert.equal(stages[1].target_rps,20);assert.deepEqual(encodeStages(decodeStages(JSON.stringify(stages),10),10),stages);
 const plan=snapshotFaultPlan({stages},targets);targets[0].port=6380;assert.equal(plan[0].target.port,6379);assert.equal(plan[0].at,2);
 assert.throws(()=>snapshotFaultPlan({stages},[]),/Timeline stage 2: fault target “redis” is no longer configured/);
 const invalid=structuredClone(stages);invalid[1].fault.action='failover';
 assert.throws(()=>snapshotFaultPlan({stages:invalid},[{id:'redis',label:'Redis',actions:['outage']}]),/Timeline stage 2: Redis does not support “failover”; choose outage/);
 const legacy=structuredClone(stages);legacy[1].fault.action='restart';assert.equal(snapshotFaultPlan({stages:legacy},targets)[0].action,'outage');
 const cloud=snapshotFaultPlan({stages},[{id:'redis',label:'Redis',actions:['failover']}]);assert.equal(cloud[0].action,'failover');
 assert.throws(()=>encodeStages([{kind:'fault',duration:301,fault:{targetId:'redis',action:'outage'}}],10),/300/);
});
test('scheduler follows active time, fires once and never injects a missed fault late',async()=>{
 const plan=snapshotFaultPlan({stages},targets),claimed=new Set(),calls=[],events=[],controls=[];
 const scheduler=faultScheduler(plan,{claim:i=>{if(claimed.has(i))return false;claimed.add(i);return true;},fire:async(...args)=>calls.push(args),event:(...args)=>events.push(args),gate:async()=>controls.push('gate'),release:async()=>controls.push('release'),recover:async()=>controls.push('recover')});
 await scheduler({loadState:'running'},1);await scheduler({loadState:'paused'},3);assert.equal(calls.length,0);
 await scheduler({loadState:'running'},2.1);await scheduler({loadState:'running'},3);assert.equal(calls.length,1);assert.equal(calls[0][1].durationSeconds,3);
 assert.deepEqual(controls,['gate']);
 const lifecycle=calls[0][3];await lifecycle.onStarted();await lifecycle.onRestore();await lifecycle.onRecovered();
 assert.deepEqual(controls,['gate','release','gate','recover']);
 claimed.clear();await scheduler({loadState:'running'},7);assert.equal(calls.length,1);assert.match(events[0][1],/missed/);
});

test('scheduler releases a setup gate when fault activation aborts',async()=>{
 const controls=[];
 const scheduler=faultScheduler(snapshotFaultPlan({stages},targets),{claim:()=>true,gate:async()=>controls.push('gate'),release:async()=>controls.push('release'),recover:async end=>controls.push(['recover',end]),fire:async(_job,_input,_target,lifecycle)=>lifecycle.onAborted(),event:()=>{}});
 await scheduler({loadState:'running'},2);
 assert.deepEqual(controls,['gate',['recover',stages[0].duration_seconds+stages[1].duration_seconds]]);
});
