import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';
const source=await readFile(new URL('../src/RunSummary.svelte',import.meta.url),'utf8');
const compiled=compile(source,{generate:'server',filename:'RunSummary.svelte'});
const js=compiled.js.code.replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`);
const {default:Summary}=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));
const run={startedAt:1000,endedAt:2000,activeSeconds:141,wallSeconds:234,mode:'single',offeredTps:9,persisted:6000,iterations:1475,serverP99:1065.27,serverLatencyPhases:[{state:'normal',count:100,p50:40,p95:60,p99:80},{state:'outage',count:10,p50:30,p95:50,p99:70},{state:'recovery',count:20,p50:90,p95:1000,p99:1500}],profile:{version:12},fixtureSetId:'8eb569d4-fixtures',savedSettings:{limitsSaved:true},hiddenVaultErrors:101,displayErrorRate:1.32,acceptance:{verdict:'fail',resilience:'recovery verified',checks:[{label:'Successful TPS',actual:9.16,limit:6.30638,outcome:'pass'},{label:'HTTP errors (%)',actual:2.99,limit:1,outcome:'fail'}]}};
test('saved checks show their limits, raw observations, and final verdict',()=>{
 const html=render(Summary,{props:{run}}).body;
 assert.match(html,/Failed/);assert.match(html,/2\.99%/);assert.doesNotMatch(html,/1\.32%/);
 assert.match(html,/≥ 6\.31 TPS/);assert.match(html,/≤ 1%/);assert.doesNotMatch(html,/vault-related failures|Fault recovery/);assert.equal((html.match(/<tr>/g)||[]).length,8);
 assert.match(html,/Server-side latency/);assert.match(html,/p50 · ms/);assert.match(html,/p95 · ms/);assert.match(html,/p99 · ms/);assert.match(html,/After recovery · first 10s/);assert.match(html,/Completed flows/);assert.doesNotMatch(html,/Client-side latency|Dropped flows|Recording gaps/);assert.match(html,/Scenario v12/);
});
test('live and legacy runs do not claim a final saved verdict',()=>{
 assert.match(render(Summary,{props:{run:{...run,live:true}}}).body,/In progress/);
 const html=render(Summary,{props:{run:{...run,savedSettings:{limitsSaved:false}}}}).body;
 assert.doesNotMatch(html,/Failed|Passed|≤ 1%/);assert.match(html,/Recorded/);
});
