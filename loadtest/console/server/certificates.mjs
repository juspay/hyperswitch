import {X509Certificate,randomUUID} from 'node:crypto';
import {mkdir,readFile,writeFile,rename,rm} from 'node:fs/promises';
import {readFileSync} from 'node:fs';
import path from 'node:path';

const idPattern=/^[0-9a-f-]{36}$/;
const pemPattern=/-----BEGIN CERTIFICATE-----[\s\S]+?-----END CERTIFICATE-----/g;
const directory=root=>path.join(root,'certificates');
const manifestPath=root=>path.join(directory(root),'manifest.json');
export const certificatePath=(root,id)=>{
  if(!idPattern.test(String(id)))throw new Error('Invalid certificate ID');
  return path.join(directory(root),`${id}.pem`);
};
function metadata(root){try{return JSON.parse(readFileSync(manifestPath(root),'utf8'));}catch(e){if(e.code==='ENOENT')return [];throw e;}}
async function saveMetadata(root,value){
  await mkdir(directory(root),{recursive:true,mode:0o700});
  const temporary=`${manifestPath(root)}.${process.pid}.tmp`;
  await writeFile(temporary,JSON.stringify(value,null,2)+'\n',{mode:0o600});
  await rename(temporary,manifestPath(root));
}
export function listCertificates(root){return metadata(root).map(({id,name,fingerprint,subject,issuer,validTo,certificateCount,createdAt})=>({id,name,fingerprint,subject,issuer,validTo,certificateCount,createdAt}));}
export function hasCertificate(root,id){return Boolean(id&&metadata(root).some(item=>item.id===id));}
export function readImportedCertificate(root,id){return readFileSync(certificatePath(root,id),'utf8');}
export async function importCertificate(root,{name,pem}){
  const text=String(pem||'').trim(),blocks=text.match(pemPattern)||[];
  if(!blocks.length)throw new Error('Choose a PEM encoded CA certificate');
  if(Buffer.byteLength(text)>1024*1024||blocks.length>20)throw new Error('Certificate bundle exceeds the 1 MiB or 20 certificate limit');
  let certificates;try{certificates=blocks.map(block=>new X509Certificate(block));}catch{throw new Error('Certificate bundle contains an invalid X.509 certificate');}
  const id=randomUUID(),first=certificates[0],entry={id,name:String(name||'Imported CA').trim().slice(0,100)||'Imported CA',fingerprint:first.fingerprint256,
    subject:first.subject,issuer:first.issuer,validTo:first.validTo,certificateCount:certificates.length,createdAt:Date.now()};
  await mkdir(directory(root),{recursive:true,mode:0o700});
  await writeFile(certificatePath(root,id),blocks.join('\n')+'\n',{mode:0o600,flag:'wx'});
  await saveMetadata(root,[...metadata(root),entry]);
  return entry;
}
export async function removeCertificate(root,id){
  if(!idPattern.test(String(id)))throw new Error('Invalid certificate ID');
  await saveMetadata(root,metadata(root).filter(item=>item.id!==id));
  await rm(certificatePath(root,id),{force:true});
}
