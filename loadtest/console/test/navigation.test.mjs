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

test('Settings has its own clean URL and preserves the selected run and metrics when returning',()=>{
 globalThis.window={location:{href:'http://localhost/performance?profile=p&fixtures=f&run=recorded&metrics=grafana&metric=serverP99'}};
 try{
  setQuery({metrics:'grafana'});
  assert.equal(sectionHref('settings',new URL(window.location.href),navigationContext.get()),'/settings');
  window.location.href='http://localhost/settings?section=grafana';setQuery({section:'database'});
  assert.equal(sectionHref('settings',new URL(window.location.href),navigationContext.get()),'/settings?section=database');
  const back=new URL(sectionHref('performance',new URL(window.location.href),navigationContext.get()),'http://localhost');
  assert.equal(back.searchParams.get('run'),'recorded');assert.equal(back.searchParams.get('profile'),'p');assert.equal(back.searchParams.get('fixtures'),'f');assert.equal(back.searchParams.get('metrics'),'grafana');assert.equal(back.searchParams.get('metric'),'serverP99');
 }finally{delete globalThis.window;}
});

test('settings shortcuts accept only known sections and retain the connections default',async()=>{
 const {settingsSections,settingsHref,settingsSection}=await import('../src/settings-sections.mjs');
 for(const [id]of settingsSections){assert.equal(settingsSection(id),id);assert.equal(settingsHref(id),'/settings?section='+id);}
 assert.equal(settingsHref(),'/settings?section=connections');
 assert.equal(settingsSection('javascript:alert(1)'),'connections');
});
