<script>
  import {activeSamples,eventActiveTime,plannedEvents,targetAt} from './chart-timeline.mjs';
  export let points=[];
  export let allPoints=[];
  export let timeline=[];
  export let durationSeconds=0;
  export let activity=[];
  export let startedAt=0;
  export let flow='all';
  export let flows=[];
  export let metric='tps';
  /** @type {number|null} */ let hover=null;
  const choices={tps:'Flows / second',p99:'Confirm p99 · ms',errors:'HTTP errors · %',backlog:'Pending writes · events'};
  $: active=activeSamples(points);
  $: samples=active.filter((_,i)=>i%Math.max(1,Math.ceil(active.length/600))===0||i===active.length-1);
  const metricValue=(p,m=metric,f=flow)=>f==='all'?p?.[m]:(m==='tps'?p?.flows?.[f]?.tps:m==='p99'?p?.flows?.[f]?.p99:p?.[m]);
  $: successful=samples.map(p=>({t:p.t,value:metricValue(p,metric,flow)}));
  $: offered=metric==='tps'&&flow==='all'?timeline.flatMap(s=>[{t:s.start,value:s.from},{t:s.end,value:s.target}]):[];
  $: maximum=Math.max(1,...[...successful,...offered].map(p=>Number(p.value)||0))*1.15;
  $: end=Math.max(1,Number(durationSeconds)||0,active.at(-1)?.t||0);
  $: recordedUntil=Math.min(end,active.at(-1)?.t||0);
  const x=t=>54+Number(t)/end*900;
  const y=value=>220-(Number(value)||0)/maximum*185;
  const line=(values,horizon,scale)=>values.map((p,i)=>`${i?'L':'M'}${(54+p.t/horizon*900).toFixed(1)},${(220-p.value/scale*185).toFixed(1)}`).join(' ');
  $: valid=successful.filter(p=>p.value!=null&&Number.isFinite(p.value));
  $: measuredPath=line(valid,end,maximum);
  $: offeredPath=line(offered,end,maximum);
  const markerColor=e=>e.kind==='fault_restored'?'#398453':e.kind==='fault_started'?'#c43b36':e.kind==='fault_restore'?'#d68a25':'#8793a5';
  $: markers=activity.filter(e=>['pause','resume','fault_started','fault_restore','fault_restored'].includes(e.kind)).map(e=>({...e,t:eventActiveTime(e,startedAt,allPoints.length?allPoints:points)})).filter(e=>e.t>=0&&e.t<=end);
  $: planned=plannedEvents(timeline);
  $: stage=hover==null?null:timeline.find(s=>hover>=s.start&&hover<s.end)||timeline.at(-1);
  $: sample=hover==null||hover>recordedUntil?null:active.reduce((best,p)=>!best||Math.abs(p.t-hover)<Math.abs(best.t-hover)?p:best,null);
  $: nearby=hover==null?[]:[...planned.map(e=>({...e,message:e.label,planned:true})),...markers].filter(e=>Math.abs(e.t-hover)<Math.max(.5,end*.015));
  $: nearest=nearby.filter(e=>!e.planned).sort((a,b)=>Math.abs(a.t-hover)-Math.abs(b.t-hover))[0] || nearby.filter(e=>e.kind!=='hold').sort((a,b)=>Math.abs(a.t-hover)-Math.abs(b.t-hover))[0];
  $: context=nearest?`${nearest.planned?'Planned · ':''}${nearest.message.replace(' (planned)','')}`:stage?.fault?`${stage.fault.target} · ${stage.fault.action}`:stage?.kind==='ramp'?'Ramping':'Hold';
  const shortLabels={tps:'TPS',p99:'p99',errors:'HTTP errors',backlog:'Pending writes'};
  const units={tps:'',p99:' ms',errors:'%',backlog:''};
  function inspect(event){const rect=event.currentTarget.getBoundingClientRect();hover=Math.max(0,Math.min(end,((event.clientX-rect.left)/rect.width*980-54)/900*end));}
