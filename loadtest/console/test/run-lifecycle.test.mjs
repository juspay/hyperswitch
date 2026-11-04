import test from 'node:test';
import assert from 'node:assert/strict';
import {recordingFinished,runJobPresentation} from '../src/run-lifecycle.mjs';
test('completed load and recording remain complete while cloud cleanup keeps the job active',()=>{
 const job={command:'load',status:'running',progress:{control:{state:'completed'}}};
 assert.equal(recordingFinished(job),true);
 assert.deepEqual(runJobPresentation(job.status,{recording_state:'complete',ended_at:100}),{jobStatus:'complete',cleanupPending:true});
 assert.deepEqual(runJobPresentation('failed',{recording_state:'complete',ended_at:100}),{jobStatus:'failed',cleanupPending:false});
});
test('running, draining, incomplete recordings and smoke tests are not prematurely completed',()=>{
 for(const state of ['running','draining','paused'])assert.equal(recordingFinished({command:'load',progress:{control:{state}}}),false);
 assert.equal(recordingFinished({command:'smoke',progress:{control:{state:'completed'}}}),false);
 assert.equal(runJobPresentation('running',{recording_state:'running'}).jobStatus,'running');
 assert.equal(runJobPresentation('cancelling',{recording_state:'incomplete',ended_at:100}).jobStatus,'cancelling');
});
