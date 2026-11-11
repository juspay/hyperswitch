const sensitive=/(api[_ -]?key|client[_ -]?secret|authorization|password|access[_ -]?token|card[_ -]?(?:number|cvc)|cvc|cvv)(["']?\s*[=:]\s*)("[^"]*"|'[^']*'|[^\s;,]+)/gi;
export function diagnosticText(value){
 return String(value).replace(/\bBearer\s+[A-Za-z0-9._~+/=-]+/gi,'Bearer [redacted]')
  .replace(sensitive,'$1$2[redacted]').replace(/\b(?:\d[ -]?){13,19}\b/g,'[redacted]').slice(0,512);
}
export function diagnosticBody(text){
 if(text.length>65536)return '[oversized error response omitted]';
 let value;try{value=JSON.parse(text);}catch{return '[non-JSON error response omitted]';}
 const project=(value,depth=0)=>{
  if(typeof value==='string')return diagnosticText(value);
  if(!value||typeof value!=='object'||Array.isArray(value)||depth>3)return null;
  return Object.fromEntries(['error','code','type','message'].filter(key=>Object.hasOwn(value,key)).map(key=>[key,project(value[key],depth+1)]));
 };
 const result=JSON.stringify(project(value));return result.length<=4096?result:'{"message":"Error diagnostic exceeded limit"}';
}
export function diagnosticUrl(value){
 try{const url=new URL(value);url.username='';url.password='';for(const key of [...url.searchParams.keys()])if(/key|secret|token|password|cvc|cvv|card/i.test(key))url.searchParams.set(key,'[redacted]');return url.href;}catch{return diagnosticText(value);}
}
