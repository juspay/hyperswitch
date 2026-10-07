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
  settings: () => request('/settings'),
  saveSettings: (settings) => request('/settings', { method: 'PUT', body: JSON.stringify(settings) }),
  runs: () => request('/runs'),
  run: (id) => request(`/runs/${encodeURIComponent(id)}`),
  start: (command, mode) => request(command === 'fixtures' ? '/fixtures' : '/runs', { method: 'POST', body: JSON.stringify({ mode }) }),
  job: (id) => request(`/jobs/${encodeURIComponent(id)}`),
  cancel: (id) => request(`/jobs/${encodeURIComponent(id)}/cancel`, { method: 'POST' }),
  panels: (panels) => request('/grafana/panels', { method: 'PUT', body: JSON.stringify(panels) }),
  requests: (id, after = 0) => request(`/runs/${encodeURIComponent(id)}/requests?after=${after}`),
  reconcile: (id, allowIncomplete = false) => request(`/runs/${encodeURIComponent(id)}/reconcile`, { method: 'POST', body: JSON.stringify({ allowIncomplete }) })
};
