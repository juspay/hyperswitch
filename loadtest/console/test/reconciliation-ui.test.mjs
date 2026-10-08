import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';
const source=await readFile(new URL('../src/ReconciliationSummary.svelte',import.meta.url),'utf8');
const compiled=compile(source,{generate:'server',filename:'ReconciliationSummary.svelte'});
const js=compiled.js.code.replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`);
const {default:Component}=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));
test('summary renders real outcome history, counts, and legacy/failed states',()=>{
 const result={total:10,expected:10,match:7,missing:2,mismatch:1,timeline:[{t:0,match:0,missing:0,mismatch:0},{t:2,match:7,missing:2,mismatch:1}],elapsedSeconds:2};
 const html=render(Component,{props:{result}}).body;
 assert.match(html,/Comparison complete/);assert.match(html,/70.0%/);assert.match(html,/3 discrepancies found/);assert.match(html,/stroke="#398453"/);assert.match(html,/stroke="#c43b36"/);assert.match(html,/stroke="#bd7c21"/);assert.match(html,/comparison progress|single batch/);assert.ok(html.indexOf('<svg')<html.indexOf('class="outcomes'));
 const legacy=render(Component,{props:{result:{...result,timeline:undefined}}}).body;assert.match(legacy,/no recorded timeline/);assert.doesNotMatch(legacy,/<svg/);
 const failed=render(Component,{props:{result,job:{status:'failed',error:'Database unavailable',progress:{compared:0}}}}).body;assert.match(failed,/Comparison failed/);assert.match(failed,/Database unavailable/);assert.doesNotMatch(failed,/Comparison complete|discrepancies found/);
 const live=render(Component,{props:{running:true,job:{progress:{compared:4,expected:10,match:4}}}}).body;assert.match(live,/Comparing payments/);assert.match(live,/<progress/);assert.match(live,/4 \/ 10 compared/);
});
