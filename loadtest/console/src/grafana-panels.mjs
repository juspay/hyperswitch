import {pausedChartSamples} from './chart-timeline.mjs';
const dashboard='/grafana/d-solo/loadtest-unified-perf/load-test-e28094-unified-performance';
export function defaultGrafanaPanels(){
 return [[4,'Payment Confirm p99 Latency'],[2,'RPS (for router service)'],[3,'Error Rate % (5xx)']].map(([id,title])=>({title,url:`${dashboard}?${new URLSearchParams({panelId:String(id),'var-datasource':'prometheus','var-namespace':'hyperswitch','var-app':'hyperswitch-router-hyperswitch-server','var-path':'$__all','var-status_code':'$__all','var-table':'$__all','var-operation':'$__all','var-redis_instance':'.*'})}`}));
}
export function configuredGrafanaPanels(run,panels=defaultGrafanaPanels()){
 if(!run?.startedAt)return [];
 const paused=run.loadState==='paused',frozen=pausedChartSamples(run.telemetry||[],run.loadState).at(-1);
 const end=run.endedAt||(paused&&frozen?run.startedAt+frozen.t*1000:paused?run.startedAt+Math.max(0,run.wallSeconds||run.activeSeconds||0)*1000:'now');
 return panels.map(panel=>{const url=new URL(panel.url,'http://console');url.searchParams.set('from',String(run.startedAt));url.searchParams.set('to',String(end));url.searchParams.set('timezone','browser');url.searchParams.set('theme','light');url.searchParams.delete('refresh');if(!run.endedAt&&!paused)url.searchParams.set('refresh','1s');return {...panel,url:url.pathname+url.search};});
}
export const unifiedPerformancePanels=run=>configuredGrafanaPanels(run);
