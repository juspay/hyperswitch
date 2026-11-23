import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

// Keep the URL logic real while replacing SvelteKit's browser-only history API.
const source=(await readFile(new URL('../src/navigation.mjs',import.meta.url),'utf8'))
  .replace("import {writable} from 'svelte/store';",'const writable=value=>({set(next){value=next;},get(){return value;}});')
  .replace("import {replaceState} from '$app/navigation';",'const replaceState=url=>{window.location.href=url.href;};');
const {setQuery,sectionHref,navigationContext}=await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('metrics preference survives URL updates and section navigation',()=>{
  globalThis.window={location:{href:'http://localhost/performance?run=one&flow=guest'}};
  try{
    setQuery({metrics:'grafana'});
    assert.equal(new URL(window.location.href).searchParams.get('metrics'),'grafana');
    setQuery({run:'two'});
    const results=new URL(sectionHref('results',new URL(window.location.href),navigationContext.get()),'http://localhost');
    assert.equal(results.searchParams.get('metrics'),'grafana');
    assert.equal(results.searchParams.get('run'),'two');
    const fixtures=new URL(sectionHref('fixtures',results,navigationContext.get()),results);
    const performance=new URL(sectionHref('performance',fixtures,navigationContext.get()),results);
    assert.equal(performance.searchParams.get('metrics'),'grafana');
    setQuery({metrics:'native'});
    const native=new URL(sectionHref('results',new URL(window.location.href),navigationContext.get()),results);
    assert.equal(native.searchParams.get('metrics'),'native');
  }finally{delete globalThis.window;}
});
