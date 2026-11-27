import test from 'node:test';
import assert from 'node:assert/strict';
import {finalJobStatus} from '../server/jobs.mjs';

test('a load cannot complete when fault recovery was not verified',()=>{
  assert.equal(finalJobStatus({failure:null,recoveryError:'Recovery could not be verified'}),'failed');
  assert.equal(finalJobStatus({failure:'k6 threshold failed'}),'failed');
  assert.equal(finalJobStatus({cancelled:true,recoveryError:'Recovery could not be verified'}),'cancelled');
  assert.equal(finalJobStatus({}),'complete');
});
