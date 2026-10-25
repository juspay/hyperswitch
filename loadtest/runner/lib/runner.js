#!/usr/bin/env node
"use strict";

const { loadConfig } = require("./config");
const { latencySummary } = require("./results");
const { run } = require("./router_api");

function printLatency(latency) {
  console.log("Aggregate latency");
  console.table(latency?.aggregate || []);
  const phases = latency?.by_phase || [];
  if (!phases.length) return;
  console.log("RPS phase overview");
  console.table(phases.map((phase) => {
    const combined = phase.latency.find((row) => row.metric === "combined") || {};
    return {
      phase: phase.phase,
      rps: phase.rps,
      started_at: phase.started_at,
      ended_at: phase.ended_at,
      combined_avg_ms: combined.avg_ms ?? null,
      combined_p90_ms: combined.p90_ms ?? null,
      combined_p99_ms: combined.p99_ms ?? null,
    };
  }));
  for (const phase of phases) {
    console.log(`${phase.phase} | ${phase.rps ?? "unknown"} RPS | ${phase.started_at} → ${phase.ended_at}`);
    console.table(phase.latency);
  }
}

async function main() {
  const action = process.argv[2] || "status";
  const result = await run(action, loadConfig(), process.argv.slice(3));
  if (result === null || result === undefined) return;
  if (action === "start") {
    console.log(`Run ${result.run_id} | attempted ${result.attempted} | succeeded ${result.succeeded} | failed ${result.failed}`);
    printLatency(result.latency || { aggregate: latencySummary(result.results || []) });
    return;
  }
  if (action === "status") {
    if (!result.active_run) {
      console.log("No active run");
      return;
    }
    console.log(`Run ${result.active_run.run_id} | state ${result.active_run.state} | fixtures ${JSON.stringify(result.fixtures)}`);
    printLatency(result.latency);
    if (result.recent_results.length) {
      console.log("Recent request results");
      console.table(result.recent_results.map((record) => ({
        phase: record.phase,
        rps: record.phase_rps,
        status: record.status,
        error: record.error || "",
        payment_confirm_ms: record.payment_confirm_latency_ms,
        combined_ms: record.total_latency_ms,
      })));
    }
    return;
  }
  console.log(typeof result === "string" ? result : JSON.stringify(result, null, 2));
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
