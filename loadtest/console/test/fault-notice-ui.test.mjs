import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {compile} from 'svelte/compiler';
import {render} from 'svelte/server';
const file=new URL('../src/FaultNotice.svelte',import.meta.url);
const code=compile(await readFile(file,'utf8'),{generate:'server',filename:'FaultNotice.svelte'}).js.code
 .replace(/from ['"](svelte[^'"]*)['"]/g,(_,name)=>`from '${import.meta.resolve(name)}'`)
 .replace("'./fault-notice.mjs'",JSON.stringify(new URL('../src/fault-notice.mjs',import.meta.url).href));
const {default:Notice}=await import('data:text/javascript;base64,'+Buffer.from(code).toString('base64'));
test('recovery notice separates its title, explanation and retry action',()=>{
 const faults={active:{target:'Sandbox RDS',action:'failover',status:'recovery_failed',databaseAvailability:'available'},blocked:true,recovering:false};
 const html=render(Notice,{props:{faults,databaseHealth:{state:'online',checkedAt:Date.now()}}}).body;
 assert.match(html,/<strong[^>]*>AWS RDS<\/strong>/);assert.match(html,/>Available<\/span>/);assert.match(html,/Failover confirmation pending/);assert.match(html,/>Recheck<\/button>/);assert.doesNotMatch(html,/Sandbox|recovery_failed|next stage waits/);
 const pending=render(Notice,{props:{faults:{...faults,recovering:true}}}).body;
 assert.match(pending,/<button[^>]*disabled/);assert.match(pending,/Checking…/);
 assert.doesNotMatch(render(Notice,{props:{faults:{active:null,blocked:false}}}).body,/fault-notice|button/);
});
