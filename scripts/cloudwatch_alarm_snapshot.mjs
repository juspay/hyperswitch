#!/usr/bin/env node
//! Transcribe the IaC CloudWatch alarm catalogue into the evaluator's input schema.
//!
//! TEMPORARY. hyperswitch-cloud#23444 replaces this with Terraform rendering the same JSON from
//! the `classified_alarms_flat` local the cloudwatch composition module already computes; this
//! script exists so the evaluator can be built and tested before that lands, without anybody
//! hand-typing twenty definitions and their thresholds.
//!
//! It is a transcription, not a second source of truth: every rule value comes from the terragrunt
//! file, and re-running it against a newer revision is how the snapshot is refreshed.
//!
//!   node scripts/cloudwatch_alarm_snapshot.mjs \
//!     <path to cloudwatch-alarms/terragrunt.hcl> <source revision> > config/cloudwatch_alarms.sandbox.json

import { readFileSync } from 'node:fs';

const [hclPath, revision] = process.argv.slice(2);
if (!hclPath || !revision) {
  console.error('usage: cloudwatch_alarm_snapshot.mjs <terragrunt.hcl> <source revision>');
  process.exit(2);
}

// Environment identity, from terraform/aws/live/sandbox/ap-south-1/root.hcl. The module builds
// alarm names as "<severity>-<environment>-<region>-<project>-<definition>"; carrying the name
// lets a dry run be diffed against the alarm AWS already evaluates.
const ENVIRONMENT = 'sandbox';
const REGION = 'ap-south-1';
const PROJECT = 'hyperswitch';
const NAME_PREFIX = `${ENVIRONMENT}-${REGION}-${PROJECT}`;

// The RDS half of `dimension_map`, resolved. In IaC these come from the database stack's outputs;
// all three are literals declared in terraform/aws/live/sandbox/ap-south-1/database/terragrunt.hcl
// (cluster_identifier, and the two instance identifiers), which is why a snapshot can resolve them
// without reading Terraform state.
const DIMENSION_MAP = {
  rds: { DBClusterIdentifier: 'hyperswitchdb-cluster' },
  'rds-primary': { DBInstanceIdentifier: 'hyperswitchdb-primary' },
  'rds-failover': { DBInstanceIdentifier: 'failover-replica-1' },
  'rds-writer': { DBClusterIdentifier: 'hyperswitchdb-cluster', Role: 'WRITER' },
  'rds-reader': { DBClusterIdentifier: 'hyperswitchdb-cluster', Role: 'READER' },
};

// Defaults from the module's own variable declaration (hyperswitch-suite,
// terraform/aws/modules/composition/cloudwatch/variables.tf @ cloudwatch-v0.1.0). A definition that
// omits a field is not unconfigured — it takes these, and so must the snapshot.
const DEFAULTS = {
  period: 60,
  statistic: 'Average',
  comparison_operator: 'GreaterThanThreshold',
  evaluation_periods: 5,
  treat_missing_data: 'notBreaching',
  datapoints_to_alarm: null,
};

const CLASSIFICATION = 'rds-alerts';

const source = readFileSync(hclPath, 'utf8');

