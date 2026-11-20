import test from 'node:test';
import assert from 'node:assert/strict';
import {faultNotice} from '../src/fault-notice.mjs';
const health=state=>({state,checkedAt:Date.now()});
const active={target:'AWS RDS',action:'failover',status:'recovery_failed',databaseAvailability:'unavailable'};
test('fresh SQL health is separate from pending failover confirmation',()=>{
 const notice=faultNotice({active,blocked:true},{databaseHealth:health('online')});
 assert.equal(notice.title,'AWS RDS');assert.equal(notice.availability.label,'Available');assert.equal(notice.availability.tone,'positive');assert.equal(notice.description,'Failover confirmation pending');assert.equal(notice.retryLabel,'Recheck');
 const offline=faultNotice({active,blocked:true},{databaseHealth:health('offline')});assert.equal(offline.availability.label,'Unavailable');
});
test('a failed operation cannot turn its cached SQL sample into current health',()=>{
 for(const databaseAvailability of ['available','unavailable',undefined])assert.equal(faultNotice({active:{...active,databaseAvailability},blocked:true}).availability,null);
 assert.equal(faultNotice({active,blocked:true},{databaseHealth:{state:'online',checkedAt:Date.now()-61000}}).availability,null);
});
test('idle recovery renders nothing and blocked recovery without a target is safe',()=>{
 assert.equal(faultNotice({active:null,blocked:false}),null);assert.equal(faultNotice({active:null,blocked:true}).title,'Fault recovery');
});
test('restoration and stopping keep their actual service action',()=>{
 const faults={active:{target:'Card vault',action:'outage',status:'restoring'},blocked:true};
 assert.equal(faultNotice(faults).retryLabel,'Retry recovery');assert.match(faultNotice({...faults,blocked:false},{stopping:true}).description,/before stopping/);assert.equal(faultNotice({...faults,recovering:true}).retryLabel,'Recovering…');
 assert.doesNotMatch(faultNotice({active:{...active,action:'outage',status:'restoring'}}).description,/failover/);
});

test('DB health is not presented as card-vault availability',()=>{
 const notice=faultNotice({active:{target:'Card vault',action:'outage',status:'recovery_failed'},blocked:true},{databaseHealth:health('online')});
 assert.equal(notice.availability,null);
});

test('legacy AWS RDS display names avoid repeating the cloud provider',async()=>{
 const {faultDisplayName,faultTargetLabel}=await import('../src/fault-types.mjs');
 assert.equal(faultDisplayName('AWS · Sandbox RDS · restoring'),'AWS RDS · restoring');
 assert.equal(faultTargetLabel({label:'AWS RDS',environment:'aws'}),'AWS RDS');
});
