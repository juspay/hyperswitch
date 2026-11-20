import test from 'node:test';
import assert from 'node:assert/strict';
import {awsFaultAdapter,validateAwsTarget} from '../server/aws-faults.mjs';
import {configureFaultTarget,faultCommandPreview,faultAccessNote} from '../src/fault-types.mjs';
import {validateTargetProvider} from '../src/environments.mjs';

const account='123456789012',region='ap-south-1',requestedAt=Date.parse('2026-11-20T00:00:00Z');
const rdsTarget={id:'postgres',environment:'aws',serviceType:'postgres',adapter:'aws-rds',accountId:account,region,profile:'loadtest',instance:'payments'};
const redisTarget={id:'redis',environment:'aws',serviceType:'redis',adapter:'aws-elasticache',accountId:account,region,replicationGroupId:'payments-cache',nodeGroupId:'0001'};
function fixture(target=rdsTarget){
 const rds=target.adapter==='aws-rds';
 const resource=rds?target.instance:target.replicationGroupId;
 const arn=`arn:aws:${rds?'rds':'elasticache'}:${region}:${account}:${rds?'db':'replicationgroup'}:${resource}`;
 const info=rds?{DBInstanceIdentifier:resource,DBInstanceArn:arn,DbiResourceId:'db-original',Engine:'postgres',MultiAZ:true,DBInstanceStatus:'available'}:
 {ReplicationGroupId:resource,ARN:arn,ReplicationGroupCreateTime:'2026-01-01T00:00:00Z',Status:'available',MultiAZ:'enabled',AutomaticFailover:'enabled',NodeGroups:[{NodeGroupId:'0001',Status:'available',NodeGroupMembers:[{CurrentRole:'primary',CacheClusterId:'primary'},{CurrentRole:'replica',CacheClusterId:'replica'}]}]};
 const f={info,events:[],cacheEvents:[],callerAccount:account,reachable:true,calls:[],error:null};
 f.adapter=awsFaultAdapter(target,async(binary,args)=>{
  assert.equal(binary,'aws');f.calls.push(args);
  if(args.includes('get-caller-identity'))return JSON.stringify({Account:f.callerAccount,Arn:`arn:aws:iam::${f.callerAccount}:user/test`});
  if(args.includes('describe-events'))return JSON.stringify({Events:args.includes('cache-cluster')?f.cacheEvents:f.events});
  if(args.includes('reboot-db-instance')||args.includes('test-failover')){if(f.error)throw f.error;return JSON.stringify(rds?{DBInstance:info}:{ReplicationGroup:info});}
  return JSON.stringify(rds?{DBInstances:[info]}:{ReplicationGroups:[info]});
 },{now:()=>requestedAt,databaseCheck:async()=>({ok:f.reachable})});
 f.complete=()=>f.events.push({SourceIdentifier:resource,SourceType:rds?'db-instance':'replication-group',SourceArn:arn,Date:new Date(requestedAt+1000).toISOString(),Message:rds?'Multi-AZ instance failover completed.':'Failover from primary node primary to replica node replica completed'});
 return f;
}
test('AWS targets default to provider-specific built-in failovers and previews',()=>{
 for(const [serviceType,adapter]of [['postgres','aws-rds'],['redis','aws-elasticache']]){
  const target=configureFaultTarget({serviceType},'aws');assert.equal(target.adapter,adapter);assert.deepEqual(target.actions,['failover']);
  assert.equal(validateTargetProvider(target,serviceType),'aws');assert.match(faultCommandPreview(target),/aws (rds reboot-db-instance|elasticache test-failover)/);assert.match(faultAccessNote(target),/AWS CLI/);
 }
 assert.equal(configureFaultTarget({serviceType:'card-vault'},'aws').adapter,'placeholder');
 assert.match(faultCommandPreview({...rdsTarget}),/--profile=loadtest/);
});
test('AWS field validation rejects option injection and missing account/resource identity',()=>{
 for(const change of [{accountId:''},{accountId:'123'},{region:'x;true'},{profile:'--endpoint-url=x'},{instance:'--all'}])assert.throws(()=>validateAwsTarget({...rdsTarget,...change}));
 assert.throws(()=>validateAwsTarget({...redisTarget,nodeGroupId:'--all'}));
 assert.throws(()=>validateTargetProvider({...rdsTarget,environment:'gcp'},'postgres'),/does not support/);
});
test('AWS RDS preflight is read-only and rejects wrong account, single-AZ, clusters and non-Postgres',async()=>{
 const f=fixture();await f.adapter.probe();assert.ok(f.calls.every(args=>!args.includes('reboot-db-instance')));
 f.callerAccount='999999999999';await assert.rejects(f.adapter.probe(),/different account/);f.callerAccount=account;
 f.info.MultiAZ=false;await assert.rejects(f.adapter.probe(),/Multi-AZ/);f.info.MultiAZ=true;
 f.info.DBClusterIdentifier='aurora';await assert.rejects(f.adapter.probe(),/DB clusters/);delete f.info.DBClusterIdentifier;
 f.info.Engine='mysql';await assert.rejects(f.adapter.probe(),/PostgreSQL/);
});
test('RDS persists request intent before a scoped fixed-argv force failover',async()=>{
 const f=fixture(),before=await f.adapter.probe(),saved=[];await f.adapter.apply('failover',before,async()=>saved.push(structuredClone(before)));
 assert.equal(saved[0].requestedAt,requestedAt);assert.equal(saved[0].failoverAccepted,undefined);assert.equal(saved[1].failoverAccepted,true);
 const args=f.calls.find(args=>args.includes('reboot-db-instance'));assert.deepEqual(args.slice(0,5),['rds','reboot-db-instance','--db-instance-identifier','payments','--force-failover']);assert.ok(args.includes(region));assert.ok(args.includes('loadtest'));
 await assert.rejects(f.adapter.apply('failover',before),/already requested/);
 await f.adapter.restore(before);assert.equal(f.calls.filter(args=>args.includes('reboot-db-instance')).length,1);
});
test('RDS readiness, old events, unrelated resources and AZ changes cannot establish recovery',async()=>{
 const f=fixture(),before=await f.adapter.probe();await f.adapter.apply('failover',before);
 f.info.AvailabilityZone='changed';assert.equal(await f.adapter.verify(before),false);
 f.complete();const event=f.events[0];event.Date=new Date(requestedAt-1000).toISOString();assert.equal(await f.adapter.verify(before),false);
 event.Date=new Date(requestedAt+1000).toISOString();event.SourceArn+='-other';assert.equal(await f.adapter.verify(before),false);event.SourceArn=before.arn;
 f.reachable=false;assert.equal(await f.adapter.verify(before),false);f.reachable=true;
 assert.equal(await f.adapter.verify(before),true);f.info.DbiResourceId='replacement';await assert.rejects(f.adapter.verify(before),/replaced/);
});
test('lost AWS response resumes event verification without submitting another request',async()=>{
 const f=fixture(),before=await f.adapter.probe();f.error=new Error('network timeout');let saved;
 await assert.rejects(f.adapter.apply('failover',before,async()=>saved=structuredClone(before)),/timeout/);assert.equal(saved.failoverRejected,undefined);
 const resumed=fixture();resumed.complete();assert.equal(await resumed.adapter.verify(saved),true);assert.ok(resumed.calls.every(args=>!args.includes('reboot-db-instance')));
 await assert.rejects(resumed.adapter.verify({}),/evidence is missing/);
});
test('AWS explicit rejection and abandoned failover are recorded as failures',async()=>{
 const f=fixture(),before=await f.adapter.probe();f.error=new Error('An error occurred (InvalidDBInstanceState) when calling the RebootDBInstance operation');
 await assert.rejects(f.adapter.apply('failover',before),/InvalidDBInstanceState/);assert.match(before.failoverRejected,/InvalidDBInstanceState/);
 const g=fixture(),other=await g.adapter.probe();await g.adapter.apply('failover',other);g.complete();g.events[0].Message='Abandoning user requested failover since a failover recently occurred on the database instance.';
 assert.equal(await g.adapter.verify(other),true);assert.match(other.failoverFailed,/did not perform/);
});
test('changed AWS resource between preflight and trigger is never mutated',async()=>{
 const f=fixture(),before=await f.adapter.probe();f.info.DBInstanceArn+='-other';await assert.rejects(f.adapter.apply('failover',before),/identity changed/);
 assert.equal(f.calls.some(args=>args.includes('reboot-db-instance')),false);
});
test('ElastiCache preflight requires HA, requested shard and a replica',async()=>{
 const f=fixture(redisTarget);await f.adapter.probe();f.info.ClusterEnabled=true;await assert.rejects(f.adapter.probe(),/custom commands/);f.info.ClusterEnabled=false;f.info.AutomaticFailover='disabled';await assert.rejects(f.adapter.probe(),/HA shard/);f.info.AutomaticFailover='enabled';
 f.info.NodeGroups[0].NodeGroupMembers.pop();await assert.rejects(f.adapter.probe(),/replica/);
});
test('ElastiCache requires changed primary and failover plus cache-node recovery events',async()=>{
 const f=fixture(redisTarget),before=await f.adapter.probe();await f.adapter.apply('failover',before);f.complete();
 assert.equal(await f.adapter.verify(before),false);
 f.info.NodeGroups[0].NodeGroupMembers.reverse();for(const member of f.info.NodeGroups[0].NodeGroupMembers)member.CurrentRole=member.CacheClusterId==='replica'?'primary':'replica';
 assert.equal(await f.adapter.verify(before),false);
 f.cacheEvents.push({SourceIdentifier:'primary',SourceType:'cache-cluster',Date:new Date(requestedAt+1000).toISOString(),Message:'Finished recovery for cache nodes 0001'});
 assert.equal(await f.adapter.verify(before),true);
 const args=f.calls.find(args=>args.includes('test-failover'));assert.deepEqual(args.slice(0,6),['elasticache','test-failover','--replication-group-id','payments-cache','--node-group-id','0001']);
});
