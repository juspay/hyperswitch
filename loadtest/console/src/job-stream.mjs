// The server owns jobs. Reconnecting only restores observation; it never stops work.
export function watchJobStream(id,onProgress,onConnection=(_state,_last)=>{}){
  let closed=false,busy=false,last=0;
  const connection=state=>onConnection(state,last);
  const receive=value=>{if(closed)return;last=Date.now();connection('connected');Promise.resolve(onProgress(value)).catch(()=>connection('reconnecting'));};
  const stream=new EventSource(`/api/jobs/${encodeURIComponent(id)}/events`);
  connection('connecting');stream.onopen=()=>connection('connected');
  stream.addEventListener('progress',event=>{try{receive(JSON.parse(event.data));}catch{connection('reconnecting');}});
  stream.onerror=()=>connection('reconnecting');
  const poll=setInterval(async()=>{if(closed||busy||Date.now()-last<10000)return;busy=true;try{const response=await fetch(`/api/jobs/${encodeURIComponent(id)}`);if(!response.ok)throw Error();receive(await response.json());}catch{connection('reconnecting');}finally{busy=false;}},5000);
  return {close(){closed=true;clearInterval(poll);stream.close();}};
}
