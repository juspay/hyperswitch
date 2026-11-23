import test from 'node:test';
import assert from 'node:assert/strict';
import {formatDuration} from '../src/duration.mjs';
test('scenario durations use hours, minutes and seconds as needed',()=>{
  for(const [seconds,label] of [[0,'0 sec'],[30,'30 sec'],[60,'1 min'],[90,'1 min 30 sec'],[300,'5 min'],[3661,'1 hr 1 min 1 sec']])assert.equal(formatDuration(seconds),label);
  assert.equal(formatDuration(-1),'—');assert.equal(formatDuration(NaN),'—');
});
