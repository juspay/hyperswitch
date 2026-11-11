import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile} from 'node:fs/promises';import os from 'node:os';import path from 'node:path';
import {targetEnvironment,validateTargetProvider,requireSupportedEnvironment} from '../src/environments.mjs';
import {normalizePaymentFlows,assertSupportedPaymentFlows} from '../src/payment-flows.mjs';
const dir=await mkdtemp(path.join(os.tmpdir(),'testbench-environments-'));process.env.LOADTEST_DATA_DIR=dir;process.env.LOADTEST_CONSOLE_ROOT=path.resolve('.');process.env.LOADTEST_ROOT=path.resolve('..');
const seed=JSON.parse(await readFile('../loadtest.config.example.json','utf8'));seed.fixtures.admin_api_key='local-admin-secret';seed.fixtures.superposition.token='local-sp-secret';seed.output=path.join(dir,'artifacts');
process.env.LOADTEST_CONFIG_FILE=path.join(dir,'seed.json');await writeFile(process.env.LOADTEST_CONFIG_FILE,JSON.stringify(seed));
const state=await import('../server/state.mjs');state.initializeConfig();
const faults=await import('../server/faults.mjs');
test('provider identity is explicit; unsupported and mismatched adapters never execute',()=>{
 assert.equal(targetEnvironment({adapter:'kubernetes'}),null);assert.equal(targetEnvironment({adapter:'kubernetes',environment:'cloud'}),'gcp');
 assert.throws(()=>requireSupportedEnvironment('aws'),/execution is not supported/);assert.throws(()=>requireSupportedEnvironment('azure'),/execution is not supported/);
 assert.throws(()=>validateTargetProvider({environment:'local',adapter:'gcp-alloydb'},'postgres'),/does not support/);
 assert.throws(()=>faults.makeFaultAdapter({adapter:'placeholder'}),/Unsupported/);
});
test('provider settings round-trip independently without returning or borrowing secrets',async()=>{
 await state.saveConnections({...state.publicSettings(),database:{...state.publicSettings().database,password:'local-db-secret'}});
 const draft=state.publicSettings('gcp');assert.equal(draft.hasAdminApiKey,false);assert.equal(draft.database.hasPassword,false);
 await state.saveConnections({...draft,router:'https://router.example',modularPm:'https://pm.example/v2',adminApiKey:'gcp-admin-secret',superposition:{...draft.superposition,token:'gcp-sp-secret'},database:{...draft.database,password:'gcp-db-secret'}});
 assert.equal(state.activeEnvironment(),'gcp');assert.equal(state.publicSettings().hasAdminApiKey,true);
 const local=state.publicSettings('local');assert.equal(local.router,seed.services.router);assert.equal(local.database.hasPassword,true);assert.equal(local.database.password,'');
 await state.saveConnections(local);assert.equal(state.activeEnvironment(),'local');assert.equal(state.localSettings().database.password,'local-db-secret');
 assert.equal(state.publicSettings('gcp').router,'https://router.example');assert.doesNotMatch(JSON.stringify(state.publicSettings('gcp')),/gcp-admin-secret|gcp-db-secret|local-admin-secret/);
 assert.equal(state.redact('gcp-admin-secret local-db-secret'),'[redacted] [redacted]');
});
test('fault targets stay scoped; a cloud target cannot shadow the local service',async()=>{
 const local={id:'vault',label:'Card vault',serviceType:'card-vault',adapter:'local-vault',environment:'local',port:3001,repository:'/tmp/vault'};
 faults.saveLocalFaultTargets([local]);
 await state.saveConnections(state.publicSettings('gcp'));
 const cloud={id:'vault',label:'Card vault',serviceType:'card-vault',adapter:'kubernetes',environment:'gcp',namespace:'test',deployment:'vault'};
 faults.saveLocalFaultTargets([cloud]);assert.equal(faults.faultTargets().find(t=>t.id==='vault').adapter,'kubernetes');
 await state.saveConnections(state.publicSettings('local'));assert.equal(faults.faultTargets().find(t=>t.id==='vault').adapter,'local-vault');
 await assert.rejects(faults.checkLocalFaultTarget(cloud),/selected environment/);
});
test('provider switching is blocked while a job or recovery is active',async()=>{
 const db=state.controlDb();db.prepare("INSERT INTO jobs(id,kind,status,started_at) VALUES('busy','load','running',?)").run(Date.now());
 await assert.rejects(state.saveConnections(state.publicSettings('gcp')),/Finish active/);assert.equal(state.activeEnvironment(),'local');db.prepare("DELETE FROM jobs WHERE id='busy'").run();
 await writeFile(path.join(dir,'fault-ledger.json'),'{}');await assert.rejects(state.saveConnections(state.publicSettings('gcp')),/recovery/);const {unlink}=await import('node:fs/promises');await unlink(path.join(dir,'fault-ledger.json'));
});
test('new flow execution always uses the payment-method service and unsupported flows fail explicitly',()=>{
 const source=[{name:'nomod_guest',scenario:'guest',merchant_path:'non_modular',weight:100}];const next=normalizePaymentFlows(source);
 assert.equal(next[0].merchant_path,'modular');assert.equal(next[0].name,'guest');assert.equal(source[0].merchant_path,'non_modular');
 assert.throws(()=>assertSupportedPaymentFlows([{scenario:'mit',weight:10}]),/not yet supported/);assert.doesNotThrow(()=>assertSupportedPaymentFlows(next));
});
test('provider filtering happens before server-target deduplication',async()=>{
 const file=path.join(dir,'server-targets.json');const local={id:'shared',serviceType:'card-vault',label:'Vault',environment:'local',adapter:'local-vault',port:3001,repository:'/tmp/vault'};
 const cloud={id:'shared',serviceType:'card-vault',label:'Vault',environment:'gcp',adapter:'kubernetes',namespace:'test',deployment:'vault'};
 await writeFile(file,JSON.stringify([cloud,local]));process.env.LOADTEST_FAULT_TARGETS=file;
 try{assert.equal(faults.faultTargets().find(t=>t.id==='shared').adapter,'local-vault');}finally{delete process.env.LOADTEST_FAULT_TARGETS;}
});
test('legacy presets are flattened once and unclassified targets remain inert',async()=>{
 const local={id:'vault',serviceType:'card-vault',label:'Vault',environment:'local',adapter:'local-vault',port:3001,repository:'/tmp/vault',presets:{cloud:{adapter:'kubernetes',namespace:'legacy',deployment:'vault',environment:'cloud'}}};
 state.controlDb().prepare("INSERT OR REPLACE INTO console_preferences VALUES('fault-targets',?)").run(JSON.stringify([local]));
 for(let i=0;i<3;i++)faults.saveLocalFaultTargets(faults.localFaultTargets());
 const raw=JSON.parse(state.controlDb().prepare("SELECT value FROM console_preferences WHERE key='fault-targets'").get().value);
 assert.equal(raw.length,2);assert.ok(raw.every(t=>!t.presets));
 const ambiguous={id:'unknown',label:'Vault',serviceType:'card-vault',adapter:'kubernetes',namespace:'legacy',deployment:'vault',environment:'custom'};
 state.controlDb().prepare("INSERT OR REPLACE INTO console_preferences VALUES('fault-targets',?)").run(JSON.stringify([ambiguous]));
 assert.equal(faults.localFaultTargets()[0].environment,'unassigned');assert.ok(!faults.faultTargets().some(t=>t.id==='unknown'));
});
test('fault confirmation rejects provider and target changes before reserving or probing',async()=>{
 const {startFaultTest}=await import('../server/jobs.mjs');
 const before=state.controlDb().prepare('SELECT COUNT(*) n FROM jobs').get().n;
 await assert.rejects(startFaultTest({environment:'gcp',targetId:'vault'}),/Environment changed/);
 assert.equal(state.controlDb().prepare('SELECT COUNT(*) n FROM jobs').get().n,before);
 faults.saveLocalFaultTargets([{id:'vault',label:'Vault',serviceType:'card-vault',environment:'local',adapter:'local-vault',port:3001,repository:'/tmp/vault'}]);
 const target=faults.localFaultTargets()[0];
 await assert.rejects(faults.testFault({id:'not-started',command:'fault-test',status:'running'},{environment:'local',targetId:'vault',action:'outage',confirm:'outage vault',durationSeconds:1,expectedTarget:{...target,port:3002}},()=>{}),/target changed/);
 assert.equal(faults.faultStatus().active,null);
});
test('active jobs freeze connections, while historical reconciliation requires the run provider',async()=>{
 const db=state.controlDb();db.prepare("INSERT INTO jobs(id,kind,status,started_at) VALUES('freeze','load','running',?)").run(Date.now());
 try{await assert.rejects(state.saveConnections({...state.publicSettings(),router:'http://127.0.0.1:9999'}),/Finish active/);}finally{db.prepare("DELETE FROM jobs WHERE id='freeze'").run();}
 const {mkdir}=await import('node:fs/promises');const directory=path.join(state.readJsonSync(state.configPath).output,'single/runs/provider-history');await mkdir(directory,{recursive:true});await writeFile(path.join(directory,'config.json'),JSON.stringify({environment:'gcp'}));
 const {startReconciliation}=await import('../server/jobs.mjs');await assert.rejects(startReconciliation('single:provider-history'),/Select GCP/);
});

