// Resolved once before launch; target edits cannot redirect a running scenario.
export function snapshotFaultPlan(load,targets){
 let at=0;const plan=[];
 for(const [index,stage] of (load.stages || []).entries()){
  if(stage.fault){const target=targets.find(t=>t.id===stage.fault.targetId);
   if(!target||!target.actions.includes(stage.fault.action))throw Error(`Step ${index+1}: select a configured fault target and action`);
   plan.push({index,at,end:at+stage.duration_seconds,target:structuredClone(target),action:stage.fault.action,durationSeconds:stage.duration_seconds});
  }at+=stage.duration_seconds;
 }return plan;
}
export function faultScheduler(plan,{claim,fire,event}){
 let busy=false;
 return async(job,seconds)=>{
  if(busy||job.loadState!=='running'||!Number.isFinite(seconds))return;
  busy=true;
  try{for(const step of plan){
   if(seconds<step.at||!claim(step.index))continue;
   if(seconds>=step.end){event('fault_error',`Step ${step.index+1}: missed fault window; no infrastructure changed`);continue;}
   try{await fire(job,{targetId:step.target.id,action:step.action,durationSeconds:step.durationSeconds,confirm:`${step.action} ${step.target.id}`},step.target);}
   catch(e){event('fault_error',`Step ${step.index+1}: ${e.message}`);}
  }}finally{busy=false;}
 };
}
