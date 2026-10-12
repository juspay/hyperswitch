import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';
import path from 'node:path';
import os from 'node:os';
import { fixtures,load } from '../run.mjs';
import { FixtureRecorder } from '../sqlite.mjs';
import { configureSuperposition } from '../superposition.mjs';

import { mockServer } from "./mock-server.mjs";

const temporary = () => mkdtemp(path.join(os.tmpdir(),'loadtest-fixtures-'));
async function configuration(dir,url,extras={}) {
 const cfg={router:url,admin_api_key:'admin-test',merchant_count:2,concurrency:2,merchant_id_prefix:'test',retry:{attempts:1,backoff_ms:1},
  connector:{connector_name:'stripe',connector_account_details:{auth_type:'HeaderKey',api_key:'test-key'},business_country:'US',business_label:'default'},
  superposition:{endpoint:url,token:'sp-test',organization_id:'sp-org',workspace_id:'test',refresh_interval_seconds:0,defaults:{flag:true}},...extras};
 const file=path.join(dir,'provision.json');await writeFile(file,JSON.stringify(cfg));return file;
}
test('fixtures are recorded, ready, and resumable without minting extra API keys',async() => {
 const {server,state,url}=await mockServer(); const dir=await temporary();
 try {
  const cfg=await configuration(dir,url),output=path.join(dir,'fixtures');
  const setup=[];await fixtures(cfg,output,{onProgress:value=>setup.push(structuredClone(value))});const keys=state.keys;
  assert.equal(setup[0].stage,"merchants");assert.equal(setup.at(-1).stage,"ready");assert.ok(setup.some(s=>s.superposition.status==="applying"));assert.ok(setup.some(s=>s.superposition.status==="verifying"));
  const patches=state.patches;
  await fixtures(cfg,output); assert.equal(state.keys,keys);assert.equal(state.patches,patches);
  const manifest=JSON.parse(await readFile(path.join(output,'manifest.json'),'utf8'));
  assert.equal(manifest.state,'ready');assert.equal(manifest.merchant_count,2);assert.equal(manifest.superposition.version,'v1');
  const db=new DatabaseSync(path.join(output,'fixtures.sqlite'),{readOnly:true});
  assert.equal(db.prepare('SELECT COUNT(*) n FROM request_events').get().n,state.requests);
  assert.equal(db.prepare("SELECT COUNT(*) n FROM runs WHERE recording_state='complete'").get().n,2);db.close();
 } finally {server.close();}
});
test('Superposition changes only default values and skips unchanged defaults',async() => {
 const {server,state,url}=await mockServer();const dir=await temporary(),recorder=new FixtureRecorder(path.join(dir,'fixtures.sqlite'));
 const cfg={endpoint:url,token:'token',organization_id:'sp-org',workspace_id:'test',refresh_interval_seconds:0,defaults:{flag:true}};
 try {
  state.default=false;
  await configureSuperposition(cfg,recorder,[{merchant_id:'m',profile_id:'p'}],'o');assert.equal(state.patches,1);
  await configureSuperposition(cfg,recorder,[{merchant_id:'m',profile_id:'p'}],'o');assert.equal(state.patches,1);
 } finally {recorder.close();server.close();}
});

