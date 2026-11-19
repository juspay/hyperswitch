// A blank secret means "reuse at the same destination", never "forward elsewhere".
export function requireBoundSecret(savedSecret,newSecret,sameDestination,label){
 if(savedSecret&&!newSecret&&!sameDestination)throw new Error(`Enter a new ${label} when changing the connection destination or weakening TLS`);
 return newSecret||savedSecret;
}
export function databaseTestConnection(saved,draft){
 const next={...saved,...draft};
 const identity=db=>JSON.stringify([String(db.host||'').toLowerCase(),Number(db.port||5432),db.database||'',db.user||'']);
 const strength=db=>({disable:0,require:1,verify:2})[db.tlsMode||(db.ssl?'verify':'disable')];
 next.password=requireBoundSecret(saved.password,draft.password,identity(saved)===identity(next)&&strength(next)>=strength(saved),'database password');
 return next;
}
export const sameEndpoint=(left,right)=>new URL(left).href.replace(/\/$/,'')===new URL(right).href.replace(/\/$/,'');
