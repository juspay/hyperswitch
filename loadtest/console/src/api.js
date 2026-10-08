async function request(path, options = {}) {
  const response = await fetch(`/api${path}`, {
    ...options,
    headers: { 'content-type': 'application/json', ...(options.headers || {}) }
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(body.error || `Request failed (${response.status})`);
  return body;
}

export const api = {
  profiles:()=>request('/profiles'),
  saveProfile:(profile)=>request(profile.id?`/profiles/${encodeURIComponent(profile.id)}`:'/profiles',{method:profile.id?'PUT':'POST',body:JSON.stringify(profile)}),
  fixtures:()=>request('/fixtures'),
  testDatabase:(database)=>request('/settings/database/test',{method:'POST',body:JSON.stringify(database)}),
  settings: () => request('/settings'),
  saveSettings: (settings) => request('/settings', { method: 'PUT', body: JSON.stringify(settings) }),
  runs: () => request('/runs'),
  run: (id) => request(`/runs/${encodeURIComponent(id)}`),
  start: (command, selection) => request(command === 'fixtures' ? '/fixtures' : '/runs', { method: 'POST', body: JSON.stringify(typeof selection==='string'?{mode:selection}:selection) }),
  job: (id) => request(`/jobs/${encodeURIComponent(id)}`),
  cancel: (id) => request(`/jobs/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  control: (id,action) => request(`/jobs/${encodeURIComponent(id)}/${action}`, {method:'POST'}),
  faultTargets:()=>request('/faults/targets'),
  saveFaultTargets:targets=>request('/faults/targets',{method:'PUT',body:JSON.stringify(targets)}),
  checkFaultTarget:target=>request('/faults/check',{method:'POST',body:JSON.stringify(target)}),
  testFault:input=>request('/faults/test',{method:'POST',body:JSON.stringify(input)}),
  faults: () => request('/faults'),
  fireFault: (input) => request('/faults', {method:'POST',body:JSON.stringify(input)}),
  restoreFault: () => request('/faults/restore', {method:'POST'}),
  panels: (panels) => request('/grafana/panels', { method: 'PUT', body: JSON.stringify(panels) }),
  requests: (id, after = 0) => request(`/runs/${encodeURIComponent(id)}/requests?after=${after}`),
  reconciliationJob:(id)=>request(`/runs/${encodeURIComponent(id)}/reconcile`),
  reconcile: (id, allowIncomplete = false) => request(`/runs/${encodeURIComponent(id)}/reconcile`, { method: 'POST', body: JSON.stringify({ allowIncomplete }) })
};
