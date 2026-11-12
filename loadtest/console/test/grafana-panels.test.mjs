import test from 'node:test';
import assert from 'node:assert/strict';
import {unifiedPerformancePanels} from '../src/grafana-panels.mjs';
import {pausedChartSamples} from '../src/chart-timeline.mjs';
test('requested Grafana panels use the selected run range and Router-only filtering',()=>{
 const panels=unifiedPerformancePanels({startedAt:1000,endedAt:301000});
 assert.deepEqual(panels.map(p=>new URL(p.url,'http://console').searchParams.get('panelId')),['4','2','3']);
 for(const p of panels){const u=new URL(p.url,'http://console');assert.ok(u.pathname.startsWith('/grafana/d-solo/'));assert.equal(u.searchParams.get('from'),'1000');assert.equal(u.searchParams.get('to'),'301000');assert.equal(u.searchParams.get('var-app'),'hyperswitch-router-hyperswitch-server');assert.equal(u.searchParams.has('refresh'),false);}
});
test('paused cards and native charts stop at zero throughput and resume with live data',()=>{
 const telemetry=[{t:10,state:'running',successfulTps:5},{t:11,state:'paused',successfulTps:1,inFlight:0},{t:12,state:'paused',successfulTps:0,inFlight:0},{t:30,state:'paused',successfulTps:0,inFlight:0}];
 assert.equal(pausedChartSamples(telemetry,'paused').at(-1).t,12);
 assert.equal(pausedChartSamples(telemetry,'running').at(-1).t,30);
 const run={startedAt:1000,loadState:'paused',telemetry};
 const before=unifiedPerformancePanels(run),later=unifiedPerformancePanels({...run,telemetry:[...telemetry,{t:40,state:'paused',successfulTps:0,inFlight:0}]});
 assert.deepEqual(before,later);
 const url=new URL(before[0].url,'http://console');assert.equal(url.searchParams.get('to'),'13000');assert.equal(url.searchParams.has('refresh'),false);
 const resumed=new URL(unifiedPerformancePanels({...run,loadState:'running'})[0].url,'http://console');assert.equal(resumed.searchParams.get('to'),'now');assert.equal(resumed.searchParams.get('refresh'),'1s');
});
test('live panels refresh without changing iframe URLs on every progress update',()=>{
 const run={startedAt:1000};
 const panels=unifiedPerformancePanels(run);
 assert.equal(new URL(panels[0].url,'http://console').searchParams.get('to'),'now');
 assert.equal(new URL(panels[0].url,'http://console').searchParams.get('refresh'),'1s');
 assert.deepEqual(unifiedPerformancePanels({...run,iterations:100}),panels);
 assert.deepEqual(unifiedPerformancePanels(null),[]);
});
