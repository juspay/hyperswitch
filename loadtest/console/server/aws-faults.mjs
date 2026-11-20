// AWS operations use fixed argv; credentials come from the supervisor's AWS CLI identity.
export function validateAwsTarget(target){
 if(!['aws-rds','aws-elasticache'].includes(target.adapter))throw new Error('Unsupported AWS failover adapter');
 if(!/^\d{12}$/.test(target.accountId||''))throw new Error('Enter the 12-digit AWS account ID');
 if(!/^[a-z]{2}(?:-[a-z]+)+-\d+$/.test(target.region||''))throw new Error('Enter an AWS region');
 if(target.profile&&!/^[A-Za-z0-9][A-Za-z0-9_.@+-]{0,127}$/.test(target.profile))throw new Error('Invalid AWS CLI profile');
 const resource=target.adapter==='aws-rds'?target.instance:target.replicationGroupId;
 if(!/^[a-z][a-z0-9-]{0,62}$/.test(resource||''))throw new Error('Enter a valid AWS resource identifier');
 if(target.adapter==='aws-elasticache'&&!/^\d{4}$/.test(target.nodeGroupId||''))throw new Error('Enter the four-digit ElastiCache shard ID');
}

// RDS has no failover operation ID. Correlate service events to the exact resource
// and persisted request time; readiness or an AZ change alone cannot prove completion.
export function awsFaultAdapter(target,command,{databaseCheck=async(_timeoutMillis)=>({ok:false}),now=Date.now}={}){
 validateAwsTarget(target);
 const rds=target.adapter==='aws-rds',resource=rds?target.instance:target.replicationGroupId;
 const service=rds?'rds':'elasticache',sourceType=rds?'db-instance':'replication-group';
 const flags=['--region',target.region,...(target.profile?['--profile',target.profile]:[]),'--output','json','--no-cli-pager','--cli-connect-timeout','5','--cli-read-timeout','20'];
 const aws=async(service,operation,args=[])=>JSON.parse(await command('aws',[service,operation,...args,...flags]));
 const describe=async()=>{
  const result=await aws(service,rds?'describe-db-instances':'describe-replication-groups',[rds?'--db-instance-identifier':'--replication-group-id',resource]);
  const info=(rds?result.DBInstances:result.ReplicationGroups)?.[0];
  if(!info)throw new Error('AWS resource was not found');return info;
 };
 const arn=info=>rds?info.DBInstanceArn:info.ARN;
 const identity=info=>rds?info.DbiResourceId:info.ReplicationGroupCreateTime;
 const primary=info=>info.NodeGroups?.find(g=>g.NodeGroupId===target.nodeGroupId)?.NodeGroupMembers?.find(m=>m.CurrentRole==='primary')?.CacheClusterId;
 const assertResource=(info,expectedArn)=>{
  if(arn(info)!==expectedArn||(rds?info.DBInstanceIdentifier:info.ReplicationGroupId)!==resource)throw new Error('AWS resource identity changed');
 };
 const ready=info=>rds?info.DBInstanceStatus==='available':info.Status==='available'&&info.NodeGroups?.find(g=>g.NodeGroupId===target.nodeGroupId)?.Status==='available';
 const events=async(before,type=sourceType,id=resource)=>(await aws(service,'describe-events',['--source-identifier',id,'--source-type',type,'--start-time',new Date(before.requestedAt).toISOString()])).Events||[];
 const recent=(event,before,type=sourceType,id=resource)=>event.SourceIdentifier===id&&event.SourceType===type&&Number.isFinite(Date.parse(event.Date))&&Date.parse(event.Date)>=before.requestedAt;
 return {
  recoveryTimeoutMs:600000,
  ...(rds?{checkAvailability:()=>databaseCheck(2000)}:{}),
  // Check account, immutable identity and HA prerequisites without changing infrastructure.
  probe:async()=>{
   const caller=await aws('sts','get-caller-identity');
   if(caller.Account!==target.accountId)throw new Error('AWS CLI is authenticated to a different account');
   const partition=/^arn:([^:]+):/.exec(caller.Arn||'')?.[1];
   if(!partition)throw new Error('AWS caller identity is missing its ARN');
   const expectedArn=`arn:${partition}:${service}:${target.region}:${target.accountId}:${rds?'db':'replicationgroup'}:${resource}`;
   const info=await describe();assertResource(info,expectedArn);
   if(!identity(info)||!ready(info))throw new Error('AWS failover target must be available');
   if(rds){
    if(info.Engine!=='postgres'||!info.MultiAZ||info.DBClusterIdentifier)throw new Error('Choose an RDS PostgreSQL Multi-AZ DB instance; Aurora and DB clusters need custom commands');
   }else{
    if(info.ClusterEnabled)throw new Error('ElastiCache cluster mode enabled requires custom commands');
    const shard=info.NodeGroups?.find(g=>g.NodeGroupId===target.nodeGroupId);
    if(info.AutomaticFailover!=='enabled'||info.MultiAZ!=='enabled'||!primary(info)||!shard?.NodeGroupMembers?.some(m=>m.CurrentRole==='replica'))throw new Error('Choose an ElastiCache HA shard with automatic failover and a replica');
   }
   return {ready:true,arn:expectedArn,created:identity(info),...(rds?{}:{primary:primary(info)})};
  },
  // Persist intent before submitting so restart recovery cannot accidentally retrigger a failover.
  apply:async(action,before,persist=async()=>{})=>{
   if(action!=='failover'||!before?.ready)throw new Error('AWS failover requires a successful target check');
   if(before.failoverRequested)throw new Error('AWS failover was already requested; verify recovery without retriggering');
   try{
    const current=await describe();assertResource(current,before.arn);
    if(identity(current)!==before.created||!ready(current))throw new Error('AWS failover target changed after preflight');
   }catch(error){before.failoverRejected='AWS preflight failed; no failover request was sent';await persist();throw error;}
   before.failoverRequested=true;before.requestedAt=now();await persist();
   let response;
   try{response=await aws(service,rds?'reboot-db-instance':'test-failover',rds?['--db-instance-identifier',resource,'--force-failover']:['--replication-group-id',resource,'--node-group-id',target.nodeGroupId]);}
   catch(error){
    const code=/An error occurred \((AccessDenied(?:Exception)?|UnauthorizedOperation|InvalidDBInstanceState(?:Fault)?|DBInstanceNotFound(?:Fault)?|InvalidReplicationGroupState(?:Fault)?|ReplicationGroupNotFound(?:Fault)?|TestFailoverNotAvailableFault|InvalidParameterValue|InvalidParameterCombination)\)/.exec(error.message)?.[1];
    if(code){before.failoverRejected=`AWS rejected failover: ${code}`;await persist();}throw error;
   }
   assertResource(rds?response.DBInstance:response.ReplicationGroup,before.arn);
   before.failoverAccepted=true;await persist();
  },
  restore:async()=>{}, // Managed failovers recover themselves; never issue another mutation here.
  // Verify post-request cloud evidence and the original resource, then check SQL for RDS.
  verify:async before=>{
   if(!before?.failoverRequested||!Number.isFinite(before.requestedAt))throw new Error('AWS failover request evidence is missing');
   const info=await describe();assertResource(info,before.arn);
   if(identity(info)!==before.created)throw new Error('AWS resource was replaced during failover');
   const completed=(await events(before)).filter(e=>recent(e,before)&&(!rds||e.SourceArn===before.arn));
   if(rds){
    if(completed.some(e=>/abandoning user requested failover/i.test(e.Message||''))){
     if(!ready(info))return false;
     before.failoverFailed='AWS RDS did not perform the requested failover';return true;
    }
    if(!ready(info)||!info.MultiAZ||!completed.some(e=>/multi-az (?:instance failover completed|failover to standby complete)/i.test(e.Message||'')))return false;
    try{return (await databaseCheck(2000)).ok===true;}catch{return false;}
   }
   if(!ready(info)||info.AutomaticFailover!=='enabled'||info.MultiAZ!=='enabled'||!primary(info)||primary(info)===before.primary||!completed.some(e=>/failover from primary node .+ to replica node .+ completed/i.test(e.Message||'')))return false;
   // AWS documents node replacement recovery as a separate cache-cluster event.
   return (await events(before,'cache-cluster',before.primary)).some(e=>recent(e,before,'cache-cluster',before.primary)&&/^Finished recovery for cache nodes /i.test(e.Message||''));
  }
 };
}