test('Superposition supports its access header authentication mode',async()=>{
 const requests=[];const server=(await import('node:http')).createServer((req,res)=>{requests.push(req.headers);res.writeHead(200,{'content-type':'application/json','x-config-version':'v1'});res.end(req.url==='/config/resolve'?'{"flag":true}':'{"value":true}');});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const recorder=new FixtureRecorder(path.join(await temporary(),'fixtures.sqlite'));
 try{await configureSuperposition({endpoint:`http://127.0.0.1:${server.address().port}`,token:'access-token',access_header:'x-superposition-access',organization_id:'org',workspace_id:'dev',refresh_interval_seconds:0,defaults:{flag:true}},recorder,[{merchant_id:'m',profile_id:'p'}],'org');assert.ok(requests.every(headers=>headers['x-superposition-access']==='access-token'&&!headers.authorization));}
 finally{recorder.close();server.close();}
});
test('partial connector failure resumes using checkpointed API keys and invalidates readiness',async() => {
 const {server,state,url}=await mockServer();const dir=await temporary();
 try {
  const cfg=await configuration(dir,url),output=path.join(dir,'fixtures');state.failConnector=true;
  await assert.rejects(fixtures(cfg,output),/fixtures failed/);assert.equal(state.keys,2);
  assert.equal(JSON.parse(await readFile(path.join(output,'manifest.json'),'utf8')).state,'failed');
  state.failConnector=false;await fixtures(cfg,output);assert.equal(state.keys,2);
 } finally {server.close();}
});
test('Superposition rejects unknown flags before writes and detects context conflicts',async() => {
 const {server,state,url}=await mockServer();const dir=await temporary(),recorder=new FixtureRecorder(path.join(dir,'fixtures.sqlite'));
 const cfg={endpoint:url,token:'token',organization_id:'sp-org',workspace_id:'test',refresh_interval_seconds:0,defaults:{flag:true,unknown:true}};
 try {
  await assert.rejects(configureSuperposition(cfg,recorder,[{merchant_id:'m',profile_id:'p'}],'o'),/404/);assert.equal(state.patches,0);
  cfg.defaults={flag:true};state.conflict=true;
  await assert.rejects(configureSuperposition(cfg,recorder,[{merchant_id:'m',profile_id:'p'}],'o'),/override conflict/);
 } finally {recorder.close();server.close();}
});
test('recorded k6 load covers all scenario paths and drains every response', {skip: !process.env.K6_SQLITE_BINARY},async() => {
 const {server,state,url}=await mockServer();const dir=await temporary();
 try {
  const cfg=await configuration(dir,url),output=path.join(dir,'fixtures');await fixtures(cfg,output);
  const before=state.requests;
  const scenarios=[['guest','non_modular'],['guest','modular'],['cit_on_session','non_modular'],['cit_off_session','modular'],['ptv_on_session','modular'],['ptv_off_session','modular'],['cit_metadata_changed','non_modular'],['sdk_checkout','non_modular'],['mit','non_modular'],['saved_card_checkout','non_modular']]
   .map(([scenario,merchant_path],i)=>({name:`s${i}`,scenario,merchant_path,weight:10}));
  const config={services:{router:url,modular_pm:`${url}/v2`},load:{total_rps:20,duration_seconds:2},payment:{amount:100,currency:'USD',card:{card_number:'4242424242424242',card_exp_month:'10',card_exp_year:'35',card_cvc:'123'},metadata_update:{card_exp_month:'11'}},scenarios};
  const configPath=path.join(dir,'load.json');await writeFile(configPath,JSON.stringify(config));
  const result=await load(configPath,output,path.join(dir,'run'),process.env.K6_SQLITE_BINARY);assert.equal(result.exitCode,0);
  const summary=JSON.parse(await readFile(path.join(dir,'run','summary.json'),'utf8'));
  assert.ok(summary.metrics.http_reqs.values.count>100);
  assert.equal(summary.metrics.payment_confirm_server_latency_ms.values['p(99)'],12);assert.ok(state.latencyOptIns>0);
  const db=new DatabaseSync(result.database,{readOnly:true});
  const events=db.prepare('SELECT COUNT(*) n FROM request_events').get().n;
  assert.equal(events,state.requests-before);assert.ok(events>100);
  assert.equal(db.prepare('SELECT MIN(api_status_code) n FROM request_events').get().n,200);
  assert.equal(db.prepare("SELECT MIN(server_latency_ms) n FROM request_events WHERE operation='payment_confirm'").get().n,12);
  assert.ok(db.prepare('SELECT AVG(latency_ms) n FROM request_events').get().n>0);
  const operations=db.prepare('SELECT DISTINCT operation FROM request_events').all().map(r=>r.operation);
  for(const operation of ['customer_create','pm_session_create','pm_session_confirm','payment_create','payment_confirm','baseline_create','baseline_confirm','payment_method_list','payment_method_list_poll','baseline_poll','session','eligibility']) assert.ok(operations.includes(operation),operation);
  assert.ok(db.prepare("SELECT COUNT(*) n FROM payments WHERE status='succeeded'").get().n>0);
  assert.equal(db.prepare('SELECT recording_state FROM runs').get().recording_state,'complete');db.close();
 } finally {server.close();}
});

