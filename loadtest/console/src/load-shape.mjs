// Shared, credential-free summary for saved load plans and the editor.
export function loadShape(load={}){
 if(Array.isArray(load.stages)&&load.stages.length){
  let duration=0,area=0,start=load.start_rps ?? load.stages[0].target_rps,peak=start;
  for(const stage of load.stages){duration+=stage.duration_seconds;area+=(start+stage.target_rps)/2*stage.duration_seconds;peak=Math.max(peak,stage.target_rps);start=stage.target_rps;}
  return {duration,peak,average:duration?area/duration:0};
 }
 return {duration:load.duration_seconds || 0,peak:load.total_rps || 0,average:load.total_rps || 0};
}

// Only display fields leave the server; connector/controller credentials stay private.
export function loadTimeline(load={},faultPlan=[]){
 let at=0,from=load.start_rps ?? load.stages?.[0]?.target_rps ?? load.total_rps ?? 0;
 const stages=load.stages?.length?load.stages:[{target_rps:load.total_rps||0,duration_seconds:load.duration_seconds||0}];
 return stages.map((stage,index)=>{
  const end=at+stage.duration_seconds,target=stage.target_rps;
  const fault=stage.fault?{action:stage.fault.action,target:faultPlan.find(p=>p.index===index)?.target?.label||stage.fault.targetId}:null;
  const row={index,start:at,end,from,target,kind:fault?'fault':from===target?'hold':'ramp',fault};at=end;from=target;return row;
 });
}
