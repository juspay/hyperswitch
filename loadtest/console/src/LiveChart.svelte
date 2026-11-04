<script>
  import {onMount} from 'svelte';
  import {plannedEvents,stageWallTimes,executionPhases,targetAt,wallToActive,availabilityGraphEvents} from './chart-timeline.mjs';
  export let points=[];
  export let allPoints=[];
  export let timeline=[];
  export let durationSeconds=0;
  export let activity=[];
  export let startedAt=0;
  export let flow='all';
  export let flows=[];
  export let metric='tps';
  /** @type {any} */
  export let runStatus=null;
  let root,plotWidth=900;
  let hover=null;
  function inspect(event){
    const svg=event.currentTarget,rect=svg.getBoundingClientRect();
    const coordinate=(event.clientX-rect.left)*plotWidth/rect.width;
    if(coordinate<48||coordinate>plotWidth-16){hover=null;return;}
    hover={t:(coordinate-48)/span*end,left:Math.max(8,Math.min(rect.width-236,event.clientX-rect.left+14)),top:Math.max(8,Math.min(rect.height-110,event.clientY-rect.top-90))};
  }
  const formatValue=value=>value==null||!Number.isFinite(value)?'—':metric==='backlog'?Math.round(value).toLocaleString():`${value.toFixed(metric==='tps'?2:1)}${['p99','serverP99'].includes(metric)?' ms':metric==='errors'?'%':''}`;
  onMount(()=>{const observer=new ResizeObserver(entries=>plotWidth=Math.max(260,entries[0].contentRect.width));observer.observe(root);return()=>observer.disconnect();});
  $: span=plotWidth-64;
  const choices={tps:'Flows / second',p99:'Client-side latency · ms',serverP99:'Server-side latency · ms',errors:'HTTP errors · %',backlog:'Pending writes · events'};
  $: rawSamples=points.filter(point=>Number.isFinite(point?.t));
  $: samples=rawSamples.filter((_,i)=>i%Math.max(1,Math.ceil(rawSamples.length/600))===0||i===rawSamples.length-1);
  const metricValue=(p,m=metric,f=flow)=>f==='all'?p?.[m]:(m==='tps'?p?.flows?.[f]?.tps:['p99','serverP99'].includes(m)?p?.flows?.[f]?.[m]:p?.[m]);
  $: successful=samples.map(p=>({t:p.t,value:metricValue(p,metric,flow)}));
  $: timingPoints=allPoints.length?allPoints:points;
  $: wallTimeline=stageWallTimes(timeline,timingPoints,activity,startedAt);
  $: offered=metric==='tps'&&flow==='all'?wallTimeline.flatMap(stage=>[
    {t:stage.wallStart,value:stage.from},
    {t:stage.wallEnd,value:stage.target}
  ]):[];
  $: maximum=metric==='errors'?100:Math.max(1,...[...successful,...offered].map(p=>Number(p.value)||0))*1.15;
  $: planned=plannedEvents(timeline);
  $: end=Math.max(1,Number(durationSeconds)||0,rawSamples.at(-1)?.t||0,wallTimeline.at(-1)?.wallEnd||0,planned.at(-1)?.t||0);
  $: recordedUntil=Math.min(end,rawSamples.at(-1)?.t||0);
  $: x=t=>48+Number(t)/end*span;
  const y=value=>220-(Number(value)||0)/maximum*185;
  const line=(values,horizon,scale,width)=>values.map((p,i)=>`${i?'L':'M'}${(48+p.t/horizon*width).toFixed(1)},${(220-p.value/scale*185).toFixed(1)}`).join(' ');
  $: valid=successful.filter(p=>p.value!=null&&Number.isFinite(p.value));
  // Rate samples describe the interval ending at their timestamp.
  $: rateIntervals=valid.flatMap((point,index)=>[{t:index?valid[index-1].t:0,value:point.value},point]);
  $: measuredValues=metric==='tps'?rateIntervals:valid;
  $: measuredPath=line(measuredValues,end,maximum,span);
  $: offeredPath=line(offered,end,maximum,span);
  const markerColor=e=>e.kind==='failover_requested'?'#5277b8':e.kind==='database_unavailable'?'#c43b36':e.kind==='database_available'?'#398453':e.kind==='fault_requested'?'#c43b36':e.kind==='fault_started'?'#8f2420':e.kind==='fault_restore'?'#d68a25':e.kind==='fault_restored'?'#398453':'#8793a5';
  $: markers=availabilityGraphEvents(activity,timeline).map(e=>({...e,t:Math.max(0,(e.at-startedAt)/1000)})).filter(e=>e.t>=0&&e.t<=end);
  $: phases=executionPhases(timeline,timingPoints,activity,startedAt,end);
  $: hoveredSample=hover&&hover.t<=recordedUntil?(metric==='tps'?rawSamples.find(p=>p.t>=hover.t)||rawSamples.at(-1):rawSamples.reduce((nearest,p)=>!nearest||Math.abs(p.t-hover.t)<Math.abs(nearest.t-hover.t)?p:nearest,null)):null;
  $: hoveredValue=hoveredSample?metricValue(hoveredSample,metric,flow):null;
  $: hoveredPhase=hover?phases.find(p=>hover.t>=p.start&&hover.t<p.end):null;
  $: hoveredEvent=hover?markers.filter(e=>Math.abs(e.t-hover.t)<end*8/span).sort((a,b)=>Math.abs(a.t-hover.t)-Math.abs(b.t-hover.t))[0]:null;
  $: hoveredTarget=hover?targetAt(wallToActive(hover.t,timingPoints),timeline):null;
