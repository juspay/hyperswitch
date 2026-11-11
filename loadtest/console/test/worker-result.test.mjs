import test from 'node:test';
import assert from 'node:assert/strict';
import { workerFailure } from '../server/worker-result.mjs';

test('threshold exits describe failed checks without calling complete recording a worker crash', () => {
  const summary = {metrics:{http_req_failed:{thresholds:{'rate<0.01':{ok:false}}},dropped_iterations:{thresholds:{'count==0':{ok:true}}}}};
  const message = workerFailure(99, null, summary);
  assert.match(message, /thresholds failed/);
  assert.match(message, /http_req_failed \(rate<0.01\)/);
  assert.doesNotMatch(message, /dropped_iterations/);
  assert.match(message, /Results/);
  assert.equal(workerFailure(0), null);
  assert.equal(workerFailure(1,{error:'Recording incomplete'}), 'Recording incomplete');
});
