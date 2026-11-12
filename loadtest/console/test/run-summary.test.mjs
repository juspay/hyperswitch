import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';
const source=await readFile(new URL('../src/RunSummary.svelte',import.meta.url),'utf8');
const compiled=compile(source,{generate:'server',filename:'RunSummary.svelte'});
const overviewUrl=new URL('../src/result-test-cases.mjs',import.meta.url).href;
const child=compile(await readFile(new URL('../src/ScenarioChecks.svelte',import.meta.url),'utf8'),{generate:'server',filename:'ScenarioChecks.svelte'}).js.code.replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`).replace("'./result-test-cases.mjs'",JSON.stringify(overviewUrl)).replace("'./flow-catalog.mjs'",JSON.stringify(new URL('../src/flow-catalog.mjs',import.meta.url).href));
const childUrl='data:text/javascript;base64,'+Buffer.from(child).toString('base64');
const js=compiled.js.code.replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`).replace("'./environments.mjs'",JSON.stringify(new URL('../src/environments.mjs',import.meta.url).href)).replace("'./ScenarioChecks.svelte'",JSON.stringify(childUrl)).replace("'./result-test-cases.mjs'",JSON.stringify(overviewUrl));
const {default:Summary}=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));
const run={startedAt:1000,endedAt:2000,activeSeconds:141,wallSeconds:234,mode:'single',offeredTps:9,persisted:6000,iterations:1475,serverP99:1065.27,serverLatencyPhases:[{state:'normal',count:100,p50:40,p95:60,p99:80},{state:'outage',count:10,p50:30,p95:50,p99:70},{state:'recovery',count:20,p50:90,p95:1000,p99:1500}],profile:{version:12},fixtureSetId:'8eb569d4-fixtures',savedSettings:{limitsSaved:true},hiddenVaultErrors:101,displayErrorRate:1.32,acceptance:{verdict:'fail',resilience:'recovery verified',checks:[{label:'Successful TPS',actual:9.16,limit:6.30638,outcome:'pass'},{label:'HTTP errors (%)',actual:2.99,limit:1,outcome:'fail'}]}};
test('saved checks show their limits, raw observations, and final verdict',()=>{
 const html=render(Summary,{props:{run}}).body;
 assert.match(html,/Failed/);assert.match(html,/2\.99%/);assert.doesNotMatch(html,/1\.32%/);
 assert.match(html,/≥ 6\.31 TPS/);assert.match(html,/≤ 1%/);assert.doesNotMatch(html,/vault-related failures|Fault recovery/);assert.equal((html.match(/<tr>/g)||[]).length,8);
 assert.match(html,/Server-side latency/);assert.match(html,/p50 · ms/);assert.match(html,/p95 · ms/);assert.match(html,/p99 · ms/);assert.match(html,/After recovery · first 10s/);assert.match(html,/Completed flows/);assert.doesNotMatch(html,/Client-side latency|Dropped flows|Recording gaps/);assert.doesNotMatch(html,/Scenario v12|Fixture 8eb569d4/);
});
test('live and legacy runs do not claim a final saved verdict',()=>{
 assert.match(render(Summary,{props:{run:{...run,live:true}}}).body,/In progress/);
 const html=render(Summary,{props:{run:{...run,savedSettings:{limitsSaved:false}}}}).body;
 assert.doesNotMatch(html,/Failed|Passed|≤ 1%/);assert.match(html,/Recorded/);
});
test('scenario stages render checks with explicit missing and failed verdicts',()=>{
 const scenarioChecks={verdict:'fail',passed:1,evaluated:2,groups:[{id:'stage-0',kind:'hold',title:'1. Hold · 10 TPS',verdict:'fail',latencySamples:12,checks:[
  {id:'throughput',label:'Achieved vs offered throughput',actual:95,limit:97,unit:'%',comparison:'≥',outcome:'fail'},
  {id:'p99',label:'Server-side latency · p99',actual:80,limit:160,unit:'ms',comparison:'≤',outcome:'pass'},
  {id:'errors',label:'HTTP errors',actual:null,limit:1,unit:'%',comparison:'≤',outcome:'not_measured'}]}]};
 const html=render(Summary,{props:{run:{...run,scenarioChecks}}}).body;
 assert.match(html,/0\/1 test cases passed/);assert.match(html,/Load &amp; performance|Load & performance/);assert.match(html,/≥ 97%/);assert.match(html,/95%/);assert.match(html,/>Fail</);assert.doesNotMatch(html,/≥ 6\.31 TPS|Hold · 10 TPS/);
});
test('scenario-derived latency phases take precedence over legacy presentation phases',()=>{
 const scenarioChecks={verdict:'recorded',groups:[],states:[{state:'normal',count:10,p50:9999,p95:9999,p99:9999}]};
 const html=render(Summary,{props:{run:{...run,scenarioChecks}}}).body;
 assert.match(html,/9,999/);assert.doesNotMatch(html,/>40<|>60<|>80</);
});
test('steady-load details show one latency set without case IDs or repeated stages',()=>{
 const checks=[{id:'throughput',label:'Achieved vs offered throughput',actual:98,limit:97,unit:'%',comparison:'≥',outcome:'pass'},{id:'p99',label:'Server-side latency · p99',actual:9999,unit:'ms',outcome:'recorded'}];
 const scenarioChecks={verdict:'pass',steady:{checks:[checks[0]]},groups:[0,2].map(i=>({id:`stage-${i}`,kind:'hold',title:`${i+1}. Hold · 6 TPS`,state:'complete',checks}))};
 const html=render(Summary,{props:{run:{...run,scenarioChecks}}}).body;
 assert.match(html,/Steady-load performance/);assert.equal((html.match(/p99 · ms/g)||[]).length,1);assert.match(html,/Steady load/);
 assert.doesNotMatch(html,/TC-\d+|Hold ·|Server-side latency · p99|9,999|9999/);
 assert.match(html,/>40</);assert.match(html,/>60</);assert.match(html,/>80</);
});

