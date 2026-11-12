export function fixtureSteps(job){
 const progress=job?.progress||{},setup=progress.setup||{},sp=setup.superposition||{};
 const done=job?.status==='complete'||setup.stage==='ready',terminal=['failed','cancelled','interrupted'].includes(job?.status);
 let phase=sp.status||'pending',remaining=sp.remainingSeconds;
 // Older generators have no structured setup status; their labelled output still
 // identifies the refresh wait without treating a full merchant bar as readiness.
 if(!progress.setup){const output=job?.output||'';if(output.includes('Verify refreshed flags'))phase='verifying';else if(output.includes('Service refresh')){phase='waiting';const matches=[...output.matchAll(/Service refresh[^\n]*\|\s*(\d+)s remaining/g)];remaining=Number(matches.at(-1)?.[1]??0);}else if(output.includes('Resolve flags'))phase='resolving';}
 const merchantsDone=done||Number(progress.fixtures?.completed)>=Number(progress.fixtures?.total)&&Number(progress.fixtures?.total)>0;
 const rank={pending:0,applying:1,resolving:1,waiting:2,verifying:3,complete:4,skipped:4}[phase]??0;
 const status=(step)=>{if(done)return phase==='skipped'&&step>0?'skipped':'complete';if(phase==='skipped'&&step>0)return 'skipped';let value=step===0?(merchantsDone?'complete':'running'):rank>step?'complete':rank===step?'running':'pending';if(terminal||setup.stage==='failed')value=value==='running'?'failed':value;return value;};
 return [
  {label:'Merchants',status:status(0),detail:`${progress.fixtures?.completed??0} / ${progress.fixtures?.total??0} provisioned`},
  {label:'Superposition defaults',status:status(1),detail:phase==='resolving'?'Checking resolved flags':phase==='applying'?'Applying configured flags':''},
  {label:'Service refresh',status:status(2),detail:phase==='waiting'?`${remaining??0}s remaining`:'',remaining,total:sp.totalSeconds},
  {label:'Readiness',status:status(3),detail:phase==='verifying'?'Verifying refreshed flags':''}
 ];
}
