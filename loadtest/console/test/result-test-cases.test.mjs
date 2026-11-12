import test from 'node:test';import assert from 'node:assert/strict';
import {testCaseCatalog,testCasePlan,resultTestCases} from '../src/result-test-cases.mjs';
const check=(id,actual=10,outcome='pass')=>({id,label:id,actual,limit:60,comparison:'≤',unit:'s',outcome});
const hold=(index,intent)=>({id:`stage-${index}`,stageIndex:index,kind:'hold',state:'complete',title:'Hold',checks:[check('throughput',95),check('p99',80)]});
const fault=index=>({id:`fault-${index}`,stageIndex:index,kind:'fault',state:'complete',title:'Database failover',checks:[check('recovery',20),{...check('failed',4,'recorded'),limit:null}]});
const config={load:{stages:[{}, {fault:{targetId:'postgres',action:'failover'}},{fault:{targetId:'card-vault'}}]},faultPlan:[{index:1,action:'failover',target:{id:'postgres'}},{index:2,target:{id:'card-vault'}}]};
test('all ten original test cases belong to the two finalized sections',()=>{
 assert.equal(testCaseCatalog.length,10);assert.equal(new Set(testCaseCatalog.map(t=>t.id)).size,10);
 assert.deepEqual(testCaseCatalog.filter(t=>t.section==='load').map(t=>t.id),['TC-01','TC-02','TC-03','TC-09']);
 assert.equal(testCaseCatalog.filter(t=>t.section==='faults').length,6);
});
test('current hold + DB + vault run maps to original TCs; ramps do not imply a burst',()=>{
 const groups=[hold(0),fault(1),fault(2),{id:'stage-3',stageIndex:3,kind:'ramp',state:'complete',checks:[check('throughput',99)]}];
 const plan=testCasePlan(config,groups);assert.deepEqual(plan.map(p=>p.id),['TC-02','TC-04','TC-05','TC-10']);
 const sections=resultTestCases({groups,testCasePlan:plan});assert.deepEqual(sections.map(s=>s.title),['Load & performance','Faults & recovery']);
 assert.ok(sections[0].cases.some(c=>c.title==='Ramp observations'));assert.ok(!sections[0].cases.some(c=>c.id==='TC-03'));
});
test('repeated failovers stay in one TC and retain every original observation',()=>{
 const groups=[fault(1),fault(2)];groups[1].checks[0].actual=70;groups[1].checks[0].outcome='fail';
 const cfg={load:{},faultPlan:[{index:1,action:'failover',target:{id:'postgres'}},{index:2,action:'failover',target:{id:'postgres'}}]};
 const before=JSON.stringify(groups),sections=resultTestCases({groups,testCasePlan:testCasePlan(cfg,groups)});
 const tc=sections[0].cases.find(c=>c.id==='TC-04');assert.equal(tc.occurrences,2);assert.equal(tc.outcome,'fail');assert.match(tc.observed,/70 s recovery/);assert.equal(tc.checks.filter(c=>c.id==='recovery').length,2);assert.equal(JSON.stringify(groups),before);
});
test('fault failures have a separate total line and recovery summaries show only time',()=>{
 const groups=[fault(1),fault(2)];
 const rows=resultTestCases({groups,testCasePlan:[{id:'TC-04',groupIds:groups.map(g=>g.id)}]}).flatMap(s=>s.cases);
 const row=rows.find(c=>c.id==='TC-04');
 assert.equal(row.observed,'20 s recovery');
 assert.equal(row.checks.find(c=>c.id==='total-failed').actual,8);
 assert.equal(row.checks.find(c=>c.id==='total-failed').label,'Total failed requests');
 assert.equal(row.checks.some(c=>c.id==='failed'),false);
});
test('reconnection and merchant coverage require evidence; vault checks use measured criteria',()=>{
 const groups=[hold(0),fault(1),fault(2)],cfg={...config,mode:'tenancy',fixtureSet:{merchantCount:1500}};
 const rows=resultTestCases({groups,testCasePlan:testCasePlan(cfg,groups)}).flatMap(s=>s.cases);
 for(const id of ['TC-05','TC-09'])assert.equal(rows.find(r=>r.id===id).outcome,'not_measured');
 const reconnect=rows.find(r=>r.id==='TC-05');
 assert.ok(reconnect.checks.every(c=>c.id!=='pod-restarts'));
 assert.doesNotMatch(JSON.stringify(reconnect),/pod restarts|restart evidence/i);
 assert.equal(rows.find(r=>r.id==='TC-10').outcome,'pass');
 assert.ok(rows.find(r=>r.id==='TC-10').checks.every(c=>!['unattended','unaffected-flows'].includes(c.id)));
 assert.match(rows.find(r=>r.id==='TC-09').observed,/1,500 merchants configured/);
});
test('application reconnection is measured from resumed successful confirmations, with no pod checks',()=>{
 const recovery={id:'recovery-1',kind:'recovery',state:'complete',checks:[{id:'completed',actual:104,limit:null,outcome:'recorded'}]};
 const summary={groups:[fault(1),recovery],testCasePlan:[{id:'TC-05',groupIds:['fault-1','recovery-1']}]};
 const row=resultTestCases(summary).flatMap(s=>s.cases)[0];
 assert.equal(row.outcome,'pass');assert.equal(row.checks[0].actual,104);
 assert.match(row.observed,/104 successful confirmations/);assert.doesNotMatch(JSON.stringify(row),/not measured|pod-restarts/i);
 recovery.checks[0].actual=0;assert.equal(resultTestCases(summary).flatMap(s=>s.cases)[0].outcome,'fail');
});
test('calibration and burst require explicit intent; unsupported selected cases are Not run',()=>{
 const groups=[hold(0),hold(1)],cfg={load:{stages:[{intent:'calibration'},{intent:'burst'}],testCases:['TC-07','TC-08']}};
 const plan=testCasePlan(cfg,groups);assert.deepEqual(plan.map(p=>p.id),['TC-01','TC-03','TC-07','TC-08']);
 const rows=resultTestCases({groups,testCasePlan:plan}).flatMap(s=>s.cases);
 assert.equal(rows.find(r=>r.id==='TC-01').outcome,'not_measured');
 for(const id of ['TC-07','TC-08'])assert.equal(rows.find(r=>r.id===id).outcome,'not_run');
});
test('unexecuted fault cases stay Not run even if run-wide integrity evidence exists',()=>{
 const groups=[{...fault(1),state:'not_run'}, {id:'integrity',kind:'integrity',checks:[check('lost',0)]}];
 const rows=resultTestCases({groups,testCasePlan:testCasePlan(config,groups)}).flatMap(s=>s.cases);
 assert.ok(rows.every(r=>r.outcome==='not_run'));
});
test('a DB stop is not a failover and a burst-labelled ramp is not a burst hold',()=>{
 const groups=[fault(1),{...hold(0),kind:'ramp'}];
 const cfg={load:{stages:[{intent:'burst'},{fault:{targetId:'postgres',action:'outage'}}]},faultPlan:[{index:1,action:'outage',target:{id:'postgres'}}]};
 assert.deepEqual(testCasePlan(cfg,groups),[]);
});
test('a hidden latency failure has an explicit reason even when throughput passes',()=>{
 const group=hold(0);group.checks=[{...check('throughput',99.5),unit:'%',limit:97,comparison:'≥'},{...check('p99',556.7,'fail'),unit:'ms',limit:160}];
 const tc=resultTestCases({groups:[group]}).flatMap(s=>s.cases)[0];
 assert.equal(tc.outcome,'fail');assert.equal(tc.failureReason,'Steady p99 exceeds 160 ms');
 const pooled={checks:group.checks.map(c=>c.id==='p99'?{...c,actual:102,outcome:'pass'}:c)};
 const corrected=resultTestCases({groups:[group],steady:pooled}).flatMap(s=>s.cases)[0];
 assert.equal(corrected.outcome,'pass');assert.equal(corrected.failureReason,null);
});
