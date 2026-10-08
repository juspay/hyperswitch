import {publicSettings,runDirectory} from './state.mjs';
import {listProfiles,listFixtureSets} from './profiles.mjs';
import {workerCommand} from './supervisor.mjs';
import {queryWorker} from './workers.mjs';
export async function screenData(url,screen){
  const profiles=listProfiles(),fixtures=listFixtureSets();
  const settings=publicSettings(),profileId=url.searchParams.get('profile') || profiles[0]?.id;
  const job=await workerCommand('active',{kind:screen==='performance'?'load':screen==='fixtures'?'generator':null});
  if(screen==='scenarios'||screen==='fixtures')return {settings,profiles,fixtures,profileId,job};
  const runs=await queryWorker('list');
  const selected=url.searchParams.get('run') || (screen==='performance'?job?.command==='load'?job.runId:null:runs[0]?.id);
  const run=selected?await queryWorker('run',{id:selected}):null;
  const reconciliationJob=screen==='results'&&selected?await workerCommand('latest-reconciliation',{path:runDirectory(selected).path}):null;
  const generator=await workerCommand('active',{kind:'generator'});
  return {settings,profiles,fixtures,profileId,reconciliationJob,busy:generator!=null,job:job?.command==='load'?job:null,runs,run,fixtureSetId:url.searchParams.get('fixtures') || job?.fixtureSetId};
}
