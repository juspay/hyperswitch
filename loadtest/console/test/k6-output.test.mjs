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

test('fixture progress replaces each labelled row and retains intervening logs',()=>{
 const result=k6Output('Organization ready\nMerchants [--------------------] 0% 0/1\nMerchants [====================] 100% 1/1\nResolve flags [--------------------] 0% 0/1\nResolve flags [====================] 100% 1/1\nWaiting for service refresh\nService refresh [--------------------] 0% 0/30\nWARN preserved\nService refresh [==------------------] 10% 3/30 | 27s remaining\nService refresh [====----------------] 20% 6/30 | 24s remaining\n');
 assert.equal((result.history.match(/Merchants \[/g)||[]).length,1);assert.equal((result.history.match(/Service refresh \[/g)||[]).length,1);assert.match(result.history,/24s remaining/);assert.doesNotMatch(result.history,/27s remaining|0% 0\//);assert.match(result.history,/Organization ready/);assert.match(result.history,/WARN preserved/);
});
test('fixture readiness remains pending while Superposition refresh is running',async()=>{
 const {fixtureSteps}=await import('../src/fixture-steps.mjs');const steps=fixtureSteps({status:'running',progress:{fixtures:{completed:1,total:1},setup:{stage:'superposition',superposition:{status:'waiting',remainingSeconds:27,totalSeconds:30}}}});
 assert.equal(steps[0].status,'complete');assert.equal(steps[1].status,'complete');assert.equal(steps[2].status,'running');assert.equal(steps[2].detail,'27s remaining');assert.equal(steps[3].status,'pending');
 const done=fixtureSteps({status:'complete',progress:{setup:{superposition:{status:'complete'}}}});assert.ok(done.every(step=>step.status==='complete'));
});
