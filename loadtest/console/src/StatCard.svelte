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
  article{position:relative;overflow:hidden;isolation:isolate;background:#fff;border:1px solid #e1e5eb;border-radius:12px;padding:20px 22px 64px;min-height:174px}
  span,em{position:relative;display:block;color:#667085;font-size:13px;font-style:normal;line-height:1.5}
  span{font-weight:500}
  strong{position:relative;display:block;font-size:34px;line-height:1.2;margin:10px 0 6px;color:var(--trend-color);font-weight:650;letter-spacing:-1px;font-variant-numeric:tabular-nums}
  small{font-size:15px;font-weight:500;margin-left:4px;letter-spacing:0;color:#667085}
  svg{position:absolute;z-index:-1;inset:auto 0 0;width:100%;height:46px;overflow:hidden}
  polygon{fill:var(--trend-color);opacity:.07}polyline{fill:none;stroke:var(--trend-color);stroke-width:1.5;vector-effect:non-scaling-stroke}
</style>
