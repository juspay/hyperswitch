<script>
  export let tooltip='';
  export let label;
  export let value;
  export let unit = '';
  export let caption;
  export let series = [];
  export let color = '#5277b8';

  $: points = series.filter(p => Number.isFinite(p.t) && Number.isFinite(p.value));
  $: maximum = Math.max(1, ...points.map(p => p.value));
  $: start = points[0]?.t || 0;
  $: duration = Math.max(1, (points.at(-1)?.t || 0) - start);
  $: line = points.map(p => `${((p.t-start)/duration*300).toFixed(2)},${(69-Math.max(0,p.value)/maximum*63).toFixed(2)}`).join(' ');
</script>

<article title={tooltip || undefined} style:--trend-color={color}>
  {#if points.length > 1}
    <svg viewBox="0 0 300 70" preserveAspectRatio="none" aria-hidden="true">
      <polygon points={`0,70 ${line} 300,70`} />
      <polyline points={line} />
    </svg>
  {/if}
  <span>{label}</span><strong>{value}<small>{unit}</small></strong><em>{caption}</em>
</article>

<style>
article{position:relative;overflow:hidden;isolation:isolate;background:var(--surface);padding:18px 20px 40px;min-height:134px;border-right:1px solid var(--border)}article:last-child{border-right:0}span,em{position:relative;display:block;color:var(--muted);font-size:11px;font-style:normal;line-height:1.5}span{font-weight:500}strong{position:relative;display:block;font-size:29px;line-height:1.15;margin:8px 0 5px;color:var(--ink);font-weight:550;letter-spacing:-.8px;font-variant-numeric:tabular-nums}small{font-size:12px;font-weight:400;margin-left:5px;letter-spacing:0;color:var(--muted)}svg{position:absolute;z-index:-1;inset:auto 0 0;width:100%;height:26px;overflow:hidden}polygon{fill:var(--trend-color);opacity:.04}polyline{fill:none;stroke:var(--trend-color);opacity:.55;stroke-width:1.2;vector-effect:non-scaling-stroke}@media(max-width:900px){article:nth-child(2){border-right:0}article:nth-child(-n+2){border-bottom:1px solid var(--border)}}@media(max-width:480px){article{padding:16px 14px 36px}strong{font-size:26px}}
</style>
