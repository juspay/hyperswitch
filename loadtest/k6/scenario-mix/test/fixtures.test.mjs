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
  await fixtures(cfg,output);const keys=state.keys;
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
  const db=new DatabaseSync(result.database,{readOnly:true});
  const events=db.prepare('SELECT COUNT(*) n FROM request_events').get().n;
  assert.equal(events,state.requests-before);assert.ok(events>100);
  assert.equal(db.prepare('SELECT MIN(api_status_code) n FROM request_events').get().n,200);
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
