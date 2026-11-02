import test from 'node:test';
import assert from 'node:assert/strict';
import {convertRule} from './import-loki-rules.mjs';
const fixture=()=>({uid:'test',title:'Webhook failure',intervalSeconds:300,isPaused:false,for:'0s',noDataState:'OK',execErrState:'OK',condition:'C',data:[
  {refId:'A',datasourceUid:'loki',relativeTimeRange:{from:300,to:0},model:{expr:'count_over_time({app="test"}[$__range])',queryType:'instant',intervalMs:1000}},
  {refId:'B',datasourceUid:'__expr__',model:{type:'reduce',expression:'A',reducer:'count'}},
  {refId:'C',datasourceUid:'__expr__',model:{type:'threshold',expression:'B',conditions:[{evaluator:{type:'gt',params:[0]}}]}}
]});
test('source predicate, reducer and threshold survive import, without enabling delivery',()=>{
 const r=fixture(),c=convertRule(r,'xyne_test');
 assert.equal(c.body.alerts_configuration.query.expression,r.data[0].model.expr);
 assert.equal(c.body.alerts_configuration.query.reducer,'count');
 assert.equal(c.body.alerts_configuration.interval_seconds,300);
 assert.equal(c.body.alerts_configuration.query.delivery_enabled,false);
 assert.equal(c.body.alerts_configuration.thresholds.severities.critical.destination,'xyne_test');
 assert(c.blockers.some(b=>b.includes('Manual')));
});
test('pending and no-data policies stay visible and cannot accidentally enable sends',()=>{
 const r=fixture();r.for='3h';r.noDataState='NoData';
 const c=convertRule(r,'xyne_test');
 assert.equal(c.body.alerts_configuration.query.pending_seconds,10800);
 assert.equal(c.body.alerts_configuration.query.delivery_enabled,false);
 assert(c.blockers.some(b=>b.includes('pending')));
 assert(c.blockers.some(b=>b.includes('no-data')));
});
test('unknown and sub-minute source cadences are not silently rounded',()=>{
 const r=fixture();delete r.intervalSeconds;
 assert.throws(()=>convertRule(r,'xyne_test'),/cadence/);
 assert.throws(()=>convertRule(r,'xyne_test',10),/cadence/);
});
test('unsupported graphs and threshold parameter sets are not guessed',()=>{
 const r=fixture();r.data[2].model.conditions[0].evaluator.params=[0,0];
 assert.throws(()=>convertRule(r,'xyne_test'),/parameters/);
 const r2=fixture();r2.data[1].model.type='math';
 assert.throws(()=>convertRule(r2,'xyne_test'),/reduction/);
});
