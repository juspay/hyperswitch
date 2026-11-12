import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {balanceWeights} from '../src/flow-catalog.mjs';
import {switchFaultEnvironment} from '../src/fault-types.mjs';
const temporary=await mkdtemp(path.join(os.tmpdir(),'console-interactions-'));
process.env.LOADTEST_DATA_DIR=temporary;process.env.LOADTEST_CONSOLE_ROOT=path.resolve('.');process.env.LOADTEST_ROOT=path.resolve('..');
const config=JSON.parse(await readFile('../loadtest.config.example.json','utf8'));config.output=path.join(temporary,'artifacts');
process.env.LOADTEST_CONFIG_FILE=path.join(temporary,'seed.json');await writeFile(process.env.LOADTEST_CONFIG_FILE,JSON.stringify(config));
const state=await import('../server/state.mjs');state.initializeConfig();
const profiles=await import('../server/profiles.mjs'),faults=await import('../server/faults.mjs');
test('deleted fixture sets stay removed after CLI import and preserve saved artifacts',async()=>{
 const directory=path.join(config.output,'single','fixtures');await mkdir(directory,{recursive:true});
 await writeFile(path.join(directory,'manifest.json'),JSON.stringify({state:'ready',merchant_count:1}));
 const artifact=path.join(directory,'merchants.json');await writeFile(artifact,'[{"merchant_id":"existing"}]');
 const set=profiles.listFixtureSets().find(s=>s.mode==='single');assert.ok(set);
 assert.deepEqual(profiles.deleteFixtureSet(set.id),{id:set.id,deleted:true});
 assert.equal(profiles.listFixtureSets().some(s=>s.id===set.id),false);
 assert.equal(profiles.listFixtureSets().some(s=>s.mode==='single'),false);
 assert.throws(()=>profiles.getFixtureSet(set.id),e=>e.status===404);
 assert.equal(await readFile(artifact,'utf8'),'[{"merchant_id":"existing"}]');
 assert.throws(()=>profiles.deleteFixtureSet('missing'),e=>e.status===404);
});
test('fixture deletion is blocked during active and paused generators',()=>{
 const db=state.controlDb();
 db.prepare('INSERT INTO fixture_sets VALUES(?,?,?,?,?,?,?,?)').run('delete-guard','tenancy',100,'signature','/unused','ready',null,1);
 for(const kind of ['fixtures','load','smoke'])for(const status of ['starting','running','pausing','paused','cancelling']){
  db.prepare('INSERT INTO jobs(id,kind,mode,status,started_at) VALUES(?,?,?,?,?)').run('deletion-job',kind,'tenancy',status,1);
  assert.throws(()=>profiles.deleteFixtureSet('delete-guard'),e=>e.status===409);
  assert.equal(profiles.getFixtureSet('delete-guard').status,'ready');
  db.prepare('DELETE FROM jobs WHERE id=?').run('deletion-job');
 }
 profiles.deleteFixtureSet('delete-guard');
});
test('scenario and shared targets save atomically, conflicts preserve both, archive preserves versions',()=>{
 const original=profiles.saveProfile({...profiles.profileDraft(),name:'Original scenario'}),targets=[{id:'redis',label:'Redis',serviceType:'redis',environment:'local',adapter:'process-compose',binary:'process-compose',controllerPort:8090,process:'r1',port:6379}];
 const before=JSON.stringify(faults.localFaultTargets());
 assert.throws(()=>profiles.saveScenario({profile:{...original,name:''},targets,expectedTargets:before}),/name/i);
 assert.equal(JSON.stringify(faults.localFaultTargets()),before);assert.equal(profiles.getProfile(original.id).version,original.version);
 const saved=profiles.saveScenario({profile:{...original,name:'Saved together'},targets,expectedTargets:before});
 assert.equal(saved.profile.version,original.version+1);assert.equal(faults.localFaultTargets().length,1);
 assert.throws(()=>profiles.saveScenario({profile:saved.profile,targets:[],expectedTargets:before}),/changed/i);
 assert.equal(profiles.getProfile(original.id).version,saved.profile.version);assert.equal(faults.localFaultTargets().length,1);
 profiles.archiveProfile(original.id,true);assert.equal(profiles.listProfiles().some(p=>p.id===original.id),false);
 assert.equal(profiles.listProfiles({includeArchived:true}).find(p=>p.id===original.id).archived,true);
 profiles.archiveProfile(original.id,false);assert.equal(profiles.getProfile(original.id).version,saved.profile.version);
});
test('scenario deletion removes the profile and its saved versions',()=>{
 const profile=profiles.saveProfile({...profiles.profileDraft(),name:'Delete me'}),deleted=profiles.deleteProfile(profile.id);
 assert.deepEqual(deleted,{id:profile.id,deleted:true});assert.throws(()=>profiles.getProfile(profile.id),/not found/i);
 assert.equal(state.controlDb().prepare('SELECT COUNT(*) count FROM profile_versions WHERE profile_id=?').get(profile.id).count,0);
});
test('deleting custom scenarios preserves the five immutable defaults',()=>{
 for(const profile of profiles.listProfiles({includeArchived:true}).filter(p=>!p.isDefault))profiles.deleteProfile(profile.id);
 assert.equal(profiles.listProfiles().length,5);
 assert.ok(profiles.listProfiles().every(p=>p.isDefault));
});
test('environment presets round trip without discarding edited values',()=>{
 const local={id:'vault',label:'Card vault',adapter:'local-vault',serviceType:'card-vault',environment:'local',repository:'/tmp/vault',port:3009};
 const cloud=switchFaultEnvironment(local,'cloud');cloud.namespace='custom-namespace';
 const restored=switchFaultEnvironment(cloud,'local');assert.equal(restored.port,3009);assert.equal(restored.repository,'/tmp/vault');
 assert.equal(switchFaultEnvironment(restored,'cloud').namespace,'custom-namespace');
});
test('explicit weight balancing totals exactly 100 and preserves hidden fields',()=>{
 const flows=[{name:'a',weight:25,extra:'keep'},{name:'b',weight:25},{name:'c',weight:0}];
 assert.deepEqual(balanceWeights(flows).map(f=>f.weight),[25,25,50]);
 for(const weights of [[1,1,1],[0,0,0],[150,20,3],[.25,.5,.25]]){
 const result=balanceWeights(weights.map(weight=>({weight})),true);assert.equal(result.reduce((n,f)=>n+f.weight,0),100);assert.ok(result.every(f=>f.weight>=0));}
 assert.equal(balanceWeights(flows,true)[0].extra,'keep');assert.equal(flows[2].weight,0);
});

