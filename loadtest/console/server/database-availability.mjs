import {setTimeout as wait} from 'node:timers/promises';

// Measure the configured SQL connection independently of the provider operation.
// Poll sequentially: a slow probe must not overlap with the next one.
export function watchDatabaseAvailability({check,onChange,onSample=(_current)=>{},initial='available',intervalMs=1000}){
 const controller=new AbortController();let previous=initial;
 const done=(async()=>{
  while(!controller.signal.aborted){
   let current;
   try{current=(await check())?.ok===true?'available':'unavailable';}
   catch{current='unavailable';}
   if(controller.signal.aborted)break;
   await onSample(current);
   if(current!==previous){previous=current;onChange(current);}
   try{await wait(intervalMs,undefined,{signal:controller.signal});}catch{break;}
  }
 })();
 return {done,stop:async()=>{controller.abort();await done;}};
}
