"use strict";

function round(value) {
  return Math.round((value + Number.EPSILON) * 100) / 100;
}

function percentile(values, requestedPercentile) {
  const sorted = values.filter(Number.isFinite).slice().sort((left, right) => left - right);
  if (!sorted.length) return null;
  if (sorted.length === 1) return round(sorted[0]);
  const position = (Math.min(100, Math.max(0, requestedPercentile)) / 100) * (sorted.length - 1);
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  const weight = position - lower;
  return round(sorted[lower] + ((sorted[upper] - sorted[lower]) * weight));
}

function summaryRow(metric, values) {
  const valid = values.filter(Number.isFinite);
  if (!valid.length) return null;
  const total = valid.reduce((sum, value) => sum + value, 0);
  return {
    metric,
    count: valid.length,
    avg_ms: round(total / valid.length),
    p50_ms: percentile(valid, 50),
    p75_ms: percentile(valid, 75),
    p90_ms: percentile(valid, 90),
    p99_ms: percentile(valid, 99),
    max_ms: round(Math.max(...valid)),
  };
}

function numericValues(results, key) {
  return results
    .map((result) => result[key])
    .filter((value) => value !== null && value !== undefined && Number.isFinite(Number(value)))
    .map(Number);
}

function latencySummary(results) {
  return [
    summaryRow("pm_session_confirm", numericValues(results, "pm_session_confirm_latency_ms")),
    summaryRow("payment_confirm", numericValues(results, "payment_confirm_latency_ms")),
    summaryRow("hyperswitch_internal_excluding_connector", numericValues(results, "hyperswitch_internal_latency_ms")),
    summaryRow("combined", numericValues(results, "combined_internal_latency_ms")),
  ].filter(Boolean);
}

function phaseLatencySummary(results) {
  const phases = new Map();
  for (const result of results) {
    const phase = result.phase || "unlabelled";
    if (!phases.has(phase)) phases.set(phase, []);
    phases.get(phase).push(result);
  }
  return [...phases.entries()]
    .map(([phase, phaseResults]) => {
      const timestamps = phaseResults
        .map((result) => Date.parse(result.created_at))
        .filter(Number.isFinite)
        .sort((left, right) => left - right);
      const rps = phaseResults.map((result) => result.phase_rps).find(Number.isFinite);
      return {
        phase,
        rps: rps ?? null,
        started_at: timestamps.length ? new Date(timestamps[0]).toISOString() : null,
        ended_at: timestamps.length ? new Date(timestamps[timestamps.length - 1]).toISOString() : null,
        latency: latencySummary(phaseResults),
      };
    })
    .sort((left, right) => left.phase.localeCompare(right.phase, undefined, { numeric: true }));
}

module.exports = { latencySummary, percentile, phaseLatencySummary };
