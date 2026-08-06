#!/usr/bin/env node
"use strict";

const { loadConfig } = require("./config");
const { latencySummary } = require("./results");
const { run } = require("./router_api");

async function main() {
  const action = process.argv[2] || "status";
  const result = await run(action, loadConfig(), process.argv.slice(3));
  if (result === null || result === undefined) return;
  if (action === "start") {
    console.log(`Run ${result.run_id} | attempted ${result.attempted} | succeeded ${result.succeeded} | failed ${result.failed}`);
    console.table(latencySummary(result.results || []));
    return;
  }
  console.log(typeof result === "string" ? result : JSON.stringify(result, null, 2));
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
