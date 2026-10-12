import {publicSettings,runDirectory} from './state.mjs';
import {listProfiles,listFixtureSets,profileDraft} from './profiles.mjs';
import {workerCommand} from './supervisor.mjs';
import {queryWorker} from './workers.mjs';
export async function screenData(url,screen){
  const settings=publicSettings(),profiles=listProfiles({includeArchived:screen==='scenarios'}),fixtures=listFixtureSets();
  const profileId=url.searchParams.get('profile') || profiles[0]?.id;
  const job=await workerCommand('active',{kind:['performance','fixtures'].includes(screen)?'generator':null});
  if(screen==='scenarios'||screen==='fixtures')return {settings,profiles,fixtures,profileId,job,profileTemplate:profileDraft(settings)};
  const runs=await queryWorker('list');
  const requested=url.searchParams.get('run');
  const selected=screen==='results'?(runs.some(run=>run.id===requested)?requested:runs[0]?.id):(runs.some(run=>run.id===requested)?requested:(job?.command==='load'?job.runId:null));
  const run=selected?await queryWorker('run',{id:selected}):null;
  const reconciliationJob=screen==='results'&&selected?await workerCommand('latest-reconciliation',{path:runDirectory(selected).path}):null;
  const generator=await workerCommand('active',{kind:'generator'});
  return {settings,profiles,fixtures,profileId,reconciliationJob,busy:generator!=null,job:['load','smoke'].includes(job?.command)?job:null,runs,run,fixtureSetId:url.searchParams.get('fixtures') || job?.fixtureSetId};
}
