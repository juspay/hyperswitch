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
 const result=connectorResults(directory,plan);assert.equal(result.unattributedConfirmations,1);const stripe=result.rows.find(r=>r.connector==='stripe');assert.equal(stripe.assigned,2);assert.equal(stripe.confirmations,1);assert.equal(stripe.observedPercent,50);assert.equal(stripe.hsP99,30);assert.equal(stripe.fullP99,100);assert.equal(result.rows.find(r=>r.connector==='adyen').successes,1);
});
