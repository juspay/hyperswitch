export function faultNotice(faults,{live=false,stopping=false,activity=[]}={}) {
 const active=faults?.active;
 if(!active&&!faults?.blocked)return '';
 const label=active?.target||'Fault recovery';
 const databaseAvailability=active?.databaseAvailability??[...activity].reverse().find(e=>['database_available','database_unavailable'].includes(e.kind)&&e.at>=(active?.recoveryAt||Infinity)-10000)?.kind?.replace('database_','');
 if(databaseAvailability==='available'&&['restoring','recovery_failed'].includes(active.status)) {
  return active.status==='recovery_failed'?`${label} · DB available · failover completion not verified`:`${label} · recovered · cloud operation finishing in background`;
 }
 const status=active?.status==='recovery_failed'?'recovery not verified':active?.status||'pending';
 return `${label} · ${status}${live&&!stopping?' · next stage waits for recovery':''}`;
}
