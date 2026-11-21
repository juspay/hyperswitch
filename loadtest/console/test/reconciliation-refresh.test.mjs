import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import vm from 'node:vm';

// Execute the actual stream handler with a controlled API and event stream.
const source=await readFile(new URL('../src/RunScreen.svelte',import.meta.url),'utf8');
const handler=source.slice(source.indexOf('  function watchReconciliation(runId){'),source.indexOf('  const modeLabel'));
function screen(fetchRun){
 const state={selectedRun:{id:'run-a',scenarioChecks:{verdict:'incomplete'}},reconciliationJob:{id:'job'},reconciliationEvents:null,reconciling:true,reconciliation:null,runTimeline:[],error:'',api:{run:fetchRun},watchJobStream(id,callback){state.complete=callback;return {close(){}};}};
 vm.createContext(state);vm.runInContext(handler,state);state.watchReconciliation('run-a');return state;
}
test('completed reconciliation refreshes run summary from the server',async()=>{
 const refreshed={id:'run-a',scenarioChecks:{groups:[{kind:'integrity',checks:[{actual:0,outcome:'pass'}]}]}};
 const state=screen(async id=>{assert.equal(id,'run-a');return refreshed;});
 await state.complete({status:'completed',result:{missing:0,total:1459}});
 assert.equal(state.selectedRun,refreshed);
 assert.equal(state.reconciliation.missing,0);assert.equal(state.reconciling,false);
});
test('a late refresh cannot overwrite another selected run',async()=>{
 let resolve;const state=screen(()=>new Promise(r=>resolve=r));
 const pending=state.complete({status:'completed',result:{missing:0}});
 state.selectedRun={id:'run-b'};resolve({id:'run-a'});await pending;
 assert.equal(state.selectedRun.id,'run-b');
});
test('failed reconciliation retains its error and does not fetch a new verdict',async()=>{
 const state=screen(()=>assert.fail('failed job must not refresh'));
 await state.complete({status:'failed',error:'Database unavailable'});
 assert.equal(state.error,'Database unavailable');
 assert.equal(state.selectedRun.scenarioChecks.verdict,'incomplete');
});