/** Index just past `classified_metric_alarms = merge({`, where the definitions begin. */
function metricAlarmsStart(text) {
  const marker = /classified_metric_alarms\s*=\s*merge\(\{/g;
  const found = marker.exec(text);
  if (!found) throw new Error('classified_metric_alarms block not found');
  return found.index + found[0].length;
}

/** Index of the `}` closing the block that opened just before `from`. */
function matchingBrace(text, from) {
  let depth = 1;
  for (let i = from; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === '"') {
      i = text.indexOf('"', i + 1); // no escaped quotes in this catalogue
      if (i < 0) throw new Error('unterminated string');
    } else if (ch === '#') {
      i = text.indexOf('\n', i);
    } else if (ch === '{') {
      depth += 1;
    } else if (ch === '}') {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  throw new Error('unbalanced braces');
}

/**
 * Parse the HCL subset the catalogue is written in: `key = value` where a value is a quoted
 * string, a number, or a nested block. Anything else — the `for` comprehensions that expand cache
 * and envoy templates — is left alone; those classifications are not this snapshot's scope.
 */
function parseBlock(text, start, end) {
  const block = {};
  let i = start;
  while (i < end) {
    const ch = text[i];
    if (ch === '#') { i = text.indexOf('\n', i) + 1; continue; }
    if (/\s/.test(ch)) { i += 1; continue; }

    const assignment = /([A-Za-z_][A-Za-z0-9_-]*)\s*=\s*/y;
    assignment.lastIndex = i;
    const key = assignment.exec(text);
    if (!key) { i += 1; continue; } // a `for` expression or similar; skip the character

    i = assignment.lastIndex;
    if (text[i] === '{') {
      const close = matchingBrace(text, i + 1);
      block[key[1]] = parseBlock(text, i + 1, close);
      i = close + 1;
    } else if (text[i] === '"') {
      const close = text.indexOf('"', i + 1);
      block[key[1]] = text.slice(i + 1, close);
      i = close + 1;
    } else {
      const literal = /-?[0-9.]+|true|false|null/y;
      literal.lastIndex = i;
      const value = literal.exec(text);
      if (!value) throw new Error(`unparsed value at ${text.slice(i, i + 60)}`);
      block[key[1]] = value[0] === 'true' ? true
        : value[0] === 'false' ? false
        : value[0] === 'null' ? null
        : Number(value[0]);
      i = literal.lastIndex;
    }
  }
  return block;
}

const blockStart = metricAlarmsStart(source);
const definitions = parseBlock(source, blockStart, matchingBrace(source, blockStart));

const alarms = [];
for (const [definition, config] of Object.entries(definitions)) {
  if (config.classification !== CLASSIFICATION) continue;
  const dimensions = DIMENSION_MAP[config.dimension_key];
  if (!dimensions) throw new Error(`no resolved dimensions for key ${config.dimension_key}`);

  for (const [severity, rule] of Object.entries(config.severities)) {
    alarms.push({
      id: `${definition}-${severity}`,
      definition,
      alarm_name: `${severity}-${NAME_PREFIX}-${definition}`,
      classification: config.classification,
      severity,
      metric_name: config.metric_name,
      namespace: config.namespace,
      dimension_key: config.dimension_key,
      dimensions,
      period: config.period ?? DEFAULTS.period,
      statistic: config.statistic ?? DEFAULTS.statistic,
      threshold: rule.threshold,
      comparison_operator: rule.comparison_operator ?? DEFAULTS.comparison_operator,
      evaluation_periods: rule.evaluation_periods ?? DEFAULTS.evaluation_periods,
      datapoints_to_alarm: rule.datapoints_to_alarm ?? DEFAULTS.datapoints_to_alarm,
      treat_missing_data: rule.treat_missing_data ?? DEFAULTS.treat_missing_data,
      description: rule.description,
    });
  }
}

// A definition without a threshold is an anomaly rule, which this evaluator does not support and
// which the catalogue keeps in a separate `classified_anomaly_alarms` block. Refuse rather than
// emit an entry with a null threshold.
for (const alarm of alarms) {
  if (typeof alarm.threshold !== 'number') throw new Error(`${alarm.id} has no threshold`);
  if (!alarm.description) throw new Error(`${alarm.id} has no description`);
}

const catalogue = {
  version: 1,
  source: {
    repo: 'hyperswitch-infra',
    path: 'terraform/aws/live/sandbox/ap-south-1/cloudwatch-alarms/terragrunt.hcl',
    revision,
    environment: ENVIRONMENT,
    region: REGION,
    classifications: [CLASSIFICATION],
    produced_by: 'scripts/cloudwatch_alarm_snapshot.mjs',
    note: 'Temporary transcription. hyperswitch-cloud#23444 replaces it with Terraform-rendered output in this same shape.',
  },
  alarms,
};

process.stdout.write(`${JSON.stringify(catalogue, null, 2)}\n`);