</script>
<section class="chart" aria-label="Load metrics over time">
  <div class="chart-head"><h2>Load timeline</h2><div class="chart-filters">{#if flows.length}<select aria-label="Flow" disabled={['errors','backlog'].includes(metric)} bind:value={flow}><option value="all">All flows</option>{#each flows as item}<option value={item.name}>{item.label}</option>{/each}</select>{/if}<select aria-label="Metric" bind:value={metric}>{#each Object.entries(choices) as [key,label]}<option value={key}>{label}</option>{/each}</select></div></div>
  <div class="plot">
  <svg viewBox="0 0 980 255" role="img" aria-label={`${choices[metric]} over the full ${Math.round(end)} second test; grey indicates unexecuted time`} onpointermove={inspect} onpointerleave={()=>hover=null}>
    <rect x={x(recordedUntil)} y="35" width={Math.max(0,x(end)-x(recordedUntil))} height="185" fill="#f0f2f5"/>
    {#each planned as event}<line x1={x(event.t)} x2={x(event.t)} y1="35" y2="220" stroke={event.kind==='fault'||event.kind==='recovery'?'#d68a25':'#b7bfcc'} stroke-dasharray="2 6"><title>{event.t}s · {event.label}</title></line>{/each}
    {#each [0,.5,1] as fraction}<line x1="54" x2="954" y1={y(maximum*fraction)} y2={y(maximum*fraction)} stroke="#e9edf3"/><text x="43" y={y(maximum*fraction)+4} text-anchor="end">{(maximum*fraction).toFixed(metric==='tps'?1:0)}</text>{/each}
    {#if valid.length}<path d={`${measuredPath} L${x(valid.at(-1).t)},220 L${x(valid[0].t)},220 Z`} fill="#0561e210"/><path d={measuredPath} fill="none" stroke="#0561e2" stroke-width="2"/>{/if}
    {#if offered.some(p=>p.value!=null)}<path d={offeredPath} fill="none" stroke="#98a2b3" stroke-width="1.5" stroke-dasharray="5 4"/>{/if}
    {#each markers as event}<line x1={x(event.t)} x2={x(event.t)} y1="28" y2="220" stroke={markerColor(event)} stroke-width="1.5"><title>{event.message}</title></line><circle cx={x(event.t)} cy="25" r="4" fill={markerColor(event)}><title>{event.message}</title></circle>{/each}
    {#if hover!=null}<line x1={x(hover)} x2={x(hover)} y1="35" y2="220" stroke="#667085"/>{/if}
    <text x="54" y="243">0s</text><text x="504" y="243" text-anchor="middle">{Math.round(end/2)}s</text><text x="954" y="243" text-anchor="end">{Math.round(end)}s</text>
    {#if !valid.length}<text x="500" y="130" text-anchor="middle">{active.length?'No metric samples yet':'Not run yet'}</text>{/if}
  </svg>
  {#if hover!=null}<div class="chart-tooltip" role="tooltip" style={`left:clamp(0px,${hover/end*100}%,max(0px,calc(100% - 360px)))`}>
    {#if hover!=null}<strong>{hover.toFixed(1)}s</strong><span>{sample?`${shortLabels[metric]} ${metricValue(sample)?.toFixed(metric==='backlog'?0:1) ?? '—'}${units[metric]}`:'Not run yet'}{#if metric==='tps'} <small>/ {targetAt(hover,timeline)?.toFixed(1) ?? '—'} target</small>{/if}</span><span class="hover-event" title={context}>{context}</span>{/if}
  </div>{/if}
  </div>
  <details class="timeline-details"><summary>Timeline · {timeline.length} stages</summary>
  <label class="inspect-time">Inspect time<input aria-label="Inspect chart time" type="range" min="0" max={end} step="0.1" value={hover ?? recordedUntil} oninput={event=>hover=Number(event.currentTarget.value)}/></label>
  {#if planned.length}<div class="planned-events">{#each planned as event}<button type="button" class="quiet" onclick={()=>hover=event.t}>{event.t}s · {event.label}</button>{/each}</div>{/if}
  </details>
  <div class="chart-legend"><span><i></i>{choices[metric]}</span>{#if offered.length}<span><i class="offered"></i>Offered TPS</span>{/if}<span class="muted">Dotted: planned · Grey: upcoming</span></div>
</section>
<style>
.plot{position:relative}.chart-tooltip{position:absolute;top:100%;display:flex;align-items:center;gap:10px;padding:8px 12px;max-width:min(360px,100%);flex-wrap:wrap;background:#202a3b;color:white;border-radius:7px;box-shadow:0 3px 10px #0002;font-size:11px;pointer-events:none;z-index:2;white-space:normal}.chart-tooltip small{color:#cbd5e1}.hover-event{max-width:200px;overflow:hidden;text-overflow:ellipsis}.plot{margin-bottom:52px}.chart-filters{display:flex;gap:8px}.chart-filters select{width:180px;min-height:34px;margin:0;font-size:12px}.timeline-details{font-size:12px;margin:12px 0;color:#667085}.timeline-details summary{cursor:pointer}.timeline-details[open] summary{margin-bottom:12px}@media(max-width:600px){.chart-head{align-items:flex-start;flex-direction:column}.chart-filters{width:100%}.chart-filters select{width:50%;min-width:0}.chart-tooltip{gap:8px;white-space:normal;flex-wrap:wrap;max-width:280px}.hover-event{max-width:180px}.plot{margin-bottom:58px}}.planned-events{max-height:84px;overflow:auto;display:flex;gap:8px;flex-wrap:wrap;margin:12px 0}.planned-events button{font-size:11px;border:1px solid #e5e9ef;border-radius:6px}.inspect-time{display:flex;align-items:center;gap:12px;font-size:11px}.inspect-time input{padding:0;margin:0;max-width:200px}.chart{background:white;border:1px solid var(--border);border-radius:12px;padding:20px;margin:0 0 20px}.chart-head{display:flex;align-items:center;justify-content:space-between;gap:16px}.chart-head h2{font-size:14px;margin:0}svg{display:block;width:100%;min-height:180px;margin-top:16px}text{fill:#98a2b3;font-size:11px;font-family:inherit}.chart-legend{display:flex;gap:20px;flex-wrap:wrap;font-size:11px;color:#667085}.chart-legend span{display:flex;align-items:center;gap:6px}.chart-legend i{width:16px;height:2px;background:#0561e2}.chart-legend i.offered{background:#98a2b3}.chart-legend .muted{margin-left:auto}@media(max-width:480px){.chart-head{align-items:flex-start;flex-direction:column}.chart-legend .muted{margin:0}}
</style>
