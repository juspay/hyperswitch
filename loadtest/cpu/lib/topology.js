"use strict";

const crypto = require("crypto");
const { execFileSync } = require("child_process");

function parseLscpuCsv(source) {
  const byCore = new Map();
  for (const raw of source.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const [cpuRaw, coreRaw, socketRaw, nodeRaw, onlineRaw = "Y"] = line.split(",");
    const cpu = Number(cpuRaw);
    const core = Number(coreRaw);
    const socket = Number(socketRaw);
    const node = nodeRaw === "" ? null : Number(nodeRaw);
    if (![cpu, core, socket].every(Number.isInteger)) throw new Error(`Invalid lscpu row: ${line}`);
    const id = `socket${socket}/core${core}`;
    if (!byCore.has(id)) byCore.set(id, { id, socket, core, node, logical_cpus: [], online: true });
    const item = byCore.get(id);
    item.logical_cpus.push(cpu);
    item.online = item.online && !["N", "no", "false", "0"].includes(onlineRaw);
  }
  const cores = [...byCore.values()]
    .map((item) => ({ ...item, logical_cpus: item.logical_cpus.sort((a, b) => a - b) }))
    .sort((a, b) => a.socket - b.socket || a.core - b.core);
  if (!cores.length) throw new Error("No online CPU topology was discovered");
  const canonical = JSON.stringify(cores);
  return {
    version: 1,
    cores,
    physical_core_count: cores.filter((core) => core.online).length,
    logical_cpu_count: cores.filter((core) => core.online).reduce((sum, core) => sum + core.logical_cpus.length, 0),
    smt_width: Math.max(...cores.map((core) => core.logical_cpus.length)),
    fingerprint: crypto.createHash("sha256").update(canonical).digest("hex"),
  };
}

function discoverTopology() {
  const output = execFileSync("lscpu", ["-p=CPU,CORE,SOCKET,NODE,ONLINE"], { encoding: "utf8" });
  return parseLscpuCsv(output);
}

function formatCpuList(values) {
  const cpus = [...new Set(values.map(Number))].sort((a, b) => a - b);
  if (!cpus.length) return "";
  const ranges = [];
  let start = cpus[0];
  let end = start;
  for (const cpu of cpus.slice(1)) {
    if (cpu === end + 1) end = cpu;
    else {
      ranges.push(start === end ? `${start}` : `${start}-${end}`);
      start = end = cpu;
    }
  }
  ranges.push(start === end ? `${start}` : `${start}-${end}`);
  return ranges.join(",");
}

module.exports = { discoverTopology, formatCpuList, parseLscpuCsv };
