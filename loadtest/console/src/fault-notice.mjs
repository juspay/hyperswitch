import {faultDisplayName} from './fault-types.mjs';
import {isDatabaseTarget} from './service-health.mjs';

export function faultNotice(faults,{stopping=false,activity=[],databaseHealth=null}={}) {
 const active=faults?.active;
 if(!active&&!faults?.blocked)return null;
 const title=faultDisplayName(active?.target||'Fault recovery');
 const failed=active?.status==='recovery_failed'||faults?.blocked;
 const failover=active?.action==='failover';
 const target=faults.targets?.find(t=>t.id===active?.targetId);
 const databaseTarget=isDatabaseTarget(target?.target||target||{label:title})||active?.databaseAvailability!=null;
 const measured=databaseTarget&&databaseHealth&&['online','offline'].includes(databaseHealth.state)&&Date.now()-databaseHealth.checkedAt>=0&&Date.now()-databaseHealth.checkedAt<60000;
 // A failed operation can leave an old availability sample behind. Only a
 // fresh SQL probe may describe DB health while verification is blocked.
 const available=!databaseTarget?null:measured?(databaseHealth.state==='online'?'available':'unavailable'):failed?null:active?.databaseAvailability??[...activity].reverse().find(e=>['database_available','database_unavailable'].includes(e.kind)&&e.at>=(active?.recoveryAt||Infinity)-10000)?.kind?.replace('database_','');
 const availability=available==='available'?{label:'Available',tone:'positive'}:available==='unavailable'?{label:'Unavailable',tone:'negative'}:null;
 let description;
 if(failed){description=failover?'Failover confirmation pending':'Recovery verification pending';
 }else if(stopping){description='Verifying recovery before stopping';
 }else if(active?.status==='restoring'){description=available==='unavailable'?'Waiting for database recovery':failover?'Confirming failover completion':'Confirming service recovery';
 }else if(failover){description='Failover in progress';
 }else{description=active?.occurred?'Waiting for scheduled recovery':'Checking fault target';}
 return {title,description,availability,tone:failed?'warning':'info',canRetry:Boolean(faults?.blocked),retryLabel:faults?.recovering?(failover?'Checking…':'Recovering…'):(failover?'Recheck':'Retry recovery')};
}
