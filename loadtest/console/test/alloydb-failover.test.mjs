import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
process.env.LOADTEST_DATA_DIR=await mkdtemp(path.join(os.tmpdir(),'alloydb-failover-'));
const {makeFaultAdapter}=await import('../server/faults.mjs');
const target={adapter:'gcp-alloydb',project:'test',region:'asia-south1',cluster:'payments',instance:'primary'};
const instanceName='projects/test/locations/asia-south1/clusters/payments/instances/primary';
const operationName='projects/test/locations/asia-south1/operations/operation-123';
function fixture(){
 const info={createTime:'original',state:'READY',availabilityType:'REGIONAL',instanceType:'PRIMARY'};
 const operation={name:operationName,done:false,metadata:{verb:'failover',target:instanceName}};
 const calls=[];let reads=0,reachable=true,submissionError=null;
 const adapter=makeFaultAdapter(target,async(binary,args)=>{
  calls.push(args);
  if(args.includes('failover')){if(submissionError)throw submissionError;return JSON.stringify({name:operationName});}
  return JSON.stringify(args.includes('operations')?operation:info);
 },{databaseCheck:async()=>{reads++;if(!reachable)throw new Error('connection refused');return {ok:true};}});
 return {adapter,operation,info,calls,get reads(){return reads;},set reachable(value){reachable=value;},set submissionError(value){submissionError=value;}};
}
test('READY during a long failover cannot release the next stage',async()=>{
 const f=fixture(),before=await f.adapter.probe(),saved=[];
 await f.adapter.apply('failover',before,async()=>saved.push(structuredClone(before)));
 assert.equal(saved[0].failoverRequested,true);assert.equal(saved[0].operation,undefined);
 assert.equal(saved[1].operation,operationName);
 assert.ok(f.calls.find(args=>args.includes('failover')).includes('--async'));
 // Model the observed 148-second operation, exceeding the old 120-second CLI deadline.
 for(let seconds=0;seconds<=145;seconds+=5)assert.equal(await f.adapter.verify(before),false);
 assert.equal(f.reads,0);assert.ok(f.adapter.recoveryTimeoutMs>148000);
 f.operation.done=true;assert.equal(await f.adapter.verify(before),true);assert.equal(f.reads,1);
});
test('successful operation still waits for DB connectivity and original instance identity',async()=>{
 const f=fixture(),before=await f.adapter.probe();await f.adapter.apply('failover',before);f.operation.done=true;
 f.reachable=false;assert.equal(await f.adapter.verify(before),false);
 f.reachable=true;f.info.state='MAINTENANCE';assert.equal(await f.adapter.verify(before),false);
 f.info.state='READY';f.info.createTime='replacement';assert.equal(await f.adapter.verify(before),false);
 f.info.createTime='original';assert.equal(await f.adapter.verify(before),true);
});
test('terminal failed operation releases the stage without reporting recovery',async()=>{
 const f=fixture(),before=await f.adapter.probe();
 await assert.rejects(f.adapter.verify(before),/operation ID is missing/);
 await f.adapter.apply('failover',before);f.operation.done=true;f.operation.error={message:'failover rejected'};
 assert.equal(await f.adapter.verify(before),true);assert.match(before.failoverFailed,/failover rejected/);assert.equal(f.reads,0);
 delete f.operation.error;f.operation.metadata.target=instanceName+'-other';
 await assert.rejects(f.adapter.verify(before),/identity changed/);
});
test('lost submission response remains unverified; persisted operation resumes without retriggering',async()=>{
 const f=fixture(),before=await f.adapter.probe();f.submissionError=new Error('CLI timeout');
 let saved;await assert.rejects(f.adapter.apply('failover',before,async()=>saved=structuredClone(before)),/CLI timeout/);
 assert.equal(saved.failoverRequested,true);await assert.rejects(f.adapter.verify(saved),/operation ID is missing/);
 assert.equal(saved.failoverRejected,undefined);
 f.submissionError=null;await f.adapter.apply('failover',before,async()=>saved=structuredClone(before));
 const resumed=fixture();resumed.operation.done=true;
 assert.equal(await resumed.adapter.verify(saved),true);
 assert.equal(resumed.calls.some(args=>args.includes('failover')),false);
});
test('explicit API rejection is persisted so it cannot enter recovery',async()=>{
 const f=fixture(),before=await f.adapter.probe();
 f.submissionError=new Error('ERROR: (gcloud.alloydb.instances.failover) ABORTED: another failover operation was recently executed');
 let saved;await assert.rejects(f.adapter.apply('failover',before,async()=>saved=structuredClone(before)),/ABORTED/);
 assert.match(saved.failoverRejected,/another failover/);assert.equal(saved.operation,undefined);
});
