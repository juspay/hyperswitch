import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,readFile,writeFile,mkdir } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import pg from 'pg';

// Opt in against an existing HS database; this test only reads PostgreSQL.
test('PostgreSQL reconciliation batches, persists outcomes and rejects incomplete recording',
  {skip:process.env.CONSOLE_TEST_PG!=='1'},async()=>{
    const temporary=await mkdtemp(path.join(os.tmpdir(),'loadtest-console-pg-'));
    process.env.LOADTEST_DATA_DIR=temporary;
    process.env.LOADTEST_CONSOLE_ROOT=path.resolve('.');process.env.LOADTEST_ROOT=path.resolve('..');
    const config=JSON.parse(await readFile('../loadtest.config.example.json','utf8'));config.output=path.join(temporary,'artifacts');
    process.env.LOADTEST_CONFIG_FILE=path.join(temporary,'seed.json');await writeFile(process.env.LOADTEST_CONFIG_FILE,JSON.stringify(config));
    const state=await import('../server/state.mjs');state.initializeConfig();
    const {databaseConnectionOptions}=await import('../server/database-options.mjs');
    const {queryWorker}=await import('../server/workers.mjs');
    const database={host:process.env.PGHOST,port:Number(process.env.PGPORT || 5432),database:process.env.PGDATABASE,
      user:process.env.PGUSER,password:process.env.PGPASSWORD,ssl:['require','verify-ca','verify-full'].includes(process.env.PGSSLMODE),caPath:process.env.PGSSLROOTCERT || ''};
    const pool=new pg.Pool({...databaseConnectionOptions(database),max:1});
    let rows;
    try {
      const client=await pool.connect();
      try {await client.query('BEGIN READ ONLY');rows=(await client.query('SELECT merchant_id,payment_id,status::text AS status FROM payment_intent WHERE status IS NOT NULL LIMIT 2')).rows;await client.query('COMMIT');}
      finally{client.release();}
    }finally{await pool.end();}
    assert.equal(rows.length,2,'Provide a database with at least two payments for this integration test');
    const directory=path.join(config.output,'single/runs/recon-test');await mkdir(directory,{recursive:true});
    await writeFile(path.join(directory,'config.json'),JSON.stringify(config));
    const db=new DatabaseSync(path.join(directory,'results.sqlite'));
    db.exec(await readFile('../k6/scenario-mix/recorder/schema.sql','utf8'));
    db.exec("INSERT INTO runs(run_id,started_at,ended_at,recording_state,enqueued,persisted) VALUES('recon',1,2,'complete',1004,1004)");
    const insert=db.prepare("INSERT INTO payments(run_id,merchant_id,payment_id,status,role,api_status_code,observed_at,response_event_id) VALUES('recon',?,?,?,'measured',200,1,1)");
    insert.run(rows[0].merchant_id,rows[0].payment_id,rows[0].status);
    insert.run(rows[1].merchant_id,rows[1].payment_id,'test-client-mismatch');
    for(let n=0;n<1002;n++)insert.run('console-test-absent',`zz-console-test-${n}`,'succeeded');
    const failure=db.prepare("INSERT INTO request_events(run_id,attempt_id,flow_id,merchant_id,payment_id,operation,role,observed_at,method,url,api_status_code,latency_ms,message) VALUES('recon',?,'flow',?,?,?,'measured',1,'POST','http://test',0,30000,'request timeout')");
    failure.run(1,rows[0].merchant_id,rows[0].payment_id,'payment_confirm');failure.run(2,rows[1].merchant_id,rows[1].payment_id,'payment_confirm');
    failure.run(3,'absent-merchant','absent-payment','payment_confirm');failure.run(4,'absent-merchant',null,'payment_create');failure.run(5,'absent-merchant',null,'customer_create');
    const progress=[];
    const result=await queryWorker('reconcile',{id:'single:recon-test',database},p=>progress.push(p.compared));
    const batches=[0,...Array.from({length:10},(_,i)=>(i+1)*100),1004];assert.deepEqual(progress,batches);assert.equal(result.expected,1004);assert.deepEqual(result.timeline.map(p=>p.compared),batches);assert.equal(result.timeline.at(-1).match,1);assert.equal(result.timeline.at(-1).missing,1002);assert.ok(result.timeline.every((p,i,all)=>!i||p.t>=all[i-1].t));assert.equal(result.match,1);assert.equal(result.mismatch,1);assert.equal(result.missing,1002);assert.equal(result.creationUnknown,1);assert.equal(result.confirmationFailed,2);assert.equal(result.accounted,1005);
    assert.deepEqual({attempts:result.transport.attempts,checked:result.transport.checkedPayments,found:result.transport.found,missing:result.transport.missing,unresolved:result.transport.unresolved},{attempts:5,checked:3,found:2,missing:1,unresolved:2});
    assert.match(result.discrepancies.find(r=>r.paymentId===rows[1].payment_id).reason,/Transport failure/);
    assert.ok(result.comparedAt);assert.ok(result.truncated);assert.equal(result.discrepancies.length,200);
    const saved=JSON.parse(await readFile(path.join(directory,'reconciliation.json'),'utf8'));assert.equal(saved.total,1004);
    const output=new DatabaseSync(path.join(directory,'reconciliation.sqlite'),{readOnly:true});
    assert.equal(output.prepare('SELECT COUNT(*) n FROM comparisons').get().n,1004);assert.equal(output.prepare('SELECT COUNT(*) n FROM transport_comparisons').get().n,3);output.close();
    db.exec("UPDATE runs SET recording_state='incomplete',enqueued=1005");
    await assert.rejects(queryWorker('reconcile',{id:'single:recon-test',database}),/incomplete/);
    assert.equal((await queryWorker('reconcile',{id:'single:recon-test',database,allowIncomplete:true})).total,1004);
    db.exec("DELETE FROM payments; UPDATE runs SET recording_state='complete',enqueued=persisted");
    const onlyTransport=await queryWorker('reconcile',{id:'single:recon-test',database});assert.equal(onlyTransport.total,0);assert.equal(onlyTransport.transport.checkedPayments,3);assert.equal(onlyTransport.transport.unresolved,2);
    db.close();
  });