</script>
<section class="chart" aria-label="Load metrics over time">
  <div class="chart-head"><h2>Load timeline</h2>
    {#if runStatus}<div class="actions">
      {#if runStatus.live!==false}
        {#if runStatus.state==='idle'||(runStatus.state==='starting'&&runStatus.startLoad)}<button class="secondary" disabled={runStatus.pending||runStatus.startBlocked} onclick={runStatus.smokeTest}>Run smoke test</button><button disabled={runStatus.pending||runStatus.startBlocked} onclick={runStatus.startLoad}>{runStatus.pending?'Starting…':'Start load'}</button>
        {:else if runStatus.stopping}<button class="secondary danger" disabled aria-busy="true">Stopping…</button>
        {:else if runStatus.smoke}<button class="secondary" disabled={runStatus.pending} onclick={runStatus.cancel}>Stop smoke test</button>
        {:else}
          {#if runStatus.state==='running'}<button disabled={runStatus.pending||runStatus.faultActive} onclick={runStatus.pause}>Ⅱ Pause</button>{/if}
          {#if runStatus.state==='pausing'}<button disabled>Draining active flows…</button>{/if}
          {#if runStatus.state==='paused'}<button disabled={runStatus.pending} onclick={runStatus.resume}>▶ Resume</button>{/if}
          {#if ['running','paused','pausing','recovering','starting'].includes(runStatus.state)}<button class="secondary danger" disabled={runStatus.pending} onclick={runStatus.cancel}>Stop</button>{/if}
        {/if}
      {:else}<button class="secondary" onclick={runStatus.runAgain}>Run again</button>{#if !runStatus.smoke}<a class="button" href={runStatus.resultsUrl}>View results</a>{/if}{/if}
    </div>{/if}
  </div>
  {#if runStatus}<div class="run-toolbar timeline-run-toolbar">
    <div><span class="state-badge" class:running={!runStatus.stopping&&(runStatus.state==='running'||runStatus.smoke)} class:paused={runStatus.state==='paused'} title={runStatus.reason} role="status">{runStatus.stopping?'Stopping…':runStatus.stateLabel||(runStatus.state==='idle'?'Ready':null)||runStatus.state[0].toUpperCase()+runStatus.state.slice(1)}</span><strong>{runStatus.label}</strong><span class="run-clock">{runStatus.clock}</span></div>
  </div>
  {#if runStatus.stopping}<p class="stop-notice" role="status">Stop requested · waiting for active requests to finish and fault recovery to be verified.</p>{/if}
  {/if}
  <slot/>
  <div class="chart-filters">{#if flows.length>1}<select aria-label="Flow" disabled={['errors','backlog'].includes(metric)} bind:value={flow}><option value="all">All flows</option>{#each flows as item}<option value={item.name}>{item.label}</option>{/each}</select>{/if}<select aria-label="Metric" bind:value={metric}>{#each Object.entries(choices) as [key,label]}<option value={key}>{label}</option>{/each}</select></div>
  <div class="plot" bind:this={root}>
  <svg onpointermove={inspect} onpointerleave={()=>hover=null} viewBox={`0 0 ${plotWidth} 255`} role="img" aria-label={`${choices[metric]} over the full ${Math.round(end)} second test; grey indicates unexecuted time`}>
    <rect x={x(recordedUntil)} y="35" width={Math.max(0,x(end)-x(recordedUntil))} height="185" fill="#f0f2f5"/>
    {#each planned as event}<line x1={x(event.t)} x2={x(event.t)} y1="35" y2="220" stroke={event.kind==='fault'||event.kind==='recovery'?'#d68a25':'#b7bfcc'} stroke-width="0.7" stroke-dasharray="2 6"><title>{event.t.toFixed(1)}s · {event.label}</title></line>{/each}
    {#each [0,.5,1] as fraction}<line x1="48" x2={plotWidth-16} y1={y(maximum*fraction)} y2={y(maximum*fraction)} stroke="#e9edf3"/><text x="37" y={y(maximum*fraction)+4} text-anchor="end">{(maximum*fraction).toFixed(metric==='tps'?1:0)}</text>{/each}
    {#if valid.length}<path d={`${measuredPath} L${x(valid.at(-1).t)},220 L${x(measuredValues[0].t)},220 Z`} fill="#0561e20a"/><path d={measuredPath} fill="none" stroke="#0561e2" stroke-width="1.1"><title>{metric==='tps'?'Flows started per second':choices[metric]}</title></path>{/if}
    {#if offered.some(p=>p.value!=null)}<path d={offeredPath} fill="none" stroke="#98a2b3" stroke-width="0.8" stroke-dasharray="4 4"><title>Target flow rate</title></path>{/if}
    {#each markers as event}<line x1={x(event.t)} x2={x(event.t)} y1="28" y2="220" stroke={markerColor(event)} stroke-dasharray={event.kind==='failover_requested'?'4 3':undefined} stroke-width="0.8"><title>{event.t.toFixed(1)}s · {event.message}</title></line><circle cx={x(event.t)} cy="25" r="2.3" fill={markerColor(event)}><title>{event.t.toFixed(1)}s · {event.message}</title></circle>{/each}
    {#if hover}<line x1={x(hover.t)} x2={x(hover.t)} y1="35" y2="220" stroke="#8793a5" stroke-width=".7" stroke-dasharray="3 3"/>{#if hoveredValue!=null}<circle cx={x(hoveredSample.t)} cy={y(hoveredValue)} r="2.5" fill="#0561e2"/>{/if}{/if}
    {#if hover}<text class="hover-seconds" x={Math.max(78,Math.min(plotWidth-46,x(hover.t)))} y="243" text-anchor="middle">{hover.t.toFixed(1)}s</text>{:else}<text x="48" y="243">0s</text><text x={48+span/2} y="243" text-anchor="middle">{Math.round(end/2)}s</text><text x={plotWidth-16} y="243" text-anchor="end">{Math.round(end)}s</text>{/if}
    {#if !valid.length}<text x={48+span/2} y="130" text-anchor="middle">{rawSamples.length?'No metric samples yet':'Not run yet'}</text>{/if}
  </svg>
  {#if hover}<div class="chart-tooltip" style:left={`${hover.left}px`} style:top={`${hover.top}px`}>
    <strong>{hover.t.toFixed(1)}s <span>{hoveredSample?hoveredPhase?.label||'Load': 'Not run yet'}</span></strong>
    {#if hoveredSample}<div>{choices[metric]} <b>{formatValue(hoveredValue)}</b></div>{#if metric==='tps'&&flow==='all'&&hoveredTarget!=null}<div>Target <b>{hoveredTarget.toFixed(1)} TPS</b></div>{/if}{/if}
    {#if hoveredEvent}<p>{hoveredEvent.message} · {hoveredEvent.t.toFixed(1)}s</p>{/if}
  </div>{/if}
  </div>
</section>
<style>
.chart{background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);padding:20px;margin:0 0 16px}.chart-head{display:flex;align-items:center;justify-content:space-between;gap:16px}.chart-head h2{font-size:14px;margin:0}.chart-filters{display:flex;justify-content:flex-end;gap:8px;margin-top:12px}.chart-filters select{width:180px;min-height:34px;margin:0;font-size:12px;border-color:transparent;background-color:transparent;box-shadow:none;padding-left:8px}.plot{position:relative;margin:4px 0 0}svg{display:block;width:100%;height:255px}text{fill:var(--subtle);font-size:11px;font-family:inherit}
@media(max-width:600px){.chart{padding:16px}.chart-head{align-items:flex-start;flex-wrap:wrap;gap:12px}.chart-head .actions{margin-left:auto}.chart-filters{width:100%}.chart-filters select{width:50%;flex:1;min-width:0}svg{height:230px}}

.timeline-run-toolbar{flex-direction:row;justify-content:flex-start;align-items:center;flex-wrap:wrap;gap:12px 20px;border:0;border-bottom:1px solid var(--border);padding:12px 0 16px;margin:0 0 12px;border-radius:0;background:none}.chart-head .actions{display:flex;justify-content:flex-end;flex-wrap:wrap;margin:0 0 0 auto}.stop-notice{font-size:12px;color:var(--muted);margin:0 0 12px}
.hover-seconds{fill:var(--ink);font-weight:500}
.chart-tooltip{position:absolute;pointer-events:none;width:220px;padding:10px 12px;background:var(--surface);border:1px solid var(--border);border-radius:6px;box-shadow:0 3px 12px #18253b14;font-size:11px;z-index:2}.chart-tooltip strong{display:flex;justify-content:space-between;gap:8px;margin-bottom:7px;font-size:12px}.chart-tooltip strong span{font-weight:400;color:var(--muted)}.chart-tooltip div{display:flex;justify-content:space-between;gap:10px;line-height:1.8;color:var(--muted)}.chart-tooltip b{font-weight:500;color:var(--ink)}.chart-tooltip p{margin:6px 0 0;line-height:1.4}
</style>