test('measurement-only rows retain values without claiming a recorded verdict or limit',()=>{
 const checks=[
  {id:'throughput',label:'Achieved vs offered throughput',actual:99.88,limit:97,unit:'%',comparison:'≥',outcome:'pass'},
  {id:'tps',label:'Successful TPS',actual:6.65,limit:null,unit:'TPS',outcome:'recorded'},
  {id:'completed',label:'Completed flows',actual:2397,limit:null,unit:'flows',outcome:'recorded'}
 ];
 const scenarioChecks={verdict:'pass',steady:{checks},groups:[{id:'stage-0',kind:'hold',state:'complete',title:'Hold',checks}]};
 const html=render(Summary,{props:{run:{...run,scenarioChecks}}}).body;
 assert.match(html,/6\.65 TPS/);assert.match(html,/2,397 flows/);assert.match(html,/>Pass</);
 assert.doesNotMatch(html,/>Recorded</);assert.match(html,/>—</);
});

test('run-wide target completion is a separate strict check from window throughput',()=>{
 const completion={expected:4200,successful:4200,check:{id:'completion',label:'Target flows completed successfully',actual:100,limit:100,unit:'%',comparison:'≥',outcome:'pass'}};
 const scenarioChecks={verdict:'pass',completion,groups:[]};
 const html=render(Summary,{props:{run:{...run,scenarioChecks}}}).body;
 assert.match(html,/Flow completion/);assert.match(html,/4,200 \/ 4,200 flows · 100%/);assert.match(html,/1\/1 test cases passed/);assert.match(html,/100% of target flows/);
 const failed={...completion,successful:4199,check:{...completion.check,actual:100*4199/4200,outcome:'fail'}};
 const failure=render(Summary,{props:{run:{...run,scenarioChecks:{...scenarioChecks,verdict:'fail',completion:failed}}}}).body;
 assert.match(failure,/Failed/);assert.match(failure,/0\/1 test cases passed/);
});

test('missing payment integrity explicitly requires reconciliation and updates with evidence',()=>{
 const fault={id:'fault-1',kind:'fault',target:'Card vault',state:'complete',checks:[{id:'recovery',label:'Recovery after restoration starts',actual:54.414,limit:60,unit:'s',comparison:'≤',outcome:'pass'}]};
 const make=verified=>({...run,scenarioChecks:{verdict:verified?'pass':'incomplete',testCasePlan:[{id:'TC-10',groupIds:['fault-1','integrity']}],groups:[fault,{id:'integrity',kind:'integrity',checks:[{id:'lost',label:'Acknowledged payments lost',actual:verified?0:null,limit:0,unit:'payments',comparison:'≤',outcome:verified?'pass':'not_measured'}]}]}});
 const pending=render(Summary,{props:{run:make(false)}}).body;
 assert.match(pending,/Reconciliation required/);assert.doesNotMatch(pending,/Pending verification/);
 const verified=render(Summary,{props:{run:make(true)}}).body;
 assert.match(verified,/0 payments/);assert.doesNotMatch(verified,/Reconciliation required|Pending verification/);
});
