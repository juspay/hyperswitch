import test from 'node:test';
import assert from 'node:assert/strict';
import {faultNotice} from '../src/fault-notice.mjs';

test('DB availability updates the notice independently of cloud operation completion',()=>{
 const active={target:'PostgreSQL',status:'restoring',databaseAvailability:'unavailable'};
 assert.match(faultNotice({active},{live:true}),/next stage waits for recovery/);
 const recovered=faultNotice({active:{...active,databaseAvailability:'available'}},{live:true});
 assert.equal(recovered,'PostgreSQL · DB available · waiting for failover operation to complete');
 assert.doesNotMatch(recovered,/restoring|waits for recovery/);
 assert.equal(faultNotice({active:null,blocked:false},{live:true}),'');
});
test('failed verification preserves DB availability and non-DB recovery messages',()=>{
 assert.equal(faultNotice({active:{target:'PostgreSQL',status:'recovery_failed',databaseAvailability:'available'},blocked:true}),
  'PostgreSQL · DB available · failover completion not verified');
 assert.equal(faultNotice({active:{target:'Card vault',status:'restoring'}},{live:true}),
  'Card vault · restoring · next stage waits for recovery');
 assert.doesNotMatch(faultNotice({active:{target:'Card vault',status:'restoring'}},{live:true,stopping:true}),/next stage/);
});
