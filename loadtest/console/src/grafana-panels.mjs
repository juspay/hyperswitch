import {pausedChartSamples} from './chart-timeline.mjs';
const dashboard='/d-solo/loadtest-unified-perf/load-test-e28094-unified-performance';
export function unifiedPerformancePanels(run){
 if(!run?.startedAt)return [];
 const paused=run.loadState==='paused';
 const frozen=pausedChartSamples(run.telemetry||[],run.loadState).at(-1);
 const end=run.endedAt||(paused&&frozen?run.startedAt+frozen.t*1000:paused?run.startedAt+Math.max(0,run.wallSeconds||run.activeSeconds||0)*1000:'now');
 return [[4,'Payment Confirm p99 Latency'],[2,'RPS (for router service)'],[3,'Error Rate % (5xx)']].map(([id,title])=>{
  const query=new URLSearchParams({panelId:String(id),from:String(run.startedAt),to:String(end),timezone:'browser',theme:'light',
   'var-datasource':'prometheus','var-namespace':'hyperswitch','var-app':'hyperswitch-router-hyperswitch-server',
   'var-path':'$__all','var-status_code':'$__all','var-table':'$__all','var-operation':'$__all','var-pod':'$__all',
   'var-alloydb_cluster':'.*','var-alloydb_instance':'.*','var-redis_instance':'.*'});
  if(!run.endedAt&&!paused)query.set('refresh','1s');
  return {title,url:`/grafana${dashboard}?${query}`};
 });
}
