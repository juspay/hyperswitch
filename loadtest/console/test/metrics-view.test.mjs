import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';
async function component(name,dependencies={}){
 const file=new URL(`../src/${name}`,import.meta.url);
 const js=compile(await readFile(file,'utf8'),{generate:'server',filename:name}).js.code
  .replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`)
  .replace(/from ['"](\.[^'"]+)['"]/g,(_,relative)=>`from '${dependencies[relative]||new URL(relative,file).href}'`);
 return 'data:text/javascript;base64,'+Buffer.from(js).toString('base64');
}
const panel=await component('GrafanaPanel.svelte');
const graphs=await component('GrafanaMetrics.svelte',{'./GrafanaPanel.svelte':panel});
const {default:View}=await import(await component('MetricsView.svelte',{'./GrafanaMetrics.svelte':graphs}));
const props={run:{startedAt:1000,endedAt:301000},grafana:{hasIdentity:true,baseUrl:'http://localhost:3002'}};
test('native mode does not load Grafana iframes',()=>{
 const html=render(View,{props}).body;
 assert.match(html,/Metrics source/);assert.doesNotMatch(html,/<iframe|<details/);
});
test('Grafana mode shows three panels without duplicate wrapper headings or a dropdown',()=>{
 const html=render(View,{props:{...props,value:'grafana'}}).body;
 assert.equal((html.match(/<iframe /g)||[]).length,3);
 assert.doesNotMatch(html,/<h3|<details/);assert.match(html,/Open ↗/);
});
