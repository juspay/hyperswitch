import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtemp,readFile,readdir} from 'node:fs/promises';import path from 'node:path';import os from 'node:os';
import {connectorCatalog,normalizeConnectorPlan} from '../src/connector-config.mjs';
import {connectorSecretStore,provisionConnectorAccounts,resolveConnectorAccounts} from '../server/connector-fixtures.mjs';
import {connectorSchedule,selectedConnector,connectorRouting,connectorTestCard,connectorBrowserInfo} from '../../k6/scenario-mix/connectors.mjs';
import {DatabaseSync} from 'node:sqlite';import {connectorResults} from '../server/connector-results.mjs';
const plan={mode:'real',mix:[{connector:'stripe',weight:70},{connector:'adyen',weight:30}]};
test('catalog canonical names and strict mode, weights and identities',()=>{
 assert.ok(connectorCatalog.some(c=>c.id==='stripe_test'&&c.dummy));assert.ok(connectorCatalog.some(c=>c.id==='stripe'&&!c.dummy));assert.deepEqual(normalizeConnectorPlan(plan),plan);
 for(const invalid of [{...plan,mode:'dummy'},{...plan,mix:[{connector:'not-real',weight:100}]},{...plan,mix:[{connector:'stripe',weight:60}]},{...plan,mix:[{connector:'stripe',weight:50},{connector:'stripe',weight:50}]}])assert.throws(()=>normalizeConnectorPlan(invalid));
 assert.deepEqual(normalizeConnectorPlan({...plan,mix:plan.mix.map(r=>({...r,password:'must-not-save'}))}),plan);
});
test('weighted schedules preserve mix per cycle and merchant-specific account selection',()=>{
 const slots=connectorSchedule(plan);assert.equal(slots.filter(c=>c==='stripe').length,70);const merchant={connector_accounts:{stripe:'mca-s',adyen:'mca-a'}};
 for(let cycle=0;cycle<5;cycle++)assert.equal(Array.from({length:100},(_,i)=>selectedConnector(slots,cycle*100+i,merchant)).filter(c=>c.connector==='stripe').length,70);
 assert.throws(()=>selectedConnector(slots,0,{merchant_connector_id:'wrong-mca'}));
 assert.deepEqual(connectorRouting(selectedConnector(slots,0,merchant)),{routing:{type:'single',data:{connector:'stripe',merchant_connector_id:'mca-s'}}});
});
test('sandbox cards and browser context support provider requirements and explicit negative tests',()=>{
 const stripe={card_number:'4242424242424242'};
 assert.equal(connectorTestCard('stripe',stripe),stripe);
 assert.equal(connectorTestCard('adyen',stripe).card_number,'4111111111111111');
 assert.equal(connectorTestCard('adyen',stripe,{adyen:stripe}),stripe);
 assert.equal(connectorBrowserInfo({language:'fr-FR'}).language,'fr-FR');
 assert.ok(connectorBrowserInfo().accept_header);assert.equal(connectorBrowserInfo().java_enabled,false);
});
test('encrypted credentials never appear in references or plaintext files',async()=>{
 const root=await mkdtemp(path.join(os.tmpdir(),'connector-vault-'));const sentinel='sandbox-secret-should-never-be-saved-plaintext';
 const refs=provisionConnectorAccounts({mode:'real',mix:[{connector:'stripe',weight:100}]},{stripe:{credentials:{api_key:sentinel}}},root);
 assert.ok(!JSON.stringify(refs).includes(sentinel));for(const file of await readdir(path.join(root,'connector-secrets')))assert.ok(!(await readFile(path.join(root,'connector-secrets',file))).includes(Buffer.from(sentinel)));
 assert.equal(resolveConnectorAccounts(refs,root)[0].connector_account_details.api_key,sentinel);assert.throws(()=>connectorSecretStore(root).get('../key'));assert.throws(()=>provisionConnectorAccounts(plan,{stripe:{credentials:{api_key:sentinel}}},root),/Enter Adyen/);
});
test('Results separate assignments and reported routing, with HS-only and full latency',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'connector-results-'));const db=new DatabaseSync(path.join(directory,'results.sqlite'));db.exec(await readFile(new URL('../../k6/scenario-mix/recorder/schema.sql',import.meta.url),'utf8'));
 const insert=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,status,latency_ms,server_latency_ms,requested_connector,connector) VALUES('r',1,?,'payment_confirm','measured',1,'POST','/confirm',?,?,?,?,?,?)");
 insert.run('a',200,'succeeded',100,30,'stripe','stripe');insert.run('b',500,'failed',90,null,'adyen',null);insert.run('c',200,'succeeded',120,40,'stripe','adyen');db.close();
 const result=connectorResults(directory,plan,{latencyWindows:[{from:-1000,to:1000}]});assert.equal(result.unattributedConfirmations,1);const stripe=result.rows.find(r=>r.connector==='stripe');assert.equal(stripe.assigned,2);assert.equal(stripe.confirmations,1);assert.equal(stripe.observedPercent,50);assert.equal(stripe.hsP99,30);assert.equal(stripe.fullP99,100);assert.equal(result.rows.find(r=>r.connector==='adyen').successes,1);
});

