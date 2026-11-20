import test from 'node:test';
import assert from 'node:assert/strict';
import {databaseConnectionOptions} from '../server/database-options.mjs';
import {databaseTestConnection} from '../server/connection-secrets.mjs';
test('verified SSH database tunnels retain the remote certificate identity',()=>{
 const db={host:'127.0.0.1',port:15433,tlsMode:'verify',serverName:'database.rds.amazonaws.com',caPath:'/ca.pem'};
 const options=databaseConnectionOptions(db,{readCa:()=> '-----BEGIN CERTIFICATE-----\nexample'});
 assert.equal(options.host,'127.0.0.1');assert.equal(options.ssl.servername,db.serverName);assert.equal(options.ssl.rejectUnauthorized,true);
 assert.throws(()=>databaseConnectionOptions({...db,serverName:'bad/name'}),/DNS hostname/);
 assert.throws(()=>databaseTestConnection({...db,password:'secret'},{serverName:'different.rds.amazonaws.com',password:''}),/new database password/);
});

test('AWS PostgreSQL command faults independently observe SQL availability',async()=>{
 const before=process.env.LOADTEST_ALLOW_CUSTOM_COMMANDS;process.env.LOADTEST_ALLOW_CUSTOM_COMMANDS='true';
 try{
  const {makeFaultAdapter}=await import('../server/faults.mjs');let timeout;
  const target={id:'rds',environment:'aws',serviceType:'postgres',adapter:'command',recoveryMode:'self-recovering',commandAction:'failover',timeoutSeconds:120,workingDirectory:'/tmp',triggerCommand:'true',readyCommand:'true',faultCheckCommand:'true'};
  const adapter=makeFaultAdapter(target,undefined,{databaseCheck:async value=>{timeout=value;return {ok:true};}});
  assert.deepEqual(await adapter.checkAvailability(),{ok:true});assert.equal(timeout,2000);
  assert.equal(makeFaultAdapter({...target,serviceType:'redis'}).checkAvailability,undefined);
 }finally{if(before===undefined)delete process.env.LOADTEST_ALLOW_CUSTOM_COMMANDS;else process.env.LOADTEST_ALLOW_CUSTOM_COMMANDS=before;}
});
