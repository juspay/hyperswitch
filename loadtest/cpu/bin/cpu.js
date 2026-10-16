#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");
const { loadYaml } = require("../../lib/config");
const {
  allocateSelectedThreads,
  recommendAllocation,
  threadWorkloadNames,
  validateAllocation,
} = require("../lib/allocation");
const { writeArtifacts, summary } = require("../lib/artifacts");
const { discoverTopology } = require("../lib/topology");
const { selectThreadGrid } = require("../lib/tui");

const ROOT = path.resolve(__dirname, "../..");
const DEFAULT_OUTPUT = path.join(ROOT, "deploy/generated/cpu");

function argument(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : fallback;
}

function resolveConfigPath(value) {
  if (path.isAbsolute(value)) return value;
  const candidates = [path.resolve(process.cwd(), value), path.resolve(ROOT, value)];
  if (value.startsWith("loadtest/")) candidates.push(path.resolve(ROOT, value.replace(/^loadtest\//, "")));
  return candidates.find((candidate) => fs.existsSync(candidate)) || candidates[1];
}

function whiptailConfirm(message) {
  const result = spawnSync("whiptail", ["--title", "CPU allocation", "--yesno", message, "24", "78"], { stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status === 1) return false;
  if (result.status !== 0) throw new Error(`whiptail exited with status ${result.status}`);
  return true;
}

function workloadLabel(name) {
  if (name === "postgres") return "PostgreSQL (psql)";
  if (name === "k6") return "load generator (k6)";
  return name;
}

async function plan() {
  const topology = discoverTopology();
  const configPath = resolveConfigPath(argument("--config", process.env.CONFIG || "deploy/config.docker-hub.yaml"));
  const config = loadYaml(configPath);
  const recommendedHost = Math.max(1, Math.ceil(topology.physical_core_count / 4));
  const nonInteractive = process.argv.includes("--non-interactive");
  const requested = argument("--host-cores", recommendedHost);
  const hostCount = Number(requested);
  let allocation;
  if (nonInteractive) {
    allocation = recommendAllocation(topology, config, hostCount);
  } else {
    const onlineCores = topology.cores.filter((core) => core.online);
    const workloads = threadWorkloadNames(config);
    const onlineCpus = onlineCores.flatMap((core) => core.logical_cpus);
    if (workloads.length >= onlineCpus.length) {
      throw new Error(`Need ${workloads.length} workload threads and at least 1 host thread; only ${onlineCpus.length} are online`);
    }
    const existing = fs.existsSync(path.join(DEFAULT_OUTPUT, "allocation.json"))
      ? readAllocation()
      : null;
    const assignments = {};
    for (const [index, name] of workloads.entries()) {
      const label = workloadLabel(name);
      const taken = new Map(Object.entries(assignments).map(([owner, cpu]) => [Number(cpu), owner]));
      const preferred = Number(existing?.assignments?.[name]);
      const defaultCpu = onlineCpus.find((cpu) => !taken.has(cpu) && cpu === preferred)
        ?? onlineCpus.find((cpu) => !taken.has(cpu));
      const selected = await selectThreadGrid({
        title: `Assign ${label}`,
        prompt: `${topology.physical_core_count} cores / ${topology.logical_cpu_count} CPUs | workload ${index + 1}/${workloads.length} | claimed threads disabled`,
        cores: onlineCores,
        initialCpu: defaultCpu,
        unavailableCpus: taken,
      });
      assignments[name] = selected.cpu;
    }
    allocation = allocateSelectedThreads(topology, config, assignments);
  }
  const errors = validateAllocation(topology, allocation);
  if (errors.length) throw new Error(errors.join("; "));
  const text = summary(topology, allocation);
  if (!nonInteractive && !whiptailConfirm(`${text}\nGenerate this allocation?`)) throw new Error("CPU planning cancelled");
  const output = path.resolve(argument("--output", DEFAULT_OUTPUT));
  const cmdline = fs.existsSync("/proc/cmdline") ? fs.readFileSync("/proc/cmdline", "utf8") : "";
  writeArtifacts(output, topology, allocation, cmdline);
  process.stdout.write(`${text}\nGenerated: ${output}\n`);
}

function readAllocation(file = path.join(DEFAULT_OUTPUT, "allocation.json")) {
  return JSON.parse(fs.readFileSync(path.resolve(file), "utf8"));
}

function verifyTopology(file) {
  const allocation = readAllocation(file);
  const errors = validateAllocation(discoverTopology(), allocation);
  if (errors.length) throw new Error(errors.join("; "));
  console.log("CPU topology matches allocation.");
}

function verifyRuntime(file) {
  const allocation = readAllocation(file);
  const topologyErrors = validateAllocation(discoverTopology(), allocation);
  if (topologyErrors.length) throw new Error(topologyErrors.join("; "));
  console.log("Checking managed Docker container cpusets...");
  const result = spawnSync("docker", ["ps", "--filter", "label=io.hyperswitch.loadtest.managed=true", "--format", "{{.Names}}\t{{.Label \"io.hyperswitch.loadtest.service\"}}"], { encoding: "utf8" });
  if (result.error?.code === "ENOENT") throw new Error("docker is required for runtime verification");
  if (result.status !== 0) throw new Error(result.stderr.trim() || "docker ps failed");
  const containers = result.stdout.trim().split(/\r?\n/).filter(Boolean);
  if (!containers.length) {
    console.log("No running managed containers; runtime cpuset verification skipped.");
    return;
  }
  const failures = [];
  for (const line of containers) {
    const [container, service] = line.split("\t");
    const expected = allocation.assignments[service] || allocation.host_logical_cpus;
    const inspected = spawnSync("docker", ["inspect", "--format", "{{.HostConfig.CpusetCpus}}", container], { encoding: "utf8" });
    const actual = inspected.stdout.trim();
    const ok = inspected.status === 0 && actual === expected;
    console.log(`${ok ? "ok" : "FAIL"} ${container} (${service || "support"}): expected=${expected} actual=${actual || "unrestricted"}`);
    if (!ok) failures.push(container);
  }
  if (failures.length) throw new Error(`CPU affinity mismatch: ${failures.join(", ")}`);
}

async function main() {
  const command = process.argv[2] || "plan";
  if (command === "plan") return plan();
  if (command === "show") return process.stdout.write(summary(discoverTopology(), readAllocation(argument("--allocation", undefined))));
  if (command === "verify-topology") return verifyTopology(process.argv[3]);
  if (command === "verify-runtime") return verifyRuntime(process.argv[3]);
  throw new Error(`Unknown CPU command: ${command}`);
}

main().catch((error) => { console.error(`error: ${error.message}`); process.exit(1); });
