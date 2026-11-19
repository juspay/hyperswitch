export const unavailableFlows=new Set(['cit_metadata_changed','mit','saved_card_checkout']);
export function normalizePaymentFlows(flows=[]){
 const used=new Set();return flows.map(flow=>{
  let name=flow.name?.replace(/^(?:nomod|mod)_/,'')||flow.scenario,base=name,index=2;
  while(used.has(name))name=`${base}_${index++}`;used.add(name);
  return {...flow,name,merchant_path:'modular'};
 });
}
export function assertSupportedPaymentFlows(flows){
 for(const flow of flows)if(Number(flow.weight)>0&&unavailableFlows.has(flow.scenario))throw new Error(`${flow.scenario.replaceAll('_',' ')} is not yet supported by the payment-method service; remove it from the traffic mix`);
}
