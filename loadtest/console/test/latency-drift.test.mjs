import test from 'node:test';
import assert from 'node:assert/strict';
import {matchedLatencyDrift} from '../server/latency-drift.mjs';
const windows=[{from:0,to:90000}];
const sample=(second,flow,ms)=>({observed_at:second*1000,flow_name:flow,server_latency_ms:ms});
const rows=[sample(5,'cit',100),sample(10,'guest',20),sample(15,'guest',20),sample(31,'cit',100),sample(59,'guest',20),sample(70,'cit',100),sample(80,'cit',100),sample(85,'guest',20)];
test('bounded expansion matches per-flow counts instead of allowing the mixture to change the median',()=>{
 const r=matchedLatencyDrift(rows,windows);
 assert.deepEqual(r.counts,[{flow:'cit',count:2},{flow:'guest',count:2}]);
 assert.equal(r.samplesPerWindow,4);assert.equal(r.firstMedianMs,60);assert.equal(r.lastMedianMs,60);assert.equal(r.percent,0);
 assert.equal(r.firstWindow.to,31000);assert.equal(r.lastWindow.from,59000);assert.ok(r.firstWindow.to<r.lastWindow.from);
});
test('real latency increases still fail while sample selection is independent of latency',()=>{
 const r=matchedLatencyDrift(rows.map(r=>({...r,server_latency_ms:r.observed_at>45000?r.server_latency_ms*2:r.server_latency_ms})),windows);
 assert.equal(r.percent,100);assert.deepEqual(r.counts,[{flow:'cit',count:2},{flow:'guest',count:2}]);
});
test('latency decreases show the absolute percentage difference rather than zero',()=>{
 const r=matchedLatencyDrift([sample(5,'cit',83.5),sample(85,'cit',80.5)],windows);
 assert.ok(Math.abs(r.percent-100*3/83.5)<1e-10);
});
test('unmatched flow types report insufficient evidence and never borrow across the tolerance bound',()=>{
 const r=matchedLatencyDrift([...rows,sample(1,'sdk',1),sample(45,'sdk',1)],windows);
 assert.equal(r.percent,null);assert.match(r.reason,/sdk/);
});
test('tiny windows keep their early and late samples disjoint and expose the effective tolerance',()=>{
 const r=matchedLatencyDrift([sample(.1,'a',10),sample(2.9,'a',11)],[{from:0,to:3000}]);
 assert.equal(r.toleranceSeconds,.25);assert.equal(r.percent,10);assert.equal(r.samplesPerWindow,1);
});
