import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,readFile,writeFile,mkdir } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const temporary=await mkdtemp(path.join(os.tmpdir(),'loadtest-console-core-'));
process.env.LOADTEST_DATA_DIR=temporary;
process.env.LOADTEST_CONSOLE_ROOT=path.resolve('.');
process.env.LOADTEST_ROOT=path.resolve('..');
const config=JSON.parse(await readFile('../loadtest.config.example.json','utf8'));
config.output=path.join(temporary,'artifacts');config.fixtures.admin_api_key='private-admin-test';config.fixtures.superposition.token='private-sp-test';
process.env.LOADTEST_CONFIG_FILE=path.join(temporary,'seed.json');await writeFile(process.env.LOADTEST_CONFIG_FILE,JSON.stringify(config));
const state=await import('../server/state.mjs');state.initializeConfig();
const auth=await import('../server/auth.mjs');
const {comparePayments,describeRun,requestHistory}=await import('../server/queries.mjs');
const {telemetryPoint}=await import('../server/telemetry.mjs');

test('trend TPS uses success counters rather than a shutdown rate spike',()=>{
  const point=telemetryPoint({elapsed_seconds:10,successful_tps:10000,http_attempts:400,http_errors:0,recorder:{persisted:400},flows:{guest:{successes:100}}},{t:9,successes:90});
  assert.equal(point.tps,10);assert.equal(point.recorded,400);
});

test('scrypt login, hashed persistent sessions, expiry and logout',async()=>{
  process.env.ADMIN_PASSWORD_HASH=await auth.passwordHash('long test password');
  assert.equal(await auth.verifyPassword('wrong',process.env.ADMIN_PASSWORD_HASH),false);
  const token=await auth.login('admin','long test password','login-ok');
  assert.ok(auth.authenticated(token));
  const onDisk=state.controlDb().prepare('SELECT token FROM sessions').get().token;assert.notEqual(onDisk,token);
  auth.logout(token);assert.equal(auth.authenticated(token),false);
  const expired=await auth.login('admin','long test password','login-expired');state.controlDb().exec('UPDATE sessions SET expires=1');assert.equal(auth.authenticated(expired),false);
});
test('persistent login limiting and strict origin checks',async()=>{
  for(let n=0;n<5;n++)assert.equal(await auth.login('admin','bad','blocked-ip'),null);
  await assert.rejects(auth.login('admin','long test password','blocked-ip'),e=>e.status===429);
  const url=new URL('https://console.example');
  assert.ok(auth.sameOrigin(new Request(url,{headers:{origin:url.origin}}),url));
  assert.equal(auth.sameOrigin(new Request(url,{headers:{origin:'https://other.example'}}),url),false);
});
test('settings and redaction never expose secrets; profiles validate paths and weights',async()=>{
  const settings=state.publicSettings();assert.equal(settings.adminApiKey,'');assert.equal(settings.superposition.token,'');
  assert.doesNotMatch(JSON.stringify(settings),/private-admin-test|private-sp-test/);
  assert.equal(state.redact('private-admin-test private-sp-test'),'[redacted] [redacted]');
  await assert.rejects(state.saveSettings({...settings,scenarios:[{name:'bad',scenario:'sdk_checkout',merchant_path:'modular',weight:100}]}),/Unsupported/);
  await assert.rejects(state.saveSettings({...settings,scenarios:settings.scenarios.map(f=>({...f,weight:0}))}),/100/);
  const saved=await state.saveSettings(settings);assert.equal(saved.totalRps,10);assert.equal(state.readJsonSync(state.configPath).fixtures.admin_api_key,'private-admin-test');
});
test('Grafana panels cannot point to arbitrary origins or schemes',()=>{
  assert.equal(state.panelPath('https://grafana.example/grafana/d-solo/k6/x?panelId=1','https://grafana.example'),'/grafana/d-solo/k6/x?panelId=1');
  assert.equal(state.panelPath('https://grafana.example/d/k6/x?viewPanel=2&from=now-1h','https://grafana.example'),'/grafana/d-solo/k6/x?from=now-1h&panelId=2');
  assert.equal(state.panelPath('https://grafana.example/d-solo/k6/x?panelId=3','https://grafana.example'),'/grafana/d-solo/k6/x?panelId=3');
  assert.throws(()=>state.panelPath('https://grafana.example/d/k6/x','https://grafana.example'),/Share → Embed/);
  assert.throws(()=>state.panelPath('https://evil.example/grafana/d-solo/x','https://grafana.example'));
  assert.throws(()=>state.httpUrl('file:///etc/passwd'));
  assert.throws(()=>state.runDirectory('single:..'));
});
test('reconciliation distinguishes merchant/payment pairs and exact statuses',()=>{
  const results=comparePayments([{merchant_id:'one',payment_id:'same',status:'succeeded'},
    {merchant_id:'two',payment_id:'same',status:'processing'},{merchant_id:'three',payment_id:'none',status:'failed'}],
    [{merchant_id:'one',payment_id:'same',status:'succeeded'},{merchant_id:'two',payment_id:'same',status:'succeeded'}]);
  assert.deepEqual(results.map(r=>r.outcome),['match','mismatch','missing']);
});
test('saved run success TPS excludes failed iterations and response history paginates',async()=>{
  const directory=path.join(config.output,'single/runs/run-test');await mkdir(directory,{recursive:true});
  await writeFile(path.join(directory,'config.json'),JSON.stringify({load:{total_rps:10,duration_seconds:10},scenarios:config.scenarios}));
  await writeFile(path.join(directory,'summary.json'),JSON.stringify({metrics:{iterations:{values:{count:100}},scenario_success_mod_cit_off:{values:{count:30}},scenario_failure_mod_cit_off:{values:{count:20}},achieved_tps_mod_cit_off:{values:{value:3}}}}));
  const db=new DatabaseSync(path.join(directory,'results.sqlite'));db.exec(await readFile('../k6/scenario-mix/recorder/schema.sql','utf8'));
  db.prepare("INSERT INTO runs(run_id,started_at,ended_at,recording_state,enqueued,persisted) VALUES('test',1,10001,'complete',3,3)").run();
  const insert=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,latency_ms) VALUES('test',?,'flow','payment_create','measured',1,'POST','http://test',500,2)");
  for(let n=1;n<=3;n++)insert.run(n);db.close();
  const run=await describeRun('single:run-test');assert.equal(run.successfulTps,3);assert.equal(run.iterations,100);assert.equal(run.flows[0].errors,20);
  const page=requestHistory({id:run.id,limit:2});assert.equal(page.rows.length,2);assert.equal(page.next,2);
  assert.equal(requestHistory({id:run.id,after:page.next,limit:2}).rows.length,1);
});
test('restart recovery interrupts stale jobs and SSE reconnect sends current state',async()=>{
  state.controlDb().prepare("INSERT INTO jobs(id,kind,mode,status,started_at) VALUES('stale','load','single','running',1)").run();
  const jobs=await import('../server/jobs.mjs');jobs.initializeJobs();assert.equal(jobs.getJob('stale').status,'interrupted');
  const response=jobs.jobEvents('stale',new Request('http://test/api/jobs/stale/events'));
  assert.match(await response.text(),/interrupted/);
});
