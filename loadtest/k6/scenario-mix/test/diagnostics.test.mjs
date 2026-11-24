import test from 'node:test';
import assert from 'node:assert/strict';
import {responseRecord} from '../record.mjs';
test('error projections keep useful diagnostics without echoed secrets or card data',()=>{
 const body=JSON.stringify({error:{code:'PAYMENT_FAILED',message:'Invalid card 4242424242424242; api_key=private-key',metadata:{password:'private-password'}},card_number:'4242424242424242',client_secret:'private-client-secret'});
 const record=responseRecord({operation:'payment_confirm'},'POST','http://router/payments/fake/confirm',{}, {status:400},body,1);
 assert.match(record.error_body,/PAYMENT_FAILED/);
 assert.doesNotMatch(JSON.stringify(record),/4242424242424242|private-key|private-password|private-client-secret/);
 const html=responseRecord({operation:'payment_confirm'},'POST','http://router/confirm',{}, {status:500},'<html>raw-secret</html>',1);
 assert.doesNotMatch(html.error_body,/raw-secret/);
});
