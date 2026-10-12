import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';import {compile} from 'svelte/compiler';import {render} from 'svelte/server';
const source=await readFile(new URL('../src/LiveChart.svelte',import.meta.url),'utf8');
const compiled=compile(source,{generate:'server',filename:'LiveChart.svelte'});
let js=compiled.js.code.replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`).replace("'./chart-timeline.mjs'",JSON.stringify(new URL('../src/chart-timeline.mjs',import.meta.url).href));
const {default:Chart}=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));
test('full horizon and planned stages render before and during execution',()=>{
 const timeline=[{start:0,end:10,from:10,target:10,kind:'hold'},{start:10,end:15,from:10,target:10,kind:'fault',fault:{target:'PostgreSQL',action:'outage'}}];
 const empty=render(Chart,{props:{durationSeconds:15,timeline}}).body;assert.match(empty,/full 15 second test/);assert.match(empty,/Not run yet/);assert.match(empty,/PostgreSQL · fault planned/);assert.match(empty,/recovery planned/);const unrun=empty.match(/<rect x="([\d.]+)" y="35" width="([\d.]+)" height="185" fill="#f0f2f5"/);assert.ok(unrun);
 const activity=[{at:8000,kind:'fault_requested',message:'Fault requested'},{at:9000,kind:'fault_started',message:'Fault confirmed'},{at:11000,kind:'fault_restore',message:'Recovery requested'},{at:13000,kind:'fault_restored',message:'Recovery verified'}];
 const live=render(Chart,{props:{durationSeconds:15,timeline,points:[{t:8,activeSeconds:5,tps:10}],activity}}).body;assert.match(live,/full 18 second test/);const future=live.match(/<rect x="([\d.]+)" y="35" width="([\d.]+)" height="185" fill="#f0f2f5"/);assert.ok(future);assert.ok(Math.abs(Number(future[2])/Number(unrun[2])-10/18)<1e-6);assert.ok(Math.abs(Number(future[1])+Number(future[2])-Number(unrun[1])-Number(unrun[2]))<1e-6);assert.match(live,/Fault requested/);assert.match(live,/Fault confirmed/);assert.match(live,/Recovery requested/);assert.match(live,/Recovery verified/);assert.match(live,/stroke="#398453"/);
 const errors=render(Chart,{props:{durationSeconds:15,timeline,points:[{t:8,activeSeconds:5,errors:100}],metric:'errors',activity}}).body;assert.match(errors,/full 18 second test/);
});
test('active run status renders inside the timeline card',()=>{
 const status={state:'running',smoke:false,label:'Single · 5 TPS',clock:'20 / 85s active',pending:false,faultActive:false,faultBlocked:false,cancel:()=>{},pause:()=>{},resume:()=>{},createFault:()=>{}};
 const page=render(Chart,{props:{durationSeconds:85,runStatus:status}}).body;
 assert.match(page,/Single · 5 TPS/);assert.match(page,/20 \/ 85s active/);assert.match(page,/Ⅱ Pause/);assert.match(page,/>Stop</);assert.doesNotMatch(page,/Create fault|>More</);
});

test('stopping is visible and disables duplicate cancellation while cleanup continues',()=>{
 const page=render(Chart,{props:{runStatus:{state:'stopping',live:true,stopping:true,label:'9 TPS',clock:'40 / 121s active',cancel:()=>{}}}}).body;
 assert.match(page,/Stopping…/);assert.match(page,/<button[^>]*disabled[^>]*>Stopping…<\/button>/);
 assert.match(page,/Stop requested/);assert.doesNotMatch(page,/Ⅱ Pause|Run again|View results/);
});
test('load actions appear beside the card title before the status row',()=>{
 for(const status of [{state:'running',live:true,pause:()=>{},cancel:()=>{}},{state:'cancelled',live:false,runAgain:()=>{},resultsUrl:'/results?run=test'}]){
  const page=render(Chart,{props:{runStatus:{label:'9 TPS',clock:'40 / 121s active',...status}}}).body;
  assert.ok(page.indexOf('Load timeline')<page.indexOf('class="actions'));
  assert.ok(page.indexOf('class="actions')<page.indexOf('timeline-run-toolbar'));
 }
});

test('ready actions share the timeline card header',()=>{
 const page=render(Chart,{props:{runStatus:{state:'idle',live:true,label:'',clock:'141s · up to 9 TPS',startLoad:()=>{},smokeTest:()=>{}}}}).body;
 assert.match(page,/Run smoke test/);assert.match(page,/Start load/);assert.match(page,/>Ready</);
 assert.ok(page.indexOf('Start load')<page.indexOf('timeline-run-toolbar'));
});
test('flow-rate samples cover their measured intervals from zero',()=>{
 const page=render(Chart,{props:{durationSeconds:10,points:[{t:1,tps:0},{t:2,tps:1}]}}).body;
 assert.match(page,/d="M48\.0,220\.0 L131\.6,220\.0 L131\.6,[\d.]+ L215\.2,[\d.]+" fill="none" stroke="#0561e2"/);
});
test('failover request renders a dashed marker alongside observed DB availability',()=>{
 const activity=[{at:2000,kind:'fault_requested',message:'PostgreSQL · failover requested'},
  {at:4000,kind:'database_unavailable',message:'DB · unavailable'},
  {at:6000,kind:'database_available',message:'DB · available'}];
 const page=render(Chart,{props:{durationSeconds:10,startedAt:0,activity}}).body;
 assert.match(page,/stroke="#5277b8" stroke-dasharray="4 3"[^>]*><title>2\.0s · PostgreSQL · failover requested/);
 assert.match(page,/4\.0s · DB · unavailable/);assert.match(page,/6\.0s · DB · available/);
});
