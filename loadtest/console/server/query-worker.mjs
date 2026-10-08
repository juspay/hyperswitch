import { parentPort, workerData } from 'node:worker_threads';
import { listRuns, describeRun, requestHistory, reconcileRun, testDatabase } from './queries.mjs';
try {
  const {task,args}=workerData;
  const result=task==='list'?await listRuns():task==='run'?await describeRun(args.id):task==='requests'?requestHistory(args)
    :task==='database-test'?await testDatabase(args.database):task==='reconcile'?await reconcileRun(args,p=>parentPort.postMessage({progress:p})):null;
  parentPort.postMessage({result});
} catch(e) {parentPort.postMessage({error:e.message});}
