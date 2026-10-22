import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { FixtureRecorder, recordedFetch } from '../sqlite.mjs';
import { responseRecord } from '../record.mjs';
import { reconcile, csvRows } from '../reconcile.mjs';

const temp = () => mkdtemp(path.join(os.tmpdir(),'loadtest-storage-'));
function record(operation,body,request={},context={},status=200,text=null) {
 return responseRecord({ flow_id:'flow-1',merchant_id:'merchant-1',operation,...context },'POST','http://local/payments',request,
  { status,headers:{'X-Request-Id':'request-123'} },text ?? JSON.stringify(body),12.5);
}
test('records fields, errors, malformed bodies and transport failures without retaining successful raw bodies',() => {
 const customer = record('customer_create',{id:'cus-1'},{merchant_reference_id:'ref-1'});
 assert.equal(customer.customer_id,'cus-1'); assert.equal(customer.request_id,'request-123');
 assert.equal(customer.merchant_reference_id,'ref-1'); assert.equal(customer.error_body,null);
 const failure = record('payment_create',{error:{message:'invalid amount'}},{amount:3},{},422);
 assert.equal(failure.payment_id,null); assert.equal(failure.message,'invalid amount');
 assert.match(failure.error_body,/invalid amount/);
 assert.equal(record('payment_confirm',{}, {}, {}, 200,'garbage').message,'invalid_json_response');
 const transport = responseRecord({flow_id:'f',operation:'payment_create'},'POST','http://local/payments',{},null,'',5,'timeout');
 assert.equal(transport.api_status_code,0); assert.equal(transport.message,'timeout');
});
test('SQLite preserves event history and latest known payment status, including baseline role',async() => {
 const dir = await temp(), file = path.join(dir,'results.sqlite');
 const recorder = new FixtureRecorder(file);
 recorder.record(record('baseline_create',{payment_id:'baseline',status:'requires_payment_method',amount:100}));
 recorder.record(record('payment_create',{payment_id:'pay-1',status:'requires_payment_method',amount:100,customer_id:'cus-1'}));
 recorder.record(record('payment_confirm',{payment_id:'pay-1',status:'succeeded'}));
 recorder.record(record('payment_confirm',{error:{message:'unavailable'}},{},{payment_id:'pay-1'},500));
 // A confirm URL supplies payment ID even when its error body has none.
 recorder.record(responseRecord({flow_id:'f',operation:'payment_confirm'},'POST','http://local/payments/pay-1/confirm',{},
  {status:500,headers:{}},'{"error":{"message":"unavailable"}}',2));
 const stale=record('payment_confirm',{payment_id:'pay-1',status:'processing'});stale.observed_at=1;recorder.record(stale);
 recorder.close();
 const db = new DatabaseSync(file,{readOnly:true});
 const payment = db.prepare("SELECT * FROM payments WHERE payment_id='pay-1'").get();
 assert.equal(payment.status,'succeeded'); assert.equal(payment.amount,100); assert.equal(payment.customer_id,'cus-1');
 assert.equal(payment.api_status_code,500); assert.equal(payment.message,'unavailable');
 assert.equal(db.prepare("SELECT role FROM payments WHERE payment_id='baseline'").get().role,'baseline');
 assert.equal(db.prepare('SELECT COUNT(*) n FROM request_events').get().n,6);
 assert.equal(db.prepare('SELECT recording_state FROM runs').get().recording_state,'complete');
 db.close();
});
test('reconciliation handles matches, missing IDs, mismatches and duplicate/conflicting input',async() => {
 const dir = await temp(), file = path.join(dir,'results.sqlite'), csv = path.join(dir,'input.csv'), output = path.join(dir,'result.csv');
 const recorder = new FixtureRecorder(file);
 recorder.record(record('payment_confirm',{payment_id:'pay-1',status:'succeeded'})); recorder.close();
 await writeFile(csv,'payment_id,status\r\npay-1,succeeded\r\nmissing,failed\r\npay-2,failed\r\npay-2,succeeded\r\npay-3,failed\r\npay-3,failed\r\n');
 const result = await reconcile({db:file,csv,output});
 assert.equal(result.exitCode,2); assert.deepEqual(result.summary,{match:1,mismatch:0,missing:1,duplicate:2,conflicting_duplicate:2});
 assert.match(await readFile(output,'utf8'),/conflicting_duplicate/);
 await writeFile(csv,'payment_id,status\npay-1,failed\n');
 assert.equal((await reconcile({db:file,csv,output:path.join(dir,'mismatch.csv')})).summary.mismatch,1);
});
test('incomplete databases require an explicit override; invalid CSV fails',async() => {
 const dir = await temp(), file = path.join(dir,'results.sqlite'),csv = path.join(dir,'input.csv');
 const recorder = new FixtureRecorder(file); recorder.close(new Error('interrupted'));
 await writeFile(csv,'payment_id,status\nmissing,failed\n');
 await assert.rejects(reconcile({db:file,csv,output:path.join(dir,'out.csv')}),/incomplete/);
 assert.equal((await reconcile({db:file,csv,output:path.join(dir,'partial.csv'),allowIncomplete:true})).summary.missing,1);
 await writeFile(csv,'status,payment_id\nfailed,missing\n');
 await assert.rejects(reconcile({db:file,csv,output:path.join(dir,'bad.csv'),allowIncomplete:true}),/header/);
});
test('CSV parser handles quotes, embedded newlines, UTF-8 and chunk boundaries',async() => {
 const dir = await temp(),csv = path.join(dir,'input.csv');
 const long = 'é'.repeat(40000);
 await writeFile(csv,`\uFEFFpayment_id,status\r\n"${long}","a,b\nwith ""quote"""\r\n`);
 const rows = []; for await (const row of csvRows(csv)) rows.push(row);
 assert.deepEqual(rows,[['payment_id','status'],[long,'a,b\nwith "quote"']]);
 await writeFile(csv,'payment_id,status\n"unterminated');
 await assert.rejects(async()=> { for await(const _ of csvRows(csv)) {} },/Unterminated/);
});
test('transport attempts are captured before fetch errors propagate',async() => {
 const dir = await temp(),file = path.join(dir,'fixtures.sqlite');
 const recorder = new FixtureRecorder(file);
 await assert.rejects(recordedFetch(recorder,'http://127.0.0.1:1',{},{operation:'fixture_GET'}));
 recorder.close();
 const db = new DatabaseSync(file,{readOnly:true});
 assert.equal(db.prepare('SELECT COUNT(*) n FROM request_events WHERE api_status_code=0').get().n,1); db.close();
});
