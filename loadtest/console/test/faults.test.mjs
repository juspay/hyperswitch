import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,realpath} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
const temporary=await realpath(await mkdtemp(path.join(os.tmpdir(),'loadtest-fault-test-')));
process.env.LOADTEST_DATA_DIR=temporary;
const {validateTargets,makeFaultAdapter,fireFault,faultStatus,restoreFaults,initializeFaults}=await import('../server/faults.mjs');
const {evaluateAcceptance}=await import('../server/acceptance.mjs');
test('targets reject command injection and duplicate IDs',()=>{
 assert.throws(()=>validateTargets([{id:'unsafe',adapter:'docker',container:'x; rm -rf'}]),/Invalid/);
 assert.throws(()=>validateTargets([{id:'x',adapter:'simulated'},{id:'x',adapter:'simulated'}]),/Duplicate/);
});
test('Docker restore checks identity and uses fixed argv',async()=>{
 const calls=[];let id='same';
 const adapter=makeFaultAdapter({adapter:'docker',container:'test'},async(binary,args)=>{calls.push([binary,args]);return JSON.stringify([{Id:id,State:{Running:true,Health:{Status:'healthy'}}}]);});
 const before=await adapter.probe();await adapter.apply('outage');assert.deepEqual(calls.at(-1),['docker',['stop','--time','30','test']]);
 id='replacement';await assert.rejects(adapter.restore(before),/identity/);assert.equal(calls.filter(c=>c[1][0]==='start').length,0);
 id='same';await adapter.restore(before);assert.ok(await adapter.verify(before));
});
test('Kubernetes recovery waits for updated replicas',async()=>{
 const info={metadata:{uid:'one',generation:2},spec:{replicas:2},status:{availableReplicas:2,updatedReplicas:1,observedGeneration:2}};
 const adapter=makeFaultAdapter({adapter:'kubernetes',context:'test',namespace:'local',deployment:'router'},async()=>JSON.stringify(info));
 const before=await adapter.probe();assert.equal(await adapter.verify(before),false);info.status.updatedReplicas=2;assert.equal(await adapter.verify(before),true);
});
test('GCP failover requires an HA target',async()=>{
 const adapter=makeFaultAdapter({adapter:'gcp-redis',project:'test',region:'us-central1',instance:'cache'},async()=>JSON.stringify({state:'READY',tier:'BASIC'}));
 await assert.rejects(adapter.probe(),/HA/);
});
test('simulation is exclusive, confirmed, durable and restores on cancellation',async()=>{
 const events=[],emit=(id,kind,message)=>events.push({id,kind,message});
 const job={id:'safe-test',command:'load',loadState:'running'};
 await assert.rejects(fireFault({...job,loadState:'paused'},{},emit),/running/);
 await assert.rejects(fireFault(job,{targetId:'demo',action:'outage'},emit),/Type outage demo/);
 await fireFault(job,{targetId:'demo',action:'outage',confirm:'outage demo',durationSeconds:30},emit);
 await assert.rejects(fireFault(job,{targetId:'demo',action:'outage',confirm:'outage demo'},emit),/active/);
 for(let n=0;n<100;n++){if(events.some(e=>e.kind==='fault_started'))break;await new Promise(r=>setTimeout(r,10));}
 assert.equal(JSON.parse(await readFile(path.join(temporary,'fault-ledger.json'),'utf8')).status,'active');
 await restoreFaults(emit);assert.equal(faultStatus().active,null);assert.equal(JSON.parse(await readFile(path.join(temporary,'fault-ledger.json'),'utf8')),null);
 assert.ok(events.some(e=>e.kind==='fault_started'&&e.message.startsWith('SIMULATED')));assert.ok(events.some(e=>e.kind==='fault_restored'));
});
test('restart recovers a durable simulated fault',async()=>{
 await writeFile(path.join(temporary,'fault-ledger.json'),JSON.stringify({jobId:'old',target:{id:'demo',label:'Demo',adapter:'simulated'},action:'outage',before:{ready:true}}));
 const events=[];await initializeFaults((id,kind)=>events.push(kind));assert.equal(faultStatus().active,null);assert.ok(events.includes('fault_restored'));
});
test('acceptance cannot call missing metrics or incomplete recording a pass',()=>{
 const run={complete:true,successfulTps:10,offeredTps:10,errorRate:0,confirmP99:100,dropped:0,gaps:0};
 assert.equal(evaluateAcceptance(run).verdict,'pass');assert.equal(evaluateAcceptance({...run,complete:false}).verdict,'incomplete');
 assert.equal(evaluateAcceptance({...run,confirmP99:null}).verdict,'unmeasured');assert.equal(evaluateAcceptance({...run,dropped:1}).verdict,'fail');
 assert.equal(evaluateAcceptance({...run,activity:[{kind:'fault_started',message:'SIMULATED · test'},{kind:'fault_restored'}]}).resilience,'simulated');
 assert.equal(evaluateAcceptance({...run,activity:[{kind:'fault_error'}]}).resilience,'incomplete');
});

