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
const registry=await import('../server/worker-registry.mjs');

test('successful TPS uses success counters rather than a shutdown rate spike',()=>{
  const point=telemetryPoint({elapsed_seconds:10,successful_tps:10000,http_attempts:400,http_errors:0,recorder:{persisted:400},flows:{guest:{successes:100}}},{t:9,successes:90});
  assert.equal(point.successfulTps,10);assert.equal(point.recorded,400);
});
test('flow rate counts admissions while requests are waiting for responses',()=>{
 const point=telemetryPoint({elapsed_seconds:10,control:{started_flows:50,flow_starts:{guest:50},in_flight:50},http_attempts:0,http_errors:0,recorder:{persisted:0,enqueued:0},flows:{guest:{successes:0,failures:0}}},{t:9,started:45,successes:0,flows:{guest:{started:45,successes:0}}});
 assert.equal(point.tps,5);assert.equal(point.flows.guest.tps,5);assert.equal(point.successfulTps,0);
 const drain=telemetryPoint({elapsed_seconds:11,control:{started_flows:50,flow_starts:{guest:50},in_flight:0},http_attempts:50,http_errors:50,recorder:{persisted:50,enqueued:50},flows:{guest:{successes:0,failures:50}}},point);
 assert.equal(drain.tps,0);assert.equal(drain.flows.guest.tps,0);
});
test('error trend shows recovery rather than carrying prior failures into every sample',()=>{
  const sample=(elapsed_seconds,http_attempts,http_errors)=>({elapsed_seconds,http_attempts,http_errors,confirm_http_attempts:http_attempts,confirm_http_errors:http_errors,recorder:{persisted:http_attempts,enqueued:http_attempts},flows:{}});
  const outage=telemetryPoint(sample(10,100,80));assert.equal(outage.errors,80);
  const recovered=telemetryPoint(sample(11,110,80),outage);assert.equal(recovered.errors,0);
  const noTraffic=telemetryPoint(sample(12,110,80),recovered);assert.equal(noTraffic.errors,0);
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
test('workers enroll, heartbeat, receive ordered commands and acknowledge once',()=>{
  const identity=registry.enrollWorker({name:'worker-a',capacityTps:250,capacityVus:2000,version:'1'},'enroll-secret','enroll-secret');
  assert.equal(registry.heartbeatWorker(identity.id,identity.token,{status:'ready'}).ok,true);
  const command=registry.queueWorkerCommand(identity.id,{runId:'run-a',kind:'prepare',payload:{segment:{start:0,end:.5}}});
  assert.equal(command.revision,1);
  assert.deepEqual(registry.pendingWorkerCommands(identity.id,identity.token)[0].payload,{segment:{start:0,end:.5}});
  assert.equal(registry.acknowledgeWorkerCommand(identity.id,identity.token,command.id,{ok:true}).ok,true);
  assert.deepEqual(registry.pendingWorkerCommands(identity.id,identity.token),[]);
  assert.throws(()=>registry.acknowledgeWorkerCommand(identity.id,identity.token,command.id,{ok:true}),/already acknowledged/);
});
test('settings and redaction never expose secrets; profiles validate paths and weights',async()=>{
  const settings=state.publicSettings();assert.equal(settings.adminApiKey,'');assert.equal(settings.superposition.token,'');
  settings.superposition.accessHeader='x-superposition-access';
  assert.doesNotMatch(JSON.stringify(settings),/private-admin-test|private-sp-test/);
  assert.equal(state.redact('private-admin-test private-sp-test'),'[redacted] [redacted]');
  await assert.rejects(state.saveSettings({...settings,scenarios:[{name:'bad',scenario:'sdk_checkout',merchant_path:'modular',weight:100}]}),/Unsupported/);
  await assert.rejects(state.saveSettings({...settings,scenarios:settings.scenarios.map(f=>({...f,weight:0}))}),/100/);
  await assert.rejects(state.saveSettings({...settings,database:{...settings.database,tlsMode:'verify',caPath:'relative.pem'}}),/absolute/);
  settings.superposition.pollSeconds=2.5;
  settings.preAllocatedVusMultiplier=4;
  settings.maxVusMultiplier=12;
  settings.database={...settings.database,host:'database.internal',port:6432,database:'payments',user:'loadtest_reader',password:'database-secret',tlsMode:'verify',caPath:'/certs/database.pem'};
  const saved=await state.saveSettings(settings);assert.equal(saved.totalRps,10);assert.equal(saved.superposition.pollSeconds,2.5);assert.equal(saved.superposition.accessHeader,'x-superposition-access');assert.equal(state.readJsonSync(state.configPath).fixtures.admin_api_key,'private-admin-test');
  assert.deepEqual(saved.grafana.panels.map(panel=>panel.title),['RPS','API latency','TP latencies']);
  assert.deepEqual(saved.smokeTest,{enabled:true,rps:1,durationSeconds:10});
  assert.deepEqual(saved.database,{host:'database.internal',port:6432,database:'payments',user:'loadtest_reader',tlsMode:'verify',caPath:'/certs/database.pem',caCertificateId:'',password:'',hasPassword:true});
  assert.equal(state.localSettings().database.password,'database-secret');
  assert.equal(state.publicSettings().database.host,'database.internal');
  assert.deepEqual(state.configuredVus(state.readJsonSync(state.configPath).load),{pre_allocated_vus_multiplier:4,max_vus_multiplier:12});
  assert.deepEqual(state.withConfiguredVus({total_rps:200,pre_allocated_vus_multiplier:1,max_vus_multiplier:2},state.readJsonSync(state.configPath).load),
    {total_rps:200,pre_allocated_vus_multiplier:4,max_vus_multiplier:12});
  await assert.rejects(state.saveConnections({...saved,preAllocatedVusMultiplier:15,maxVusMultiplier:10}),/VU multipliers/);
});
test('Grafana panels cannot point to arbitrary origins or schemes',()=>{
  assert.equal(state.panelPath('https://grafana.example/grafana/d-solo/k6/x?panelId=1','https://grafana.example'),'/grafana/d-solo/k6/x?panelId=1&theme=light');
  assert.equal(state.panelPath('https://grafana.example/d/k6/x?viewPanel=panel-2&from=now-1h','https://grafana.example'),'/grafana/d-solo/k6/x?from=now-1h&panelId=2&theme=light');
  assert.equal(state.panelPath('https://grafana.example/d-solo/k6/x?panelId=3&theme=dark','https://grafana.example'),'/grafana/d-solo/k6/x?panelId=3&theme=dark');
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
  assert.equal(run.confirmErrorRate,0); // Failed payment-create calls are outside this display metric.
  const page=requestHistory({id:run.id,limit:2});assert.equal(page.rows.length,2);assert.equal(page.next,2);
  assert.equal(requestHistory({id:run.id,after:page.next,limit:2}).rows.length,1);
});
test('saved HTTP error card counts only measured payment confirmations',async()=>{
 const directory=path.join(config.output,'single/runs/confirm-errors');await mkdir(directory,{recursive:true});
 await writeFile(path.join(directory,'config.json'),JSON.stringify({load:{total_rps:10,duration_seconds:10},scenarios:config.scenarios}));
 await writeFile(path.join(directory,'summary.json'),JSON.stringify({metrics:{http_req_failed:{values:{rate:.5}}}}));
 const db=new DatabaseSync(path.join(directory,'results.sqlite'));db.exec(await readFile('../k6/scenario-mix/recorder/schema.sql','utf8'));
 db.exec("INSERT INTO runs(run_id,started_at,ended_at,recording_state,enqueued,persisted) VALUES('test',1,10001,'complete',4,4)");
 const insert=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,latency_ms) VALUES('test',?,'flow',?,'measured',1,'POST','http://test',?,2)");
 insert.run(1,'pm_session_confirm',0);insert.run(2,'pm_session_confirm',400);insert.run(3,'payment_confirm',200);insert.run(4,'payment_confirm',500);db.close();
 const run=await describeRun('single:confirm-errors');assert.equal(run.confirmErrorRate,50);assert.equal(run.errorRate,50);
});
test('restart recovery interrupts stale jobs and SSE reconnect sends current state',async()=>{
  state.controlDb().prepare("INSERT INTO jobs(id,kind,mode,status,started_at) VALUES('stale','load','single','running',1)").run();
  const jobs=await import('../server/jobs.mjs');jobs.initializeJobs();assert.equal(jobs.getJob('stale').status,'interrupted');
  const response=jobs.jobEvents('stale',new Request('http://test/api/jobs/stale/events'));
  assert.match(await response.text(),/interrupted/);
});

test('staged settings preserve shape and snapshot criteria',async()=>{
 const input=state.publicSettings();
 input.loadShape='staged';input.startRps=0;input.stagesJson=JSON.stringify([{target_rps:10,duration_seconds:4}]);
 input.acceptance.maximumConfirmP99Ms=350;
 const saved=await state.saveSettings(input);assert.equal(saved.totalRps,10);assert.equal(saved.durationSeconds,4);
 const c=state.readJsonSync(state.configPath);assert.equal(c.load.total_rps,undefined);assert.equal(c.load.duration_seconds,undefined);assert.equal(c.load.start_rps,0);assert.equal(c.acceptance.maximumConfirmP99Ms,350);
 await assert.rejects(state.saveSettings({...input,stagesJson:'[]'}),/positive/);
 await state.saveSettings({...saved,loadShape:'flat'});assert.equal(state.readJsonSync(state.configPath).load.stages,undefined);
});

test('versioned profiles are isolated from connection edits and reject stale saves',async()=>{
 const {saveProfile,getProfile,profileDraft}=await import('../server/profiles.mjs');
 const first=saveProfile({...profileDraft(),name:'Single 200 TPS',totalRps:200,durationSeconds:60});
 const second=saveProfile({...first,totalRps:1000},first.id);
 assert.equal(second.version,2);assert.equal(getProfile(first.id,1).totalRps,200);
 assert.throws(()=>saveProfile({...first,totalRps:300},first.id),/changed/);
 const settings=state.publicSettings();await state.saveConnections({...settings,scenarios:[],database:{...settings.database,user:'existing_user'}});
 assert.equal(getProfile(first.id).totalRps,1000);assert.equal(state.publicSettings().database.user,'existing_user');
 assert.throws(()=>saveProfile({...first,name:'bad',mode:'single',merchantCount:1500}),/Single/);
});
test('fixture compatibility includes merchant count and provisioning settings, not rate',async()=>{
 const {fixtureSignature,validateFixtureSet}=await import('../server/profiles.mjs');
 const c=state.readJsonSync(state.configPath),sig=fixtureSignature(c,'single',1);
 assert.equal(sig,fixtureSignature({...c,load:{total_rps:1000}},'single',1));
 assert.equal(sig,fixtureSignature({...c,services:{...c.services,modular_pm:'http://different'}},'single',1));
 assert.equal(sig,fixtureSignature({...c,fixtures:{...c.fixtures,admin_api_key:'rotated',superposition:{...c.fixtures.superposition,endpoint:'http://different',token:'rotated',refresh_interval_seconds:1,poll_interval_seconds:.25}}},'single',1));
 assert.notEqual(sig,fixtureSignature(c,'tenancy',1500));
 assert.notEqual(sig,fixtureSignature({...c,fixtures:{...c.fixtures,router:'http://different'}},'single',1));
 assert.notEqual(sig,fixtureSignature({...c,fixtures:{...c.fixtures,superposition:{...c.fixtures.superposition,defaults:{changed:true}}}},'single',1));
 const dir=path.join(temporary,'fixture-set');await mkdir(dir);await writeFile(path.join(dir,'manifest.json'),JSON.stringify({state:'ready'}));
 state.controlDb().prepare('INSERT INTO fixture_sets VALUES(?,?,?,?,?,?,NULL,?)').run('fixture-test','single',1,sig,dir,'ready',Date.now());
 assert.equal(validateFixtureSet('fixture-test',{mode:'single',merchantCount:1},c).id,'fixture-test');
 assert.throws(()=>validateFixtureSet('fixture-test',{mode:'tenancy',merchantCount:1500},c),/does not match/);
});
test('ramp editor round trips holds and calculates offered-flow preview',async()=>{
 const {encodeStages,decodeStages,rampPoints}=await import('../src/ramp-editor.mjs');
 const {loadShape}=await import('../src/load-shape.mjs');
 const stages=encodeStages([{kind:'ramp',target:200,duration:60},{kind:'hold',target:99,duration:120},{kind:'ramp',target:1000,duration:60}],10);
 assert.equal(stages[1].target_rps,200);assert.equal(decodeStages(JSON.stringify(stages),10)[1].kind,'hold');
 assert.equal(loadShape({start_rps:10,stages}).duration,240);assert.equal(rampPoints(10,stages).at(-1).value,1000);
 assert.throws(()=>encodeStages([{kind:'hold',duration:0}],10),/positive/);
});
test('new flows get unique metric names without changing existing flows',async()=>{
 const {newFlow,profileSummary}=await import('../src/flow-catalog.mjs');
 const flows=[{name:'guest',scenario:'guest',merchant_path:'modular',weight:50},{name:'guest_2',scenario:'guest',merchant_path:'non_modular',weight:50}];
 const before=JSON.stringify(flows),flow=newFlow('guest',flows);assert.equal(flow.name,'guest_3');assert.equal(flow.weight,0);assert.equal(JSON.stringify(flows),before);
 assert.match(profileSummary({mode:'tenancy',merchantCount:1500,totalRps:1000,durationSeconds:180,loadShape:'staged'}),/Tenancy · 1500 merchants · 1000 peak TPS · 180s/);
});
