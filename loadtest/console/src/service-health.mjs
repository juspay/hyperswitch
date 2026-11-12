export const isDatabaseTarget=target=>target.serviceType==='postgres'||target.adapter==='gcp-alloydb'||/^(postgres(?:ql)?|db)$/i.test(String(target.label||'').trim());

export function faultHealthOverride(service,fault,targets=[]){
 const active=fault?.active;
 if(!active||active.simulated)return null;
 const target=targets.find(target=>target.id===active.targetId);
 const matches=service.id==='database'?(target&&isDatabaseTarget(target)):service.targetId&&service.targetId===active.targetId;
 if(!matches)return null;
 if(service.id==='database'&&['available','unavailable'].includes(active.databaseAvailability))return {state:active.databaseAvailability==='available'?'online':'offline',message:`DB · ${active.databaseAvailability} · SQL connection probe`};
 // A failed rollout verification is not evidence that a service is still down.
 // Let the regular readiness probe measure non-DB services again.
 if(active.status==='recovery_failed'&&service.id!=='database')return null;
 if(active.status==='recovery_failed')return {state:'offline',message:`${active.target} · recovery not verified`};
 if(active.status==='restoring')return {state:'recovering',message:`${active.target} · verifying recovery`};
 if(active.action==='failover')return {state:'recovering',message:`${active.target} · failover in progress`};
 if(active.occurred)return {state:'offline',message:`${active.target} · interruption verified`};
 return {state:'recovering',message:`${active.target} · verifying interruption`};
}

export function pendingHealth(service,previous,override){
 return {...service,state:previous?.state||'checking',message:previous?.message||'Checking…',...override};
}
