async function request(path, options = {}) {
  const form=typeof FormData!=='undefined'&&options.body instanceof FormData;
  const response = await fetch(`/api${path}`, {
    ...options,
    headers: { ...(form?{}:{'content-type':'application/json'}), ...(options.headers || {}) }
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(body.error || `Request failed (${response.status})`);
  return body;
}

export const api = {
  profiles:()=>request('/profiles'),
  archiveProfile:(id,archived)=>request(`/profiles/${encodeURIComponent(id)}/archive`,{method:'PUT',body:JSON.stringify({archived})}),
  deleteProfile:id=>request(`/profiles/${encodeURIComponent(id)}`,{method:'DELETE'}),
  saveScenario:input=>request('/scenarios/save',{method:'POST',body:JSON.stringify(input)}),
  saveProfile:(profile)=>request(profile.id?`/profiles/${encodeURIComponent(profile.id)}`:'/profiles',{method:profile.id?'PUT':'POST',body:JSON.stringify(profile)}),
  fixtures:()=>request('/fixtures'),
  testDatabase:(database)=>request('/settings/database/test',{method:'POST',body:JSON.stringify(database)}),
  testService:(kind,endpoint,superposition)=>request('/settings/services/test',{method:'POST',body:JSON.stringify({kind,endpoint,superposition})}),
  settings: () => request('/settings'),
  certificates:()=>request('/settings/certificates'),
  importCertificate:(file,name='')=>{const form=new FormData();form.append('certificate',file);form.append('name',name);return request('/settings/certificates',{method:'POST',body:form});},
  removeCertificate:id=>request(`/settings/certificates/${encodeURIComponent(id)}`,{method:'DELETE'}),
  workers:()=>request('/workers'),
  workerPlan:id=>request(`/workers/plan?profile=${encodeURIComponent(id)}`),
  removeWorker:id=>request(`/workers/${encodeURIComponent(id)}`,{method:'DELETE'}),
  saveSettings: (settings) => request('/settings', { method: 'PUT', body: JSON.stringify(settings) }),
  runs: () => request('/runs'),
  run: (id) => request(`/runs/${encodeURIComponent(id)}`),
  start: (command, selection) => request(command === 'fixtures' ? '/fixtures' : command === 'smoke' ? '/smoke' : '/runs', { method: 'POST', body: JSON.stringify(typeof selection==='string'?{mode:selection}:selection) }),
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
  requests: (id, options = {}) => request(`/runs/${encodeURIComponent(id)}/requests?${new URLSearchParams(typeof options==='number'?{after:options}:options)}`),
  paymentBars:(id,options={})=>request(`/runs/${encodeURIComponent(id)}/payment-bars?${new URLSearchParams(options)}`),
  paymentPage:(id,options={})=>request(`/runs/${encodeURIComponent(id)}/payment-page?${new URLSearchParams(options)}`),
  paymentDetail:(id,options={})=>request(`/runs/${encodeURIComponent(id)}/payment-detail?${new URLSearchParams(options)}`),
  nextDiscrepancy:(id,after)=>request(`/runs/${encodeURIComponent(id)}/next-discrepancy?after=${after}`),
  reconciliationJob:(id)=>request(`/runs/${encodeURIComponent(id)}/reconcile`),
  reconcile: (id, allowIncomplete = false) => request(`/runs/${encodeURIComponent(id)}/reconcile`, { method: 'POST', body: JSON.stringify({ allowIncomplete }) })
};
