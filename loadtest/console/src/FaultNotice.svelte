<script>
import {faultNotice} from './fault-notice.mjs';
export let faults;
export let stopping=false;
export let activity=[];
export let databaseHealth=null;
export let onRetry=()=>{};
$: notice=faultNotice(faults,{stopping,activity,databaseHealth});
</script>

{#if notice}
<div class="fault-notice" class:warning={notice.tone==='warning'} role="status">
  <div class="notice-copy">
    <div class="notice-heading"><strong>{notice.title}</strong>{#if notice.availability}<span class="availability" class:positive={notice.availability.tone==='positive'} class:negative={notice.availability.tone==='negative'}>{notice.availability.label}</span>{/if}</div>
    <p><span class="notice-dot" aria-hidden="true"></span>{notice.description}</p>
  </div>
  {#if notice.canRetry}<button type="button" class="quiet notice-action" disabled={faults.recovering} onclick={onRetry}>{notice.retryLabel}</button>{/if}
</div>
{/if}

<style>
.fault-notice{display:flex;align-items:center;justify-content:space-between;gap:20px;margin:16px 0 20px;padding:4px 0}
.notice-copy{min-width:0}.notice-heading{display:flex;align-items:center;flex-wrap:wrap;gap:8px 12px}.notice-heading strong{font-size:12px;font-weight:600;color:var(--ink);line-height:1.5}
.availability{display:inline-flex;align-items:center;gap:6px;padding-left:12px;border-left:1px solid var(--border);font-size:11px;line-height:1.5}.availability::before{content:'';width:5px;height:5px;border-radius:50%;background:currentColor}.availability.positive{color:var(--success)}.availability.negative{color:var(--danger)}
.notice-copy p{display:flex;align-items:center;gap:7px;margin:5px 0 0;font-size:12px;line-height:1.5;color:var(--muted)}.notice-dot{width:5px;height:5px;border-radius:50%;background:var(--subtle);flex:0 0 auto}.warning .notice-dot{background:var(--warning)}
.fault-notice .notice-action{flex:0 0 auto;min-height:32px;margin:0;padding:5px 10px;font-size:12px;font-weight:500;color:var(--ink)}
@media(max-width:600px){.fault-notice{gap:12px}.notice-copy p{align-items:baseline}.notice-dot{transform:translateY(-1px)}}
</style>
