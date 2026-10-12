export function faultNotice(faults,{live=false,stopping=false,activity=[]}={}) {
 const active=faults?.active;
 if(!active&&!faults?.blocked)return '';
 const label=active?.target||'Fault recovery';
 const databaseAvailability=active?.databaseAvailability??[...activity].reverse().find(e=>['database_available','database_unavailable'].includes(e.kind)&&e.at>=(active?.recoveryAt||Infinity)-10000)?.kind?.replace('database_','');
 if(databaseAvailability==='available'&&['restoring','recovery_failed'].includes(active.status)) {
  return `${label} · DB available · ${active.status==='recovery_failed'?'failover completion not verified':'waiting for failover operation to complete'}`;
 }
 const status=active?.status==='recovery_failed'?'recovery not verified':active?.status||'pending';
 return `${label} · ${status}${live&&!stopping?' · next stage waits for recovery':''}`;
}