test('default scenarios reject edit, deletion and archival through the server',()=>{
 for(const profile of profiles.listProfiles().filter(p=>p.isDefault)){
  assert.throws(()=>profiles.saveProfile({...profile,name:'Changed'},profile.id),e=>e.status===403);

  assert.throws(()=>profiles.deleteProfile(profile.id),e=>e.status===403);
  assert.throws(()=>profiles.archiveProfile(profile.id,true),e=>e.status===403);
  assert.equal(profiles.getProfile(profile.id).name,profile.name);
 }
 assert.throws(()=>profiles.getProfile('default:steady-load',99),/version not found/i);
});
test('default load definitions and provider-specific faults remain distinct',async()=>{
 const {defaultScenarios}=await import('../server/default-scenarios.mjs');
 const local=defaultScenarios('local',[{id:'actual-vault',serviceType:'card-vault',environment:'local',adapter:'local-vault',actions:['outage']}]);
 assert.deepEqual(local.map(p=>p.name),['Card vault outage','DB failover','Steady load','Ramped load','Tenancy steady load']);
 assert.ok(local.every(p=>p.durationSeconds===300));
 assert.equal(local[0].load.stages[1].duration_seconds,30);
 assert.equal(local[2].totalRps,5);assert.equal(local[2].durationSeconds,300);
 const tenancy=profiles.getProfile('default:tenancy-steady-load');
 assert.equal(tenancy.mode,'tenancy');assert.equal(tenancy.merchantCount,100);
 assert.equal(tenancy.totalRps,5);assert.equal(tenancy.durationSeconds,300);assert.equal(tenancy.loadShape,'flat');
 assert.deepEqual(tenancy.load,local[2].load);assert.deepEqual(tenancy.scenarios,local[2].scenarios);assert.deepEqual(tenancy.acceptance,local[2].acceptance);
 assert.equal(local[3].totalRps,10);assert.equal(local[3].version,3);assert.equal(profiles.getProfile('default:ramped-load',3).totalRps,10);assert.equal(local[3].durationSeconds,300);
 assert.equal(local[0].load.stages[1].fault.targetId,'actual-vault');assert.equal(local[0].unavailableReason,null);
 assert.match(local[1].unavailableReason,/GCP/);assert.equal(local[1].load.stages[1].fault.action,'failover');
 const cloud=defaultScenarios('gcp',[{id:'ha-primary',serviceType:'postgres',environment:'gcp',adapter:'gcp-alloydb',actions:['failover']}]);
 assert.equal(cloud[1].unavailableReason,null);assert.equal(cloud[1].load.stages[1].fault.targetId,'ha-primary');
 assert.ok(cloud[0].unavailableReason);assert.ok(defaultScenarios('aws').every(p=>p.unavailableReason));
 assert.ok(defaultScenarios('local',[{id:'fake',serviceType:'card-vault',environment:'local',adapter:'simulated',actions:['outage']}])[0].unavailableReason);
 local[2].scenarios[0].weight=0;assert.equal(defaultScenarios('local')[2].scenarios[0].weight,50);
});


test('custom scenarios sort by creation time ahead of defaults, and edits retain their position',()=>{
 const first=profiles.saveProfile({...profiles.profileDraft(),name:'Earlier'});
 const second=profiles.saveProfile({...profiles.profileDraft(),name:'Later'});
 const db=state.controlDb();db.prepare('UPDATE profiles SET created_at=? WHERE id=?').run(100,first.id);db.prepare('UPDATE profiles SET created_at=? WHERE id=?').run(200,second.id);
 const updated=profiles.saveProfile({...first,name:'Earlier edited'},first.id);
 assert.equal(updated.createdAt,100);
 const list=profiles.listProfiles(),custom=list.filter(p=>!p.isDefault);
 assert.ok(custom.findIndex(p=>p.id===second.id)<custom.findIndex(p=>p.id===first.id));
 assert.ok(list.slice(-5).every(p=>p.isDefault));assert.ok(list.slice(0,-5).every(p=>!p.isDefault));
});
test('saving an edited default creates an independent scenario and keeps the built-in intact',()=>{
 const original=profiles.getProfile('default:steady-load');
 const saved=profiles.saveScenario({profile:{...original,totalRps:7}}).profile;
 assert.notEqual(saved.id,original.id);assert.equal(saved.version,1);assert.equal(saved.isDefault,undefined);assert.equal(saved.totalRps,7);assert.equal(saved.name,'Steady load (copy)');
 assert.equal(profiles.getProfile(original.id).totalRps,5);
 const renamed=profiles.saveScenario({profile:{...original,name:'My steady load'}}).profile;
 assert.equal(renamed.name,'My steady load');assert.notEqual(renamed.id,saved.id);
 assert.equal(profiles.getProfile(original.id).name,'Steady load');
});
