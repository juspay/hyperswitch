import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,chmod,mkdir} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {rootCertificates} from 'node:tls';
import {resolveK6Binary} from '../server/k6-binary.mjs';
import {testServiceEndpoint,testSuperpositionEndpoint} from '../server/service-checks.mjs';
import {databaseConnectionOptions} from '../server/database-options.mjs';
import {importCertificate,listCertificates,readImportedCertificate,removeCertificate} from '../server/certificates.mjs';

test('k6-sqlite resolution honors configuration, source build, PATH and gives an actionable error',async()=>{
  const root=await mkdtemp(path.join(os.tmpdir(),'k6-resolution-')),source=path.join(root,'k6/scenario-mix/bin/k6-sqlite');
  await mkdir(path.dirname(source),{recursive:true});
  await writeFile(source,'#!/bin/sh\n');await chmod(source,0o700);
  assert.equal(await resolveK6Binary({loadtestRoot:root,consoleRoot:path.join(root,'console'),pathValue:''}),source);
  const configured=path.join(root,'configured');await writeFile(configured,'#!/bin/sh\n');await chmod(configured,0o700);
  assert.equal(await resolveK6Binary({configured,loadtestRoot:'/missing',consoleRoot:'/missing',pathValue:''}),configured);
  await assert.rejects(resolveK6Binary({configured:'/missing',loadtestRoot:'/missing',consoleRoot:'/missing',pathValue:''}),/npm run build:recorder/);
});

test('service endpoint checks are read-only, timed and reject unhealthy responses',async()=>{
  let request;
  const healthy=await testServiceEndpoint('router','https://router.example/base',{fetcher:async(url,options)=>{request={url:String(url),options};return new Response('{}',{status:200});}});
  assert.equal(request.url,'https://router.example/base/health');assert.equal(request.options.method,'GET');assert.equal(healthy.ok,true);
  let prefixed;
  await testServiceEndpoint('modularPm','http://127.0.0.1:8081/v2',{fetcher:async(url)=>{prefixed=String(url);return new Response('{}',{status:200});}});
  assert.equal(prefixed,'http://127.0.0.1:8081/v2/health');
  await assert.rejects(testServiceEndpoint('superposition','https://sp.example',{fetcher:async()=>new Response('',{status:503})}),/503/);
  await assert.rejects(testServiceEndpoint('unknown','https://sp.example',{fetcher:async()=>new Response()}),/Unknown/);
});

test('Superposition checks authenticate by reading a configured default',async()=>{
  let request;
  const config={endpoint:'https://sp.example/base',token:'secret',access_header:'x-superposition-access',organization_id:'org',workspace_id:'dev',defaults:{flag:true}};
  const healthy=await testSuperpositionEndpoint(config,{fetcher:async(url,options)=>{request={url:String(url),options};return new Response('{}',{status:200});}});
  assert.equal(request.url,'https://sp.example/default-config/flag');
  assert.equal(request.options.headers['x-superposition-access'],'secret');assert.equal(request.options.headers['x-org-id'],'org');
  assert.equal(healthy.ok,true);
});

test('database TLS verifies system or supplied certificate authorities',()=>{
  const database={host:'db.example',port:5432,database:'hyperswitch',user:'reader',password:'secret'};
  assert.equal(databaseConnectionOptions({...database,tlsMode:'disable'}).ssl,false);
  assert.deepEqual(databaseConnectionOptions({...database,tlsMode:'require'}).ssl,{rejectUnauthorized:false});
  assert.deepEqual(databaseConnectionOptions({...database,tlsMode:'verify'}).ssl,{rejectUnauthorized:true});
  const configured=databaseConnectionOptions({...database,tlsMode:'verify',caPath:'/certs/db.pem'},{readCa:()=> '-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----'});
  assert.equal(configured.ssl.rejectUnauthorized,true);assert.match(configured.ssl.ca,/BEGIN CERTIFICATE/);
  assert.throws(()=>databaseConnectionOptions({...database,tlsMode:'verify',caPath:'relative.pem'}),/absolute/);
  assert.throws(()=>databaseConnectionOptions({...database,tlsMode:'verify',caPath:'/certs/bad'},{readCa:()=> 'not a certificate'}),/PEM/);
});

test('certificate bundles are validated, stored by opaque ID and usable by database TLS',async()=>{
  const root=await mkdtemp(path.join(os.tmpdir(),'loadtest-certificates-'));
  const certificate=await importCertificate(root,{name:'Test trust',pem:rootCertificates[0]});
  assert.equal(listCertificates(root)[0].id,certificate.id);
  assert.match(certificate.fingerprint,/^[0-9A-F:]+$/);
  const configured=databaseConnectionOptions({host:'db',port:5432,database:'db',user:'reader',password:'secret',tlsMode:'verify',caCertificateId:certificate.id},{readCertificate:id=>readImportedCertificate(root,id)});
  assert.match(configured.ssl.ca,/BEGIN CERTIFICATE/);
  await removeCertificate(root,certificate.id);assert.deepEqual(listCertificates(root),[]);
  await assert.rejects(importCertificate(root,{name:'Bad',pem:'not a certificate'}),/PEM encoded/);
});
