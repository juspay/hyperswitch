export const flowCatalog=[['cit_off_session','CIT off-session'],['sdk_checkout','SDK checkout'],['guest','Guest payment'],['cit_on_session','CIT on-session'],['ptv_on_session','Pay to vault · on-session'],['ptv_off_session','Pay to vault · off-session'],['cit_metadata_changed','Card metadata update'],['mit','Merchant-initiated payment'],['saved_card_checkout','Saved-card checkout']];
export const flowLabel=type=>flowCatalog.find(([key])=>key===type)?.[1] || type;
export function newFlow(type,flows){const names=new Set(flows.map(flow=>flow.name));let name=type,index=2;while(names.has(name))name=`${type}_${index++}`;return {name,scenario:type,merchant_path:'non_modular',weight:0};}
export function profileSummary(profile){return `${profile.mode==='tenancy'?'Tenancy':'Single'} · ${profile.merchantCount} ${profile.merchantCount===1?'merchant':'merchants'} · ${profile.totalRps} ${profile.loadShape==='staged'?'peak TPS':'TPS'} · ${profile.durationSeconds}s`;}

export function balanceWeights(flows,normalize=false){
 if(!flows.length)return flows;
 const total=flows.reduce((sum,flow)=>sum+Number(flow.weight||0),0),remaining=Math.max(0,100-total);
 const empty=flows.filter(flow=>!flow.weight).length;
 const values=flows.map(flow=>normalize?(total?Number(flow.weight||0)/total*100:100/flows.length):Number(flow.weight||0)+(!empty||!flow.weight?remaining/(empty||flows.length):0));
 const rounded=values.map(Math.floor);let extra=Math.round(values.reduce((a,b)=>a+b,0))-rounded.reduce((a,b)=>a+b,0);
 const order=values.map((v,i)=>({i,remainder:v-rounded[i]})).sort((a,b)=>b.remainder-a.remainder);
 for(let n=0;n<extra;n++)rounded[order[n%order.length].i]++;
 return flows.map((flow,i)=>({...flow,weight:rounded[i]}));
}
