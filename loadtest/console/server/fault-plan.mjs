// Resolved once before launch; target edits cannot redirect a running scenario.
export function snapshotFaultPlan(load,targets){
 let at=0;const plan=[];
 for(const [index,stage] of (load.stages || []).entries()){
  if(stage.fault){const target=targets.find(t=>t.id===stage.fault.targetId);
   if(!target)throw Error(`Timeline stage ${index+1}: fault target “${stage.fault.targetId}” is no longer configured`);
   const action=target.actions.includes(stage.fault.action)?stage.fault.action:target.actions.length===1&&(stage.fault.action==='restart'||(stage.fault.action==='outage'&&target.actions[0]==='failover'))?target.actions[0]:null;
   if(!action)throw Error(`Timeline stage ${index+1}: ${target.label} does not support “${stage.fault.action}”; choose ${target.actions.join(' or ')}`);
   plan.push({index,at,end:at+stage.duration_seconds,target:structuredClone(target),action,durationSeconds:stage.duration_seconds});
  }at+=stage.duration_seconds;
 }return plan;
}
export function faultScheduler(plan,{claim,fire,event,gate=async()=>{},release=async()=>{},recover=release}){
 let busy=false;
 return async(job,seconds)=>{
  if(busy||job.loadState!=='running'||!Number.isFinite(seconds))return;
  busy=true;
  try{for(const step of plan){
   if(seconds<step.at||!claim(step.index))continue;
   if(seconds>=step.end){event('fault_error',`Step ${step.index+1}: missed fault window; no infrastructure changed`);continue;}
   try{
    await gate();
    await fire(job,{targetId:step.target.id,action:step.action,durationSeconds:step.durationSeconds,confirm:`${step.action} ${step.target.id}`},step.target,{
     // Keep the fault stage active until recovery is verified. The controller
     // automatically re-enters the gate at its configured end boundary.
     onStarted:()=>release(step.end),
     onRestore:gate,
     onRecovered:recover,
     onAborted:()=>recover(step.end)
    });
   }
   catch(e){await recover().catch(()=>{});event('fault_error',`Step ${step.index+1}: ${e.message}`);}
  }}finally{busy=false;}
 };
}