test('connector totals reconcile outside-outage success and keep unreported HTTP failures inconclusive only for routing',async()=>{
 const {evaluateScenarioChecks}=await import('../server/scenario-checks.mjs');
 const {defaultResultsLimits}=await import('../src/results-limits.mjs');
 const directory=await mkdtemp(path.join(os.tmpdir(),'connector-outage-counts-')),db=new DatabaseSync(path.join(directory,'results.sqlite'));
 db.exec(await readFile(new URL('../../k6/scenario-mix/recorder/schema.sql',import.meta.url),'utf8'));
 const insert=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,status,latency_ms,requested_connector,connector) VALUES('r',1,?,?,'measured',?,'POST','/test',?,?,?,'stripe_test',?)");
 const rows=[];
 function record(flow,operation,at,code,status,latency,connector){insert.run(flow,operation,at,code,status,latency,connector);rows.push({flow_id:flow,operation,observed_at:at,api_status_code:code,status,latency_ms:latency});}
 for(let i=0;i<1445;i++)record('outside-'+i,'payment_confirm',5000,200,'succeeded',10,'stripe_test');
 for(let i=0;i<21;i++)record('outage-'+i,'payment_confirm',15000,200,'succeeded',10,'stripe_test');
 for(let i=0;i<2;i++)record('unknown-'+i,'payment_confirm',22000,500,null,3000,null);
 for(let i=0;i<20;i++)record('setup-'+i,i<10?'customer_create':'payment_create',16000,500,null,10,null);
 db.close();
 const config={load:{start_rps:5,stages:[{target_rps:5,duration_seconds:10},{target_rps:5,duration_seconds:10,fault:{targetId:'postgres',action:'failover'}},{target_rps:5,duration_seconds:10}]},resultsLimits:defaultResultsLimits};
 const events=[{at:11000,kind:'scheduled_fault',message:'scheduled step 2'},{at:11000,kind:'fault_requested'},{at:12000,kind:'fault_started'},{at:21000,kind:'fault_restored'}];
 const completion=evaluateScenarioChecks({config,recording:{run_id:'r',started_at:1000,ended_at:31000,recording_state:'complete'},events,samples:Array.from({length:31},(_,i)=>({at:1000+i*1000,active:i})),requests:rows,flows:[],jobStatus:'complete'}).completion;
 const result=connectorResults(directory,{mix:[{connector:'stripe_test',weight:100}]}),stripe=result.rows[0];
 assert.equal(stripe.assigned,1488);assert.equal(stripe.successes,1466);assert.equal(completion.successful,1445);assert.equal(completion.outageAttempts-completion.outageFailures,21);
 assert.equal(result.scope,'all_periods');
 assert.equal(result.inconclusive.confirmations,2);assert.equal(result.inconclusive.successes,0);assert.equal(result.inconclusive.failures,2);
 assert.equal(result.inconclusive.http5xx,2);assert.equal(result.inconclusive.connector,null);
 assert.equal(stripe.successes,completion.successful+completion.outageAttempts-completion.outageFailures);
 assert.equal(stripe.confirmations+result.inconclusive.confirmations,1468);
});

