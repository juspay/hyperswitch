export const defaultCriteria={minimumSuccessRatio:.95,maximumErrorPercent:1,maximumConfirmP99Ms:2000,maximumDropped:0};
export function evaluateAcceptance(run,criteria=defaultCriteria){
 const limit={...defaultCriteria,...criteria};
 const check=(label,actual,maximum,minimum=false)=>({label,actual,limit:maximum,outcome:actual==null?'unmeasured':(minimum?actual>=maximum:actual<=maximum)?'pass':'fail'});
 const checks=[check('Successful TPS',run.successfulTps,(run.expectedAverageTps ?? run.offeredTps)*limit.minimumSuccessRatio,true),
  check('HTTP errors (%)',run.errorRate,limit.maximumErrorPercent),check('Confirm p99 (ms)',run.confirmP99,limit.maximumConfirmP99Ms),
  check('Dropped flows',run.dropped,limit.maximumDropped),check('Recording gaps',run.gaps,0)];
 const faults=run.activity?.filter(e=>e.kind==='fault_started') || [];
 const faultErrors=run.activity?.some(e=>e.kind==='fault_error');
 const recovered=run.activity?.filter(e=>e.kind==='fault_restored').length || 0;
 const resilience=faultErrors||recovered<faults.length?'incomplete':faults.length?faults.some(e=>e.message.startsWith('SIMULATED'))?'simulated':'recovery verified':'not tested';
 const verdict=(!run.complete||['cancelled','interrupted'].includes(run.jobStatus))?'incomplete':checks.some(c=>c.outcome==='fail')?'fail':checks.some(c=>c.outcome==='unmeasured')?'unmeasured':'pass';
 return {verdict,checks,resilience,criteria:limit};
}
