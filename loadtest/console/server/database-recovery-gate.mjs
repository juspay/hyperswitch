// SQL recovery releases traffic independently of the provider operation.
// Do not release on an initial successful probe before failover takes effect.
export function databaseRecoveryGate(entry,release) {
 let pending=null;
 return async current=>{
  if(current==='unavailable')entry.databaseOutageObserved=true;
  if(current!=='available'||!entry.databaseOutageObserved||entry.status!=='restoring'||entry.loadReleased)return;
  if(pending)return pending;
  pending=(async()=>{if(await release())entry.loadReleased=true;})();
  try{await pending;}finally{pending=null;}
 };
}
