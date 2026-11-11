// The router reports internal processing milliseconds when requested with true.
export function serverLatency(headers) {
  const entry=Object.entries(headers||{}).find(([name])=>name.toLowerCase()==='x-hs-latency');
  if(!entry||typeof entry[1]!=='string')return null;
  const text=entry[1].trim();
  if(!/^\d+(?:\.\d+)?$/.test(text))return null;
  const value=Number(text);return Number.isFinite(value)?value:null;
}
export function latencyHeaders(headers) {
  return {...Object.fromEntries(Object.entries(headers||{}).filter(([name])=>name.toLowerCase()!=='x-hs-latency')),'x-hs-latency':'true'};
}
