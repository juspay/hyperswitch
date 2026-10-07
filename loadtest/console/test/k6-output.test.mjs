import test from 'node:test';
import assert from 'node:assert/strict';
import { k6Output } from '../src/k6-output.mjs';

test('k6 progress replaces previous frames and preserves warnings and summary', () => {
  const output = `k6 started\n\nrunning (0m01.0s), 001/120 VUs\nmod_cit_off [  1% ] 01/60 VUs 1.5 iters/s\n\nWARN request failed\nrunning (0m02.0s), 002/120 VUs\nmod_cit_off [  2% ] 02/60 VUs 1.5 iters/s\nnomod_sdk [  2% ] 01/36 VUs 0.9 iters/s\n\nhttp_req_duration: 100ms\n`;
  const result = k6Output(output);
  assert.match(result.progress,/0m02/);
  assert.doesNotMatch(result.progress,/0m01|1%/);
  assert.match(result.progress,/nomod_sdk/);
  assert.match(result.history,/WARN request failed/);
  assert.match(result.history,/http_req_duration/);
  assert.doesNotMatch(result.history,/VUs/);
  assert.match(result.before,/k6 started/);
  assert.match(result.before,/WARN request failed/);
  assert.match(result.after,/http_req_duration/);
});

test('ANSI output and truncated progress rows remain readable', () => {
  const result = k6Output('\x1b[2Kmod_guest [ 50% ] 1/10 VUs 1 iters/s\rmod_guest [ 51% ] 1/10 VUs 1 iters/s');
  assert.equal(result.progress.split('\n').length,1);
  assert.match(result.progress,/51%/);
  assert.equal(result.history,'');
  assert.equal(k6Output('Creating fixtures\nReady').history,'Creating fixtures\nReady');
});
