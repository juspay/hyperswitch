import test from 'node:test';
import assert from 'node:assert/strict';
import {databaseRecoveryGate} from '../server/database-recovery-gate.mjs';

test('SQL recovery releases the load once while the cloud operation is still pending',async()=>{
 const entry={status:'active',loadReleased:false};let releases=0;
 const sample=databaseRecoveryGate(entry,async()=>{releases++;return true;});
 await sample('available');assert.equal(releases,0);
 await sample('unavailable');await sample('available');assert.equal(releases,0);
 entry.status='restoring';await Promise.all([sample('available'),sample('available')]);
 assert.equal(releases,1);assert.equal(entry.loadReleased,true);
 await sample('available');assert.equal(releases,1);
});
test('failed load-clock release retries instead of falsely declaring it resumed',async()=>{
 const entry={status:'restoring',databaseOutageObserved:true,loadReleased:false};let calls=0;
 const sample=databaseRecoveryGate(entry,async()=>++calls>1);
 await sample('available');assert.equal(entry.loadReleased,false);
 await sample('available');assert.equal(entry.loadReleased,true);assert.equal(calls,2);
});