test('local configuration rejects invalid ports, binaries and relative repositories',()=>{
 assert.throws(()=>validateTargets([{id:'v',adapter:'local-vault',port:3001,repository:'relative'}]),/absolute/);
 assert.throws(()=>validateTargets([{id:'db',adapter:'process-compose',port:5432,controllerPort:8090,process:'db',binary:'/bin/sh'}]),/binary/);
 assert.throws(()=>validateTargets([{id:'db',adapter:'process-compose',port:0,controllerPort:8090,process:'db',binary:'process-compose'}]),/port/);
});
test('process-compose stops only the named process and restores it; array JSON supported',async()=>{
 const {localAdapter}=await import('../server/local-faults.mjs');const calls=[];let status='Running',listening=true;
 const adapter=localAdapter({adapter:'process-compose',binary:'process-compose',controllerPort:8090,port:5432,process:'p1'},async(binary,args)=>{calls.push([binary,args]);if(args.includes('list'))return JSON.stringify([{name:'p1',status},{name:'keeper',status:'Running'}]);if(args.includes('get'))return JSON.stringify([{name:'p1',namespace:'default',status}]);if(args.includes('stop')){status='Stopped';listening=false;}if(args.includes('start')){status='Running';listening=true;}return '';},{reachable:async()=>listening});
 const before=await adapter.probe();await adapter.apply('outage',before);assert.ok(await adapter.isDown(before));assert.deepEqual(calls.find(c=>c[1].includes('stop')),['process-compose',['-p','8090','process','-a','127.0.0.1','stop','p1']]);await adapter.restore(before);assert.ok(await adapter.verify(before));
 await assert.rejects(adapter.apply('outage',{identity:{name:'different'}}),/identity changed/);
});
test('vault checks identity before signalling and recovers through its launcher',async()=>{
 const {localAdapter}=await import('../server/local-faults.mjs');let pid=1234,listening=true,launches=0,kills=0;
 const adapter=localAdapter({id:'vault',adapter:'local-vault',repository:temporary,port:3001},async(binary,args)=>binary==='lsof'?(args.includes('-t')?`${pid}\n`:`fcwd\nn${temporary}\nftxt\nn${temporary}/target/debug/locker\n`):'start-time',{reachable:async()=>listening,kill:()=>{kills++;listening=false;},launch:async()=>{launches++;listening=true;}});
 const before=await adapter.probe();pid=2345;await assert.rejects(adapter.apply('outage',before),/identity changed/);assert.equal(kills,0);
 pid=1234;await adapter.apply('outage',before);assert.equal(kills,1);assert.ok(await adapter.isDown());await adapter.restore(before);assert.equal(launches,1);await adapter.restore(before);assert.equal(launches,1);assert.ok(await adapter.verify());
});
test('local targets persist and remain separate from server-defined targets',async()=>{
 const {saveLocalFaultTargets,localFaultTargets,faultTargets}=await import('../server/faults.mjs');
 const targets=[{id:'vault-test',adapter:'local-vault',port:3001,repository:temporary}];await saveLocalFaultTargets(targets);assert.equal(localFaultTargets()[0].id,'vault-test');assert.ok(faultTargets().some(t=>t.id==='demo'));await saveLocalFaultTargets([]);
});
test('process-compose refuses to stop its last running process and restore is idempotent',async()=>{
 const {localAdapter}=await import('../server/local-faults.mjs');const calls=[],info={name:'only',namespace:'default',status:'Running'};
 const adapter=localAdapter({adapter:'process-compose',binary:'process-compose',controllerPort:8090,port:5432,process:'only'},async(b,args)=>{calls.push(args);return JSON.stringify([info]);},{reachable:async()=>true});
 const before=await adapter.probe();await assert.rejects(adapter.apply('outage',before),/another running process/);await adapter.restore(before);assert.equal(calls.some(args=>args.includes('stop')||args.includes('start')),false);
});
test('one local target per service type, regardless of ID, label or port',()=>{
 const vault={id:'vault',adapter:'local-vault',repository:temporary,port:3001};
 const db={id:'db',adapter:'process-compose',serviceType:'postgres',binary:'process-compose',controllerPort:8090,process:'p1',port:5432};
 assert.throws(()=>validateTargets([vault,{...vault,id:'other',port:3002}]),/Only one card-vault/);
 assert.throws(()=>validateTargets([db,{...db,id:'other',label:'Renamed',port:5555}]),/Only one postgres/);
 const redis={...db,id:'redis',serviceType:'redis',process:'r1',port:6379};
 assert.throws(()=>validateTargets([redis,{...redis,id:'redis-two'}]),/Only one redis/);
 assert.equal(validateTargets([vault,db,redis]).length,3);
 assert.equal(validateTargets([{...redis,serviceType:undefined}])[0].serviceType,'redis');
 assert.throws(()=>validateTargets([{...db,serviceType:'card-vault'}]),/Invalid fault service type/);
});

