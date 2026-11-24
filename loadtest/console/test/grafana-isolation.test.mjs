import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {createServer} from 'node:http';
process.env.LOADTEST_DATA_DIR=await mkdtemp(path.join(os.tmpdir(),'grafana-isolation-'));
process.env.ORIGIN='http://127.0.0.1:4174';process.env.GRAFANA_EMBED_ORIGIN='http://127.0.0.1:4176';
const state=await import('../server/state.mjs');state.initializeConfig();
const auth=await import('../server/auth.mjs');
const {createGrafanaEmbed,grafanaEmbedRequest}=await import('../server/grafana-embed.mjs');
test('active Grafana content is isolated with one-time tickets and revocable sessions',async()=>{
 let headers,hits=0;
 const upstream=createServer((req,res)=>{hits++;headers=req.headers;if(req.url==='/api/health'){res.setHeader('content-type','application/json');res.end('{"database":"ok"}');}else{res.setHeader('content-type','text/html');res.end('<script>/* untrusted upstream */</script>');}});
 await new Promise(r=>upstream.listen(0,'127.0.0.1',r));
 try{
  process.env.GRAFANA_URL=`http://127.0.0.1:${upstream.address().port}`;process.env.GRAFANA_TOKEN='fake-viewer-token';
  await state.atomicJson(state.localPath,{grafana:{baseUrl:process.env.GRAFANA_URL}});
  process.env.ADMIN_PASSWORD_HASH=await auth.passwordHash('fake-test-password');
  const session=await auth.login('admin','fake-test-password','grafana-test');
  const {url}=createGrafanaEmbed(session,'/grafana/d-solo/test/panel?panelId=1');
  assert.notEqual(new URL(url).origin,process.env.ORIGIN);
  const handshake=await grafanaEmbedRequest(new Request(url));assert.equal(handshake.status,303);
  assert.equal((await grafanaEmbedRequest(new Request(url))).status,401,'ticket cannot be replayed');
  const cookie=handshake.headers.get('set-cookie').split(';')[0],panel=new URL(handshake.headers.get('location'),url);
  assert.equal((await grafanaEmbedRequest(new Request(panel,{headers:{cookie:`loadtest_grafana=${new URL(url).searchParams.get('embed_ticket')}`}}))).status,401,'redeemed URL ticket is not a session credential');
  assert.match(handshake.headers.get('set-cookie'),/HttpOnly/);assert.equal(handshake.headers.get('referrer-policy'),'no-referrer');
  const response=await grafanaEmbedRequest(new Request(panel,{headers:{cookie}}));
  assert.equal(response.status,200);assert.match(await response.text(),/untrusted upstream/);
  assert.equal(headers.authorization,'Bearer fake-viewer-token');assert.equal(headers.cookie,undefined);
  assert.equal((await grafanaEmbedRequest(new Request(new URL('/api/settings',panel),{headers:{cookie}}))).status,404);
  assert.equal((await grafanaEmbedRequest(new Request(new URL('/grafana/api/ds/query',panel),{method:'POST',headers:{cookie,origin:process.env.ORIGIN},body:'{}'}))).status,403);
  const main=await import('../src/routes/grafana/[...path]/+server.js');assert.equal((await main.GET()).status,403);
  await state.atomicJson(state.localPath,{grafana:{baseUrl:'https://replacement.invalid'}});
  const before=hits;assert.equal((await grafanaEmbedRequest(new Request(panel,{headers:{cookie}}))).status,403);assert.equal(hits,before,'saved token cannot follow another origin');
  auth.logout(session);assert.equal((await grafanaEmbedRequest(new Request(panel,{headers:{cookie}}))).status,401);
 }finally{upstream.closeAllConnections();await new Promise(r=>upstream.close(r));}
});
