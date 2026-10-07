<script>
  import { tick } from 'svelte';
  import { k6Output } from './k6-output.mjs';
  export let output = '';
  let element;
  let following = true;

  function scrolled(){if(element.scrollHeight>element.clientHeight+24)following=element.scrollHeight-element.scrollTop-element.clientHeight<24;}
  async function update(_output){await tick();if(element && following)element.scrollTop=element.scrollHeight;}
  async function follow(){following=true;await update(output);}
  $: update(output);
  $: display = k6Output(output);
</script>

<div class="output-toolbar"><span>k6 output</span><button class="quiet" aria-pressed={following} onclick={() => {if(following)following=false;else follow();}}>{following ? 'Following output' : 'Follow output ↓'}</button></div>
<pre bind:this={element} onscroll={scrolled} aria-label="k6 output">{#if display.progress}<span>{display.before}{display.before ? '\n\n' : ''}</span><span aria-label="Current k6 progress">{display.progress}</span><span>{display.after ? '\n\n' : ''}{display.after}</span>{:else}{display.history}{/if}</pre>

<style>
  .output-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:8px;color:#667085;font-size:12px}
  .output-toolbar button{min-height:28px;font-size:12px}
  pre{margin:0;min-height:180px;max-height:none!important;overflow-x:auto;white-space:pre!important;line-height:1.55}
</style>
