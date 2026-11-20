import {randomBytes,randomUUID,createCipheriv,createDecipheriv} from 'node:crypto';
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import path from 'node:path';
import {normalizeConnectorPlan,connectorCatalog} from '../src/connector-config.mjs';
// Account credentials are encrypted separately from scenario, job and run snapshots.
export function connectorSecretStore(root){
 const directory=path.join(root,'connector-secrets');mkdirSync(directory,{recursive:true,mode:0o700});
 const keyfile=path.join(directory,'key');let key;
 try{key=readFileSync(keyfile);}catch(e){if(e.code!=='ENOENT')throw e;try{writeFileSync(keyfile,randomBytes(32),{flag:'wx',mode:0o600});}catch(e){if(e.code!=='EEXIST')throw e;}key=readFileSync(keyfile);}
 return {put(value){const ref=randomUUID(),iv=randomBytes(12),cipher=createCipheriv('aes-256-gcm',key,iv);const data=Buffer.concat([cipher.update(JSON.stringify(value),'utf8'),cipher.final()]);writeFileSync(path.join(directory,ref),JSON.stringify({iv:iv.toString('base64'),tag:cipher.getAuthTag().toString('base64'),data:data.toString('base64')}),{mode:0o600,flag:'wx'});return ref;},get(ref){if(!/^[a-f0-9-]{36}$/.test(ref))throw new Error('Invalid connector credential reference');const sealed=JSON.parse(readFileSync(path.join(directory,ref),'utf8')),cipher=createDecipheriv('aes-256-gcm',key,Buffer.from(sealed.iv,'base64'));cipher.setAuthTag(Buffer.from(sealed.tag,'base64'));return JSON.parse(Buffer.concat([cipher.update(Buffer.from(sealed.data,'base64')),cipher.final()]).toString());}};
}
export function provisionConnectorAccounts(plan,input={},root){
 const normalized=normalizeConnectorPlan(plan),store=connectorSecretStore(root);
 return normalized.mix.map(({connector})=>{
  const definition=connectorCatalog.find(c=>c.id===connector),draft=input[connector]||{};
  const details={auth_type:definition.authType};
  for(const field of definition.fields){const value=draft.credentials?.[field.name]||(normalized.mode==='dummy'?'local-test':'');if(typeof value!=='string'||!value.trim()||value.length>16384)throw new Error(`Enter ${definition.label}: ${field.label}`);details[field.name]=value;}
  if(!['HeaderKey','BodyKey','SignatureKey','MultiAuthKey','NoKey'].includes(details.auth_type))throw new Error(`Credential form for ${definition.label} is not supported yet`);
  if(draft.metadata!=null&&(typeof draft.metadata!=='object'||Array.isArray(draft.metadata)))throw new Error('Connector metadata must be a JSON object');
  const account={connector_name:connector,connector_account_details:details,connector_type:'payment_processor',test_mode:true,business_country:'US',business_label:'default',disabled:false,payment_methods_enabled:[{payment_method:'card',payment_method_types:[{payment_method_type:'credit',card_networks:definition.networks?.length?definition.networks:['Visa','Mastercard'],recurring_enabled:true}]}],metadata:draft.metadata||{}};
  return {connector,credentialRef:store.put(account)};
 });
}
export const resolveConnectorAccounts=(refs,root)=>refs?.map(row=>{
 const account=connectorSecretStore(root).get(row.credentialRef);
 // Earlier console accounts used a label absent from the fixture merchant.
 // Repair those encrypted snapshots too, so interrupted fixtures can resume.
 return account.business_label==='loadtest'?{...account,business_label:'default'}:account;
});
