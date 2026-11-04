import {readFileSync} from 'node:fs';
import path from 'node:path';
import {dataRoot} from './state.mjs';
import {readImportedCertificate} from './certificates.mjs';

/** Build a pg connection config without forwarding UI-only fields. */
export function databaseConnectionOptions(database,{readCa=readFileSync,readCertificate=id=>readImportedCertificate(dataRoot,id)}={}){
  const connection={host:database.host,port:Number(database.port),database:database.database,user:database.user,password:database.password};
  const tlsMode=database.tlsMode || (database.ssl?'verify':'disable');
  if(tlsMode==='disable')return {...connection,ssl:false};
  if(tlsMode==='require')return {...connection,ssl:{rejectUnauthorized:false}};
  if(tlsMode!=='verify')throw new Error('Database TLS mode must be disable, require, or verify');
  if(database.caCertificateId){
    let ca;try{ca=readCertificate(database.caCertificateId);}catch(e){throw new Error(`Cannot read imported database CA certificate: ${e.message}`);}
    if(!String(ca).includes('-----BEGIN CERTIFICATE-----'))throw new Error('Imported database CA certificate is not PEM encoded');
    return {...connection,ssl:{rejectUnauthorized:true,ca}};
  }
  const caPath=String(database.caPath || '').trim();
  if(!caPath)return {...connection,ssl:{rejectUnauthorized:true}};
  if(!path.isAbsolute(caPath))throw new Error('Database CA certificate path must be absolute');
  let ca;
  try{ca=readCa(caPath,'utf8');}catch(e){throw new Error(`Cannot read database CA certificate: ${e.message}`);}
  if(!String(ca).includes('-----BEGIN CERTIFICATE-----'))throw new Error('Database CA certificate is not PEM encoded');
  return {...connection,ssl:{rejectUnauthorized:true,ca}};
}
