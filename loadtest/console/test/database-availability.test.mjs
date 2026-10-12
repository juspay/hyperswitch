import test from 'node:test';
import assert from 'node:assert/strict';
import {watchDatabaseAvailability} from '../server/database-availability.mjs';
test('probes mark only SQL availability transitions, independently of a long cloud operation',async()=>{
 const samples=[true,false,false,true,true,false,true],events=[],observed=[];
 let resolve;const finished=new Promise(r=>resolve=r);
 const observer=watchDatabaseAvailability({intervalMs:1,check:async()=>{const ok=samples.shift();if(!ok)throw Error('connection unavailable');return {ok};},onSample:state=>{observed.push(state);if(!samples.length)resolve();},onChange:state=>events.push(state)});
 await finished;await observer.stop();
 assert.deepEqual(events,['unavailable','available','unavailable','available']);assert.equal(observed.length,7);
});
test('restarting from an unavailable observation records actual recovery',async()=>{
 let resolve;const checked=new Promise(r=>resolve=r),events=[];
 const observer=watchDatabaseAvailability({initial:'unavailable',intervalMs:1,check:async()=>({ok:true}),onChange:state=>{events.push(state);resolve();}});
 await checked;await observer.stop();assert.deepEqual(events,['available']);
});
test('stopping an in-flight probe does not emit a late availability marker',async()=>{
 let finish;const check=new Promise(r=>finish=r),events=[];
 const observer=watchDatabaseAvailability({check:()=>check,onChange:s=>events.push(s)});
 const stopped=observer.stop();finish({ok:false});await stopped;assert.deepEqual(events,[]);
});