test('final threshold failure returns nonzero while fully persisted recording stays complete', {skip: !process.env.K6_SQLITE_BINARY},async() => {
 const {server,url}=await mockServer();const dir=await temporary();
 try {
  const cfg=await configuration(dir,url),output=path.join(dir,'fixtures');await fixtures(cfg,output);
  const config={services:{router:url},load:{total_rps:2,duration_seconds:1},
   payment:{amount:100,currency:'USD',card:{card_number:'4242424242424242',card_exp_month:'10',card_exp_year:'35',card_cvc:'123'}},
   scenarios:[{name:'guest',merchant_path:'non_modular',scenario:'guest',weight:100}],thresholds:{http_req_failed:['rate<0']}};
  const configPath=path.join(dir,'load.json');await writeFile(configPath,JSON.stringify(config));
  const result=await load(configPath,output,path.join(dir,'run'),process.env.K6_SQLITE_BINARY);
  assert.equal(result.exitCode,99);
  const db=new DatabaseSync(result.database,{readOnly:true});
  const run=db.prepare('SELECT recording_state,enqueued,persisted FROM runs').get();
  assert.equal(run.recording_state,'complete');assert.equal(run.enqueued,run.persisted);db.close();
 } finally {server.close();}
});

test('native capture preserves non-2xx bodies and tolerates malformed entity JSON', {skip: !process.env.K6_SQLITE_BINARY},async()=>{
 const {server,state,url}=await mockServer();const dir=await temporary();
 try {
  const cfg=await configuration(dir,url),fixtureOutput=path.join(dir,'fixtures');await fixtures(cfg,fixtureOutput);
  const config={services:{router:url,modular_pm:`${url}/v2`},load:{total_rps:4,duration_seconds:1},
   payment:{amount:100,currency:'USD',card:{card_number:'4242424242424242'}},
   scenarios:[{name:'customer_error',scenario:'cit_off_session',merchant_path:'non_modular',weight:100}]};
  const file=path.join(dir,'load.json');await writeFile(file,JSON.stringify(config));
  for(const mode of ['errorCustomers','malformedCustomers']) {
   state.errorCustomers=mode==='errorCustomers';state.malformedCustomers=mode==='malformedCustomers';
   const result=await load(file,fixtureOutput,path.join(dir,mode),process.env.K6_SQLITE_BINARY);
   assert.equal(result.exitCode,0);
   const db=new DatabaseSync(result.database,{readOnly:true});
   const events=db.prepare('SELECT * FROM request_events').all();assert.ok(events.length>=4);
   for(const event of events) {
    assert.equal(event.api_status_code,mode==='errorCustomers'?422:200);
    assert.equal(event.message,mode==='errorCustomers'?'invalid customer':'invalid_json_response');
    if(mode==='errorCustomers') assert.match(event.error_body,/invalid customer/);
    assert.ok(event.merchant_reference_id);assert.equal(event.customer_id,null);
   }
   assert.equal(db.prepare('SELECT COUNT(*) n FROM customers').get().n,0);db.close();
  }
 } finally {server.close();}
});

test('Superposition refresh countdown precedes final readiness verification',async()=>{
 const {server,url}=await mockServer(),dir=await temporary();
 try{
  const file=await configuration(dir,url),cfg=JSON.parse(await readFile(file,'utf8'));cfg.superposition.refresh_interval_seconds=1;cfg.superposition.poll_interval_seconds=.25;await writeFile(file,JSON.stringify(cfg));
  const updates=[];await fixtures(file,path.join(dir,'fixtures'),{onProgress:value=>updates.push(structuredClone(value))});
  const waiting=updates.filter(s=>s.superposition.status==='waiting');assert.equal(waiting[0].superposition.remainingSeconds,1);assert.equal(waiting.at(-1).superposition.remainingSeconds,0);assert.ok(waiting.length>=4);
  assert.ok(updates.findIndex(s=>s.superposition.status==='verifying')>updates.findIndex(s=>s.superposition.status==='waiting'));assert.equal(updates.at(-1).stage,'ready');
 }finally{server.close();}
});

