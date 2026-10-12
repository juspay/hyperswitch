import test from 'node:test';import assert from 'node:assert/strict';
import {serverLatency,latencyHeaders} from '../server-latency.mjs';
test('server latency parses case-insensitive milliseconds and preserves zero',()=>{
 assert.equal(serverLatency({'X-Hs-Latency':' 123.5 '}),123.5);assert.equal(serverLatency({'x-hs-latency':'0'}),0);
 for(const value of ['', 'true','NaN','Infinity','-1','10ms',null,10])assert.equal(serverLatency({'x-hs-latency':value}),null);
 assert.equal(serverLatency({}),null);
});
test('router latency opt-in replaces conflicting casing and keeps other headers',()=>{
 assert.deepEqual(latencyHeaders({'X-HS-LATENCY':'false','x-feature':'test'}),{'x-feature':'test','x-hs-latency':'true'});
});
