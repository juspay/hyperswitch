#!/usr/bin/env node
//! Check a generated alarm snapshot against the IaC catalogue it was transcribed from.
//!
//! Re-derives the entries with a second, line-oriented parser — deliberately not the brace-matching
//! one `cloudwatch_alarm_snapshot.mjs` uses — and deep-compares. A parser bug has to occur twice, in
//! two different shapes, to pass unnoticed. Run it after regenerating, and when reviewing a change
//! to either the catalogue or the snapshot.
//!
//!   node scripts/cloudwatch_alarm_snapshot_verify.mjs \
//!     <path to cloudwatch-alarms/terragrunt.hcl> config/cloudwatch_alarms.sandbox.json
import { readFileSync } from 'node:fs';
const [hcl, json] = process.argv.slice(2);
const lines = readFileSync(hcl, 'utf8').split('\n');

const DIMS = {
  rds: { DBClusterIdentifier: 'hyperswitchdb-cluster' },
  'rds-primary': { DBInstanceIdentifier: 'hyperswitchdb-primary' },
  'rds-failover': { DBInstanceIdentifier: 'failover-replica-1' },
  'rds-writer': { DBClusterIdentifier: 'hyperswitchdb-cluster', Role: 'WRITER' },
  'rds-reader': { DBClusterIdentifier: 'hyperswitchdb-cluster', Role: 'READER' },
};
const val = (s) => (s.startsWith('"') ? s.slice(1, -1) : Number(s));

let inMetricBlock = false, def = null, sev = null;
const defs = {};
for (const line of lines) {
  if (/^  classified_metric_alarms = merge\(\{/.test(line)) { inMetricBlock = true; continue; }
  if (/^  classified_anomaly_alarms/.test(line)) { inMetricBlock = false; continue; }
  if (!inMetricBlock) continue;

  let m;
  if ((m = /^    ([a-z0-9-]+) = \{$/.exec(line))) { def = m[1]; sev = null; defs[def] = { severities: {} }; continue; }
  if ((m = /^        (sev[0-9]) = \{$/.exec(line))) { sev = m[1]; defs[def].severities[sev] = {}; continue; }
  if (/^        \}$/.test(line)) { sev = null; continue; }
  if ((m = /^      ([a-z_]+)\s*= (.+)$/.exec(line)) && def && !sev) { if (m[2].trim() !== "{") defs[def][m[1]] = val(m[2].trim()); continue; }
  if ((m = /^          ([a-z_]+)\s*= (.+)$/.exec(line)) && sev) { defs[def].severities[sev][m[1]] = val(m[2].trim()); continue; }
}

const expected = [];
for (const [name, d] of Object.entries(defs)) {
  if (d.classification !== 'rds-alerts') continue;
  for (const [s, rule] of Object.entries(d.severities)) {
    expected.push({
      id: `${name}-${s}`,
      definition: name,
      alarm_name: `${s}-sandbox-ap-south-1-hyperswitch-${name}`,
      classification: 'rds-alerts',
      severity: s,
      metric_name: d.metric_name,
      namespace: d.namespace,
      dimension_key: d.dimension_key,
      dimensions: DIMS[d.dimension_key],
      period: d.period ?? 60,
      statistic: d.statistic ?? 'Average',
      threshold: rule.threshold,
      comparison_operator: rule.comparison_operator ?? 'GreaterThanThreshold',
      evaluation_periods: rule.evaluation_periods ?? 5,
      datapoints_to_alarm: rule.datapoints_to_alarm ?? null,
      treat_missing_data: rule.treat_missing_data ?? 'notBreaching',
      description: rule.description,
    });
  }
}

const actual = JSON.parse(readFileSync(json, 'utf8')).alarms;
const norm = (a) => JSON.stringify([...a].sort((x, y) => x.id.localeCompare(y.id)));
if (norm(expected) !== norm(actual)) {
  console.error(`MISMATCH: independent parse produced ${expected.length} entries, snapshot has ${actual.length}`);
  const byId = new Map(actual.map((a) => [a.id, a]));
  for (const e of expected) {
    const a = byId.get(e.id);
    if (!a) { console.error(`missing from snapshot: ${e.id}`); continue; }
    for (const k of Object.keys(e)) {
      if (JSON.stringify(e[k]) !== JSON.stringify(a[k])) console.error(`${e.id}.${k}: hcl=${JSON.stringify(e[k])} snapshot=${JSON.stringify(a[k])}`);
    }
  }
  process.exit(1);
}
console.log(`OK: ${expected.length} entries across ${new Set(expected.map((e) => e.definition)).size} definitions match the source`);
