import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile} from 'node:fs/promises';
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
test('deleting every scenario leaves an empty library',()=>{
 for(const profile of profiles.listProfiles({includeArchived:true}))profiles.deleteProfile(profile.id);
 assert.deepEqual(profiles.listProfiles(),[]);
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