test('controlled smoke counts starts before delayed responses and preserves failed checks', {skip:!process.env.K6_SQLITE_BINARY},async()=>{
 const {server,state,url}=await mockServer(),dir=await temporary(),oldControl=process.env.SQLITE_RECORDER_CONTROL_FILE,oldProgress=process.env.SQLITE_RECORDER_PROGRESS_FILE,oldToken=process.env.SQLITE_RECORDER_CONTROL_TOKEN;
 try{
  const fixturePath=path.join(dir,'fixtures');await fixtures(await configuration(dir,url),fixturePath);
  const config={services:{router:url},load:{total_rps:2,duration_seconds:3},payment:{amount:100,currency:'USD',card:{card_number:'4242424242424242'}},scenarios:[{name:'guest',scenario:'guest',merchant_path:'non_modular',weight:100}]},file=path.join(dir,'load.json');await writeFile(file,JSON.stringify(config));
  const output=path.join(dir,'delayed');process.env.SQLITE_RECORDER_CONTROL_FILE=path.join(output,'control.json');process.env.SQLITE_RECORDER_PROGRESS_FILE=path.join(output,'progress.json');state.delayPayments=4000;process.env.SQLITE_RECORDER_CONTROL_TOKEN='delayed-test-token';
  const pending=load(file,fixturePath,output,process.env.K6_SQLITE_BINARY,{recording:false,telemetry:true});let observed=false;
  for(let attempt=0;attempt<80;attempt++){
   const p=JSON.parse(await readFile(process.env.SQLITE_RECORDER_PROGRESS_FILE,'utf8').catch(()=>'{}'));
   if(p.control?.started_flows>=2){assert.equal(p.flows.guest.successes,0);assert.equal(p.control.flow_starts.guest,p.control.started_flows);assert.equal(p.control.in_flight,p.control.started_flows);observed=true;break;}
   await new Promise(resolve=>setTimeout(resolve,50));
  }
  const result=await pending;assert.ok(observed,'admissions must be visible while HTTP is blocked');assert.equal(result.exitCode,0);
  const final=JSON.parse(await readFile(process.env.SQLITE_RECORDER_PROGRESS_FILE,'utf8'));assert.equal(final.control.state,'completed');assert.equal(final.control.started_flows,6);assert.equal(final.flows.guest.successes,6);assert.equal(final.control.in_flight,0);
  await assert.rejects(readFile(path.join(output,'results.sqlite')),error=>error.code==='ENOENT');
  state.delayPayments=0;config.thresholds={http_req_failed:['rate<0']};await writeFile(file,JSON.stringify(config));const failedOutput=path.join(dir,'failed');process.env.SQLITE_RECORDER_CONTROL_FILE=path.join(failedOutput,'control.json');process.env.SQLITE_RECORDER_PROGRESS_FILE=path.join(failedOutput,'progress.json');
  assert.equal((await load(file,fixturePath,failedOutput,process.env.K6_SQLITE_BINARY,{recording:false,telemetry:true})).exitCode,99);
 }finally{server.close();if(oldControl==null)delete process.env.SQLITE_RECORDER_CONTROL_FILE;else process.env.SQLITE_RECORDER_CONTROL_FILE=oldControl;if(oldProgress==null)delete process.env.SQLITE_RECORDER_PROGRESS_FILE;else process.env.SQLITE_RECORDER_PROGRESS_FILE=oldProgress;if(oldToken==null)delete process.env.SQLITE_RECORDER_CONTROL_TOKEN;else process.env.SQLITE_RECORDER_CONTROL_TOKEN=oldToken;}
});