test('blank connector identities are inconclusive without changing known payment outcomes or routed percentages',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'connector-unknown-')),db=new DatabaseSync(path.join(directory,'results.sqlite'));
 db.exec(await readFile(new URL('../../k6/scenario-mix/recorder/schema.sql',import.meta.url),'utf8'));
 const insert=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,status,latency_ms,requested_connector,connector) VALUES('r',1,?,'payment_confirm','measured',1,'POST','/confirm',?,?,10,'stripe',?)");
 insert.run('known',200,'succeeded','stripe');insert.run('no-name',200,'succeeded','');insert.run('no-field',503,null,null);db.close();
 const result=connectorResults(directory,plan);
 assert.equal(result.rows[0].observedPercent,100);assert.equal(result.unattributedConfirmations,2);
 assert.equal(result.inconclusive.successes,1);assert.equal(result.inconclusive.failures,1);assert.equal(result.inconclusive.http5xx,1);
});

test('connector latency uses settled whole-flow windows and retains genuine steady-state spikes',async()=>{
 const directory=await mkdtemp(path.join(os.tmpdir(),'connector-steady-latency-')),db=new DatabaseSync(path.join(directory,'results.sqlite'));
 db.exec(await readFile(new URL('../../k6/scenario-mix/recorder/schema.sql',import.meta.url),'utf8'));
 const insert=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,operation,role,observed_at,method,url,api_status_code,status,latency_ms,server_latency_ms,requested_connector,connector) VALUES('r',1,?,?,'measured',?,'POST','/test',?,?,?,?,'stripe','stripe')");
 const confirm=(id,at,hs,full,code=200,status='succeeded')=>insert.run(id,'payment_confirm',at,code,status,full,hs);
 confirm('startup',5000,9000,10000);
 confirm('normal-1',15000,10,100);confirm('normal-2',16000,20,200);
 confirm('spike',18000,3000,3100);
 insert.run('cross-state','payment_create',32000,200,'requires_payment_method',20000,null);
 confirm('cross-state',45000,8000,100);
 confirm('outage',25000,9000,10000);
 confirm('settling',35000,9000,10000);
 confirm('recovered',46000,30,300);
 confirm('missing-header',47000,null,400);
 confirm('failed',48000,90000,90000,500,null);
 confirm('declined',49000,90000,90000,200,'failed');
 db.close();
 const result=connectorResults(directory,plan,{latencyWindows:[{from:10000,to:20000},{from:40000,to:60000}]}),stripe=result.rows[0];
 assert.equal(stripe.confirmations,11);assert.equal(stripe.successes,9);assert.equal(stripe.failures,2);
 assert.equal(result.latencyScope,'settled_steady_load');
 assert.deepEqual({...stripe.latency.hs,p99:null},{count:4,min:10,max:3000,mean:765,median:25,p99:null});assert.ok(Math.abs(stripe.latency.hs.p99-2910.9)<1e-7);
 assert.deepEqual(stripe.latency.full,{count:5,min:100,max:3100,mean:820,median:300,p99:2992});
 assert.equal(stripe.hsP99,stripe.latency.hs.p99);assert.equal(stripe.fullP99,stripe.latency.full.p99);
 const unavailable=connectorResults(directory,plan);
 assert.equal(unavailable.latencyScope,null);assert.equal(unavailable.rows[0].hsP99,null);
 assert.equal(unavailable.rows[0].latency.full.count,0,'missing state evidence never falls back to all-period latency');
 assert.equal(connectorResults(directory,plan,{latencyWindows:[]}).rows[0].latency.full.count,0,'a run with no steady period has no eligible samples');
});
