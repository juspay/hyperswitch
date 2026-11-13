export const importBusy=job=>['starting','running','cancelling'].includes(job?.status);
export function matchesImport(job,profileId){const id=job?.progress?.verification?.profileId;return Boolean(job?.progress?.verification)&&(!id||id===profileId);}
// A slower fallback poll or an old EventSource must not overwrite newer progress.
export function acceptImportUpdate(current,incoming,profileId){
 if(!current||current.id!==incoming?.id||!matchesImport(incoming,profileId))return false;
 if(!importBusy(current))return false;
 const rank={credentials:0,setup:1,saving:2};
 if(importBusy(incoming)&&(rank[incoming.progress.verification.phase]??0)<(rank[current.progress.verification.phase]??0))return false;
 const count=job=>job.progress?.verification?.checked??job.progress?.verification?.checks?.length??0;
 return !importBusy(incoming)||count(incoming)>=count(current);
}
export function importView(job){
 const verification=job?.progress?.verification;if(!verification)return null;
 const busy=importBusy(job),checks=verification.checks||[],failures=checks.filter(c=>!c.ok);
 const phase=busy?(verification.phase||'credentials'):job.status==='complete'&&verification.ok!==false?'ready':['cancelled','interrupted'].includes(job.status)?'cancelled':'failed';
 const phases=['credentials','setup'];
 const stages=phases.map((id,index)=>{
  const saved=verification.stages?.[id],rows=checks.filter(c=>c.stage===id);
  let status=saved?.status;
  if(!status){status=rows.some(c=>!c.ok)?'failed':phase==='ready'?'passed':phase==='cancelled'?'blocked':phase==='failed'?(index===0&&failures[0]?.stage==='setup'?'passed':'blocked'):phase===id?'running':index===0&&phase!=='credentials'?'passed':'pending';}
  return {id,label:index===0?'Credentials':'Connector setup',status,completed:saved?.completed??rows.length,total:saved?.total??null};
 });
 stages.push({id:'ready',label:'Ready',status:phase==='ready'?'passed':phase==='saving'?'running':['failed','cancelled'].includes(phase)?'blocked':'pending',completed:0,total:null});
 const first=failures[0],title=phase==='ready'?'Verified':phase==='cancelled'?'Verification cancelled':phase==='failed'?(first?.stage==='setup'?'Setup verification failed':first?'Credential verification failed':'Verification failed'):job.status==='cancelling'?'Cancelling…':phase==='setup'?'Verifying connectors…':phase==='saving'?'Preparing fixture…':phase==='failed'?'Finishing verification…':'Verifying credentials…';
 const detail=phase==='failed'?(first?`${first.merchant} · ${first.label} — ${first.message}`:job.error||'Verification could not complete'):phase==='cancelled'?'Edit the inputs and try again.':'';
 return {phase,busy,title,detail,checks,failures,stages,fixtureSetId:verification.fixtureSetId||job.fixtureSetId,profileId:verification.profileId};
}
