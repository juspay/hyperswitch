import { publicSettings } from '../../server/state.mjs';
import { queryWorker } from '../../server/workers.mjs';
import { activeJob } from '../../server/jobs.mjs';
export async function load() {
  const runs=await queryWorker('list');
  return { settings:publicSettings(),runs,run:runs.length?await queryWorker('run',{id:runs[0].id}):null,job:activeJob() };
}
