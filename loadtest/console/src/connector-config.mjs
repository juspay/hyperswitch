import catalog from './connector-catalog.json' with {type:'json'};
export {catalog as connectorCatalog};
export const supportedConnectorAuth=new Set(['HeaderKey','BodyKey','SignatureKey','MultiAuthKey','NoKey']);
export const defaultConnectorPlan=()=>({mode:'dummy',mix:[{connector:'stripe_test',weight:100}]});
export function normalizeConnectorPlan(value){
 if(value==null)return defaultConnectorPlan();
 if(!['dummy','real'].includes(value.mode))throw new Error('Choose Dummy or Real connectors');
 if(!Array.isArray(value.mix)||!value.mix.length||value.mix.length>20)throw new Error('Choose between 1 and 20 connectors');
 const seen=new Set();const mix=value.mix.map(row=>{
  const definition=catalog.find(c=>c.id===row.connector),weight=Number(row.weight);
  if(!definition||definition.dummy!==(value.mode==='dummy')||!definition.card||!supportedConnectorAuth.has(definition.authType))throw new Error('Choose a configured card connector for this mode');
  if(seen.has(row.connector))throw new Error('Each connector can only appear once');seen.add(row.connector);
  if(!Number.isInteger(weight)||weight<1||weight>100)throw new Error('Connector weights must be whole percentages between 1 and 100');
  return {connector:row.connector,weight};
 });
 if(mix.reduce((s,r)=>s+r.weight,0)!==100)throw new Error('Connector weights must total 100%');
 return {mode:value.mode,mix};
}
export const connectorSet=plan=>normalizeConnectorPlan(plan).mix.map(r=>r.connector).sort();
export function connectorPlanError(value){try{normalizeConnectorPlan(value);return '';}catch(e){return e.message;}}
