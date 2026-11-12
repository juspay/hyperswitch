import {latencyPercentile} from './successful-latency.mjs';

// Match by flow type and time only. Never select samples by their latency.
export function matchedLatencyDrift(samples,windows,{toleranceSeconds=5}={}){
 const ordered=[...windows].sort((a,b)=>a.from-b.from),duration=ordered.reduce((n,w)=>n+w.to-w.from,0);
 const width=duration/3,tolerance=Math.min(toleranceSeconds*1000,duration/12);
 const position=at=>ordered.reduce((n,w)=>n+Math.max(0,Math.min(at,w.to)-w.from),0);
 const rows=samples.map(row=>({...row,position:position(row.observed_at),flow:row.flow_name||String(row.flow_id||'unknown').split(':').at(-3)||'unknown'})).sort((a,b)=>a.observed_at-b.observed_at);
 const first=[],last=[],counts=[];
 for(const flow of [...new Set(rows.map(r=>r.flow))].sort()){
  const type=rows.filter(r=>r.flow===flow),early=type.filter(r=>r.position<width+tolerance),late=type.filter(r=>r.position>=duration-width-tolerance);
  const target=Math.max(type.filter(r=>r.position<width).length,type.filter(r=>r.position>=duration-width).length);
  const count=Math.min(target,early.length,late.length);
  if(!count)return {percent:null,reason:`Insufficient matched samples for ${flow}`,counts:[],firstMedianMs:null,lastMedianMs:null};
  first.push(...early.slice(0,count));last.push(...late.slice(-count));counts.push({flow,count});
 }
 const median=rows=>latencyPercentile(rows.map(r=>r.server_latency_ms),.5),firstMedianMs=median(first),lastMedianMs=median(last);
 const percent=firstMedianMs>0&&lastMedianMs!=null?Math.abs(100*(lastMedianMs-firstMedianMs)/firstMedianMs):firstMedianMs===0&&lastMedianMs===0?0:null;
 const bounds=rows=>rows.length?rows.reduce((b,r)=>({from:Math.min(b.from,r.observed_at),to:Math.max(b.to,r.observed_at)}),{from:Infinity,to:-Infinity}):null;
 return {percent,firstMedianMs,lastMedianMs,counts,samplesPerWindow:first.length,windowSeconds:width/1000,toleranceSeconds:tolerance/1000,firstWindow:bounds(first),lastWindow:bounds(last)};
}
