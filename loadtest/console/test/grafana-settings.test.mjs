import test from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {grafanaConnection,normalizeGrafanaPanel,prepareGrafanaSettings,migrateGrafanaPanels,testGrafanaConnection,resolveGrafanaPanel} from '../server/grafana-settings.mjs';
import {configuredGrafanaPanels} from '../src/grafana-panels.mjs';

test('panel links preserve variables, strip transient and secret parameters, and normalize dashboard links',()=>{
 const panel=normalizeGrafanaPanel('https://pasted.invalid/d/test/demo?viewPanel=panel-4&var-app=router&var-app=pm&from=123&to=now&refresh=1s&token=secret','https://configured.invalid');
 assert.equal(panel.url,'/grafana/d-solo/test/demo?panelId=4&var-app=router&var-app=pm');
 for(const url of ['javascript:alert(1)','https://user:secret@x/d/test/demo?panelId=1','https://x/d/test/demo','https://x/d/test/demo?panelId=NaN','https://x/api/user?panelId=1'])assert.throws(()=>normalizeGrafanaPanel(url,'https://grafana.invalid'));
});
test('tokens are retained only for the exact configured server and never followed to a new one',()=>{
 const saved={baseUrl:'https://old.invalid',token:'old-viewer',credentialOrigin:'https://old.invalid'};
 assert.equal(grafanaConnection({baseUrl:saved.baseUrl,token:''},saved).token,'old-viewer');
 assert.throws(()=>grafanaConnection({baseUrl:'https://new.invalid'},saved),/token/);
 assert.equal(grafanaConnection({baseUrl:'https://new.invalid',token:'new-viewer'},saved).token,'new-viewer');
 assert.throws(()=>grafanaConnection({baseUrl:'http://169.254.169.254',token:'x'},saved),/metadata/);
 assert.throws(()=>grafanaConnection({baseUrl:saved.baseUrl,token:'bad\r\nheader'},saved),/Invalid/);
});
test('legacy defaults migrate once; deliberately empty lists persist and duplicate panels are rejected',()=>{
 assert.equal(migrateGrafanaPanels({panels:[{url:'/grafana/d-solo/k6/k6?panelId=17'}]}).length,3);
 assert.deepEqual(migrateGrafanaPanels({presetsVersion:2,panels:[]}),[]);
 const saved={baseUrl:'https://grafana.invalid',token:'viewer',credentialOrigin:'https://grafana.invalid'};
 const panel={title:'Latency',url:'/grafana/d-solo/test/demo?panelId=4'};
 assert.throws(()=>prepareGrafanaSettings({...saved,panels:[panel,panel]},saved),/already added/);
 const first=prepareGrafanaSettings({...saved,panels:[panel]},saved);
 assert.equal(prepareGrafanaSettings({...first,token:''},first).revision,first.revision);
 assert.deepEqual(prepareGrafanaSettings({...first,token:'',panels:[]},first).panels,[]);
});
test('custom panels use run bounds and preserve explicit empty lists and identical titles',()=>{
 const run={startedAt:1000,endedAt:5000,loadState:'completed'};
 const panels=[1,2].map(id=>({title:'CPU',url:`/grafana/d-solo/test/demo?panelId=${id}&var-instance=db&from=stale&refresh=1s`}));
 const result=configuredGrafanaPanels(run,panels);
 assert.equal(result.length,2);
 for(const panel of result){const url=new URL(panel.url,'http://console');assert.equal(url.searchParams.get('from'),'1000');assert.equal(url.searchParams.get('to'),'5000');assert.equal(url.searchParams.get('var-instance'),'db');assert.equal(url.searchParams.has('refresh'),false);}
 assert.deepEqual(configuredGrafanaPanels(run,[]),[]);
});
test('connection and panel validation use the configured server, authenticate, and reject absent panels',async()=>{
 const requests=[];
 const server=createServer((req,res)=>{
  requests.push({path:req.url,authorization:req.headers.authorization});res.setHeader('content-type','application/json');
  if(req.url==='/api/health')return res.end('{"database":"ok"}');
  if(req.headers.authorization!=='Bearer viewer'){res.statusCode=401;return res.end('{}');}
  if(req.url==='/api/user')return res.end('{"id":0,"orgId":1}');
  if(req.url==='/api/dashboards/uid/test')return res.end(JSON.stringify({dashboard:{title:'Metrics',panels:[{id:10,type:'row',panels:[{id:4,type:'stat',title:'Latency'}]}]}}));
  res.statusCode=404;res.end('{}');
 });
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 try{
  const connection={baseUrl:`http://127.0.0.1:${server.address().port}`,token:'viewer'};
  assert.equal((await testGrafanaConnection(connection)).ok,true);
  const panel=await resolveGrafanaPanel(connection,'https://not-fetched.invalid/d/test/demo?panelId=4');
  assert.equal(panel.title,'Latency');assert.equal(panel.dashboardTitle,'Metrics');
  await assert.rejects(resolveGrafanaPanel(connection,'/d/test/demo?panelId=10'),/not found/);
  await assert.rejects(resolveGrafanaPanel(connection,'/d/test/demo?panelId=999'),/not found/);
  await assert.rejects(testGrafanaConnection({...connection,token:'wrong'}),/denied/);
  assert.ok(requests.every(r=>r.path.startsWith('/api/')));
 }finally{server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
});

test('saved Grafana credentials and panels are environment-scoped and redacted from public settings',async()=>{
 const {mkdtemp}=await import('node:fs/promises');
 process.env.LOADTEST_DATA_DIR=await mkdtemp('/private/tmp/grafana-settings-state-');
 const state=await import('../server/state.mjs');state.initializeConfig();
 const local=state.publicSettings();
 await state.saveConnections({...local,grafana:{...local.grafana,baseUrl:'https://local-grafana.invalid',token:'local-private-viewer'}});
 assert.equal(state.localSettings().grafana.token,'local-private-viewer');
 const aws=state.publicSettings('aws');
 await state.saveConnections({...aws,grafana:{...aws.grafana,baseUrl:'https://aws-grafana.invalid',token:'aws-private-viewer',panels:[]}});
 assert.deepEqual(state.publicSettings().grafana.panels,[]);
 assert.equal(state.publicSettings().grafana.hasToken,true);
 assert.equal(state.environmentGrafana('local').token,'local-private-viewer');
 assert.equal(state.environmentGrafana('aws').token,'aws-private-viewer');
 assert.equal(state.publicSettings('local').grafana.panels.length,3);
 assert.equal(state.publicSettings().grafana.token,'');
 assert.doesNotMatch(JSON.stringify(state.publicSettings()),/private-viewer/);
 assert.equal(state.redact('local-private-viewer aws-private-viewer'),'[redacted] [redacted]');
 const draft={...state.publicSettings().grafana,token:'',baseUrl:'https://different.invalid'};
 await assert.rejects(state.saveConnections({...state.publicSettings(),grafana:draft}),/token/);
 assert.equal(state.localSettings().grafana.baseUrl,'https://aws-grafana.invalid');
});
