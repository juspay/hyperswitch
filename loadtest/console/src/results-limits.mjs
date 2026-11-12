// Shared by Settings and the run evaluator; values are copied into each new run.
export const defaultResultsLimits={
  minimumThroughputPercent:97,maximumHttpErrorPercent:.1,
  maximumHoldP99Ms:160,maximumRampP99Ms:500,maximumRecoveryP99Ms:300,maximumLatencyDriftPercent:10,
  maximumRecoverySeconds:60,recoveryWindowSeconds:10,maximumLostPayments:0
};
export const resultsLimitFields=[
  ['minimumThroughputPercent','Achieved throughput','% of offered',100],
  ['maximumHttpErrorPercent','Normal-stage HTTP errors','%',100],
  ['maximumHoldP99Ms','Hold server-side p99','ms'],
  ['maximumRampP99Ms','Ramp server-side p99','ms'],
  ['maximumRecoveryP99Ms','After recovery payment-confirm p99','ms'],
  ['maximumLatencyDriftPercent','Hold latency drift','%'],
  ['maximumRecoverySeconds','Recovery after restoration starts','s'],
  ['recoveryWindowSeconds','Transition settling time','s'],
  ['maximumLostPayments','Acknowledged payments lost','payments']
];
export function validateResultsLimits(input={}){
  if(!input||typeof input!=='object'||Array.isArray(input))throw new Error('Invalid Results limits');
  return Object.fromEntries(resultsLimitFields.map(([key,label,,max])=>{
    const raw=input[key]??defaultResultsLimits[key],value=Number(raw);
    if(raw===''||!['number','string'].includes(typeof raw)||!Number.isFinite(value)||value<0||(max!==undefined&&value>Number(max))||
      (key==='recoveryWindowSeconds'&&(value<=0||value>3600))||(key==='maximumLostPayments'&&!Number.isSafeInteger(value)))throw new Error(`Invalid Results limit: ${label}`);
    return [key,value];
  }));
}
