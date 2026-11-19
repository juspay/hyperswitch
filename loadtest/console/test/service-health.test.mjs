import test from 'node:test';
import assert from 'node:assert/strict';
import {faultHealthOverride,pendingHealth} from '../src/service-health.mjs';
const database={id:'database',label:'DB'};
const targets=[{id:'postgres',adapter:'gcp-alloydb',serviceType:'postgres',label:'PostgreSQL'}];
const fault=active=>({active:{targetId:'postgres',target:'PostgreSQL',action:'failover',status:'active',...active}});
test('DB represents failover before and after the command completes without claiming an outage',()=>{
 for(const occurred of [false,true])assert.equal(faultHealthOverride(database,fault({occurred}),targets).state,'recovering');
 assert.equal(faultHealthOverride(database,fault({status:'restoring'}),targets).state,'recovering');
 assert.equal(faultHealthOverride(database,fault({status:'recovery_failed'}),targets).state,'offline');
 assert.equal(faultHealthOverride(database,{active:null},targets),null);
});
test('database outage, legacy targets, and unrelated services stay distinct',()=>{
 assert.equal(faultHealthOverride(database,fault({action:'outage',occurred:true}),targets).state,'offline');
 assert.equal(faultHealthOverride(database,fault({}),[{id:'postgres',label:'PostgreSQL'}]).state,'recovering');
 assert.equal(faultHealthOverride({id:'router'},fault({}),targets),null);
 assert.equal(faultHealthOverride(database,fault({simulated:true}),targets),null);
});
test('DB health uses SQL availability while the provider operation is still restoring',()=>{
 assert.equal(faultHealthOverride(database,fault({status:'restoring',databaseAvailability:'available'}),targets).state,'online');
 assert.equal(faultHealthOverride(database,fault({status:'restoring',databaseAvailability:'unavailable'}),targets).state,'offline');
});
test('failed card-vault recovery falls back to a current readiness probe',()=>{
 const service={id:'fault-vault',targetId:'vault'};
 const vaultFault={active:{targetId:'vault',target:'Card vault',status:'recovery_failed',action:'outage',occurred:true}};
 assert.equal(faultHealthOverride(service,vaultFault,[{id:'vault',serviceType:'card-vault'}]),null);
});
test('background refresh preserves the measured result; fault overrides apply immediately',()=>{
 for(const state of ['online','offline'])assert.equal(pendingHealth(database,{state,message:'Measured'},null).state,state);
 assert.equal(pendingHealth(database,null,null).state,'checking');
 assert.equal(pendingHealth(database,{state:'online'},faultHealthOverride(database,fault({}),targets)).state,'recovering');
});
