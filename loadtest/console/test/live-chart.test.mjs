import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {compile} from 'svelte/compiler';import {render} from 'svelte/server';
const source=await readFile(new URL('../src/LiveChart.svelte',import.meta.url),'utf8');
const compiled=compile(source,{generate:'server',filename:'LiveChart.svelte'});
let js=compiled.js.code.replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`).replace("'./chart-timeline.mjs'",JSON.stringify(new URL('../src/chart-timeline.mjs',import.meta.url).href));
const {default:Chart}=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));
test('full horizon and planned stages render before and during execution',()=>{
 const timeline=[{start:0,end:10,from:10,target:10,kind:'hold'},{start:10,end:15,from:10,target:10,kind:'fault',fault:{target:'PostgreSQL',action:'outage'}}];
 const empty=render(Chart,{props:{durationSeconds:15,timeline}}).body;assert.match(empty,/full 15 second test/);assert.match(empty,/Not run yet/);assert.match(empty,/PostgreSQL · outage/);assert.match(empty,/recovery begins \(planned\)/);assert.match(empty,/width="900" height="185" fill="#f0f2f5"/);
 const live=render(Chart,{props:{durationSeconds:15,timeline,points:[{t:8,activeSeconds:5,tps:10}],activity:[{at:8000,kind:'fault_restored',message:'Recovery verified'}]}}).body;assert.match(live,/full 15 second test/);assert.match(live,/width="600" height="185" fill="#f0f2f5"/);assert.match(live,/Recovery verified/);assert.match(live,/stroke="#398453"/);
});
