import {queryWorker} from './workers.mjs';
export function csvCell(value){let text=String(value??'');if(/^[=+@\-\t\r]/.test(text))text="'"+text;return '"'+text.replaceAll('"','""')+'"';}
// Read one bounded page per pull. Cancelling the download stops further queries.
export function csvStream(task,args,columns){
 let cursor={},started=false,done=false;const encoder=new TextEncoder();
 return new ReadableStream({async pull(controller){if(done)return;try{if(!started){controller.enqueue(encoder.encode(columns.join(',')+'\r\n'));started=true;return;}
 const page=await queryWorker(task,{...args,...cursor,limit:200});if(done)return;
 controller.enqueue(encoder.encode(page.rows.map(row=>columns.map(c=>csvCell(row[c])).join(',')).join('\r\n')+(page.rows.length?'\r\n':'')));
 if(page.next){cursor=typeof page.next==='number'?{after:page.next}:page.next;}else{done=true;controller.close();}
 }catch(e){done=true;controller.error(e);}},cancel(){done=true;}});
}
