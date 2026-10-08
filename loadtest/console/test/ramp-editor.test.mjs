import test from 'node:test';
import assert from 'node:assert/strict';
import {stageRates,encodeStages} from '../src/ramp-editor.mjs';

test('hold display follows starting TPS and ignores stale targets',()=>{
  const rows=[{kind:'hold',target:4,duration:60},{kind:'ramp',target:4,duration:60}];
  assert.deepEqual(stageRates(rows,1),[1,4]);
  assert.deepEqual(stageRates(rows,2),[2,4]);
  assert.deepEqual(stageRates(rows,1),encodeStages(rows,1).map(s=>s.target_rps));
});

test('holds inherit preceding ramps across faults, reordering and removal',()=>{
  const ramp={kind:'ramp',target:3,duration:60};
  const hold={kind:'hold',target:9,duration:60};
  const fault={kind:'fault',target:9,duration:10,fault:{targetId:'demo',action:'outage'}};
  assert.deepEqual(stageRates([ramp,fault,hold],1),[3,3,3]);
  assert.deepEqual(stageRates([hold,fault,ramp],1),[1,1,3]);
  assert.deepEqual(stageRates([fault,hold],1),[1,1]);
  assert.deepEqual(stageRates([{...ramp,kind:'hold'},hold],1),[1,1]);
});