test('stalled DB recovery permits only a verified TLS repair with no active jobs',async()=>{
 const previous=state.publicSettings();await state.saveConnections(state.publicSettings('gcp'));
 const before=state.publicSettings(),ledger=path.join(dir,'fault-ledger.json');
 await writeFile(ledger,JSON.stringify({target:{adapter:'gcp-alloydb'},status:'restoring',action:'failover'}));
 const input={...before,database:{...before.database,tlsMode:'require'}};let probes=0;
 const options={databaseTest:async database=>{probes++;assert.equal(database.tlsMode,'require');return {ok:true};}};
 try{
  await assert.rejects(state.saveConnections({...input,database:{...input.database,host:'other-db'}},options),/Finish active/);
  const db=state.controlDb();db.prepare("INSERT INTO jobs(id,kind,status,started_at) VALUES('repair-busy','load','running',?)").run(Date.now());
  try{await assert.rejects(state.saveConnections(input,options),/Finish active/);}finally{db.prepare("DELETE FROM jobs WHERE id='repair-busy'").run();}
  assert.equal(probes,0);
  await assert.rejects(state.saveConnections(input,{databaseTest:async()=>({ok:false})}),/Database TLS repair could not be verified/);
  assert.equal(state.localSettings().database.tlsMode,before.database.tlsMode);
  await state.saveConnections(input,options);assert.equal(probes,1);assert.equal(state.localSettings().database.tlsMode,'require');
  assert.equal(JSON.parse(await readFile(ledger,'utf8')).status,'restoring');
 }finally{const {unlink}=await import('node:fs/promises');await unlink(ledger);await state.saveConnections(previous);}
});