test('vault recovery resolves an executable Nix before stopping a service',async()=>{
 const {nixExecutable}=await import('../server/local-faults.mjs');
 const bin=await mkdtemp(path.join(os.tmpdir(),'loadtest-nix-test-'));await writeFile(path.join(bin,'nix'),'#!/bin/sh\nexit 0\n',{mode:0o700});
 const previous=process.env.PATH;try{process.env.PATH=bin;assert.equal(await nixExecutable(),path.join(bin,'nix'));}finally{process.env.PATH=previous;}
});
test('vault without a flake uses its existing executable and rejects unrecoverable targets',async()=>{
 const {vaultLaunchSpec}=await import('../server/local-faults.mjs');
 const {mkdir}=await import('node:fs/promises');
 const repository=await mkdtemp(path.join(os.tmpdir(),'vault-launch-'));
 await writeFile(path.join(repository,'Cargo.toml'),'[package]');
 await assert.rejects(vaultLaunchSpec({repository}),/Build the vault/);
 await mkdir(path.join(repository,'target','debug'),{recursive:true});
 const binary=path.join(repository,'target','debug','locker');await writeFile(binary,'#!/bin/sh\nexit 0\n',{mode:0o700});
 assert.deepEqual(await vaultLaunchSpec({repository}),{binary,args:[]});
});

test('cloud presets persist, reject duplicates and invoke managed AlloyDB failover with fixed argv',async()=>{
 const {configureFaultTarget,faultEnvironment}=await import('../src/fault-types.mjs');
 const {saveLocalFaultTargets,localFaultTargets}=await import('../server/faults.mjs');
 const target=configureFaultTarget({id:'postgres',serviceType:'postgres',label:'PostgreSQL'},'cloud');
 assert.equal(faultEnvironment(target),'cloud');
 await saveLocalFaultTargets([target]);
 assert.equal(localFaultTargets()[0].actions[0],'failover');
 assert.equal(localFaultTargets()[0].instance,'dev-hyperswitch-alloydb-primary');
 const local=configureFaultTarget({...target,id:'other'},'local');
 assert.throws(()=>validateTargets([target,local]),/Only one postgres/);
 assert.throws(()=>validateTargets([{...target,project:'x;echo bad'}]),/Invalid/);
 assert.throws(()=>validateTargets([{...target,serviceType:'redis'}]),/does not match/);
 const calls=[];const info={createTime:'original',state:'READY',availabilityType:'REGIONAL',instanceType:'PRIMARY'};
 const adapter=makeFaultAdapter(target,async(binary,args)=>{calls.push([binary,args]);return JSON.stringify(info);});
 const before=await adapter.probe();assert.ok(calls.every(c=>c[1].includes('describe')));
 await adapter.apply('failover',before);
 assert.deepEqual(calls.at(-1),['gcloud',['alloydb','instances','failover','dev-hyperswitch-alloydb-primary','--project','hyperswitch-dev','--region','asia-south1','--cluster','dev-hyperswitch-alloydb','--quiet']]);
 info.state='MAINTENANCE';assert.equal(await adapter.verify(before),false);
 info.state='READY';assert.equal(await adapter.verify(before),true);
 info.createTime='replacement';assert.equal(await adapter.verify(before),false);
 await saveLocalFaultTargets([]);
});

