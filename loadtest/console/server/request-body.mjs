import {HttpError} from './errors.mjs';
export async function boundedRequest(request,limit=1024*1024){
 if(!request.body)return request;
 if(Number(request.headers.get('content-length')||0)>limit)throw new HttpError(413,'Request body exceeds 1 MiB');
 const reader=request.body.getReader(),chunks=[];let bytes=0;
 try{for(;;){const {done,value}=await reader.read();if(done)break;bytes+=value.byteLength;if(bytes>limit)throw new HttpError(413,'Request body exceeds 1 MiB');chunks.push(value);}}
 // Cancelling the adapter's stream destroys the TCP socket before a 413 can
 // reach the client. Return a close-delimited rejection instead of buffering
 // or draining the remaining untrusted body.
 catch(error){reader.releaseLock();throw error;}
 return new Request(request,{body:Buffer.concat(chunks)});
}