test('switching environments retains local values and custom target configuration',async()=>{
 const {configureFaultTarget,faultEnvironment}=await import('../src/fault-types.mjs');
 const original={id:'db',serviceType:'postgres',adapter:'process-compose',port:5555,process:'mydb',controllerPort:8091,binary:'process-compose'};
 const cloud=configureFaultTarget(original,'cloud');
 const custom=configureFaultTarget({...cloud,container:'db-test'},'custom');
 assert.equal(custom.adapter,'docker');assert.equal(faultEnvironment(custom),'custom');
 assert.equal(validateTargets([custom])[0].container,'db-test');
 const local=configureFaultTarget(custom,'local');
 assert.equal(local.port,5555);assert.equal(local.process,'mydb');assert.equal(local.controllerPort,8091);
});

test('cloud vault defaults scale down and restore original replicas; restart uses rollout',async()=>{
 const {configureFaultTarget}=await import('../src/fault-types.mjs');
 const target=configureFaultTarget({id:'vault',serviceType:'card-vault'},'cloud');
 assert.equal(target.namespace,'hyperswitch');
 assert.equal(target.deployment,'hyperswitch-card-vault-hyperswitch-card-vault');
 validateTargets([target]);
 const calls=[],info={metadata:{uid:'vault',generation:1},spec:{replicas:3},status:{availableReplicas:3,updatedReplicas:3,observedGeneration:1}};
 const adapter=makeFaultAdapter(target,async(binary,args)=>{calls.push([binary,args]);return JSON.stringify(info);});
 const before=await adapter.probe();await adapter.apply('outage',before);
 assert.deepEqual(calls.at(-1),['kubectl',['--namespace','hyperswitch','scale','deployment',target.deployment,'--replicas=0']]);
 await adapter.restore(before);
 assert.deepEqual(calls.at(-1),['kubectl',['--namespace','hyperswitch','scale','deployment',target.deployment,'--replicas=3']]);
 await adapter.apply('restart',before);
 assert.deepEqual(calls.at(-1),['kubectl',['--namespace','hyperswitch','rollout','restart','deployment',target.deployment]]);
 const custom=configureFaultTarget({...target,namespace:'other',deployment:'custom-vault'},'cloud');
 assert.equal(custom.namespace,'other');assert.equal(custom.deployment,'custom-vault');
});

test('switching existing local targets to cloud exposes supplied command defaults',async()=>{
 const {configureFaultTarget,faultCommandPreview}=await import('../src/fault-types.mjs');
 const local=[{id:'vault',adapter:'local-vault',serviceType:'card-vault',repository:temporary,port:3001},{id:'db',adapter:'process-compose',serviceType:'postgres',binary:'process-compose',process:'p1',controllerPort:8090,port:5432,instance:'',project:'',region:'',cluster:''}];
 const targets=local.map(t=>configureFaultTarget(t,'cloud'));
 validateTargets(targets);
 assert.equal(faultCommandPreview(targets[0]),'Outage: kubectl -n hyperswitch scale deploy hyperswitch-card-vault-hyperswitch-card-vault --replicas=0\nRestart: kubectl -n hyperswitch rollout restart deploy hyperswitch-card-vault-hyperswitch-card-vault');
 assert.equal(faultCommandPreview(targets[1]),'gcloud alloydb instances failover dev-hyperswitch-alloydb-primary --cluster=dev-hyperswitch-alloydb --region=asia-south1 --project=hyperswitch-dev --quiet');
});
