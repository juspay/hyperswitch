"use strict";

const { formatCpuList } = require("./topology");

const APP_NAMES = ["router", "modular-pm", "encryption", "vault"];
const THREAD_APP_NAMES = [...APP_NAMES, "superposition"];

function enabled(config, section, name) {
  const item = config?.[section]?.[name];
  return Boolean(item && item.enabled !== false);
}

function workloadNames(config) {
  const names = APP_NAMES.filter((name) => enabled(config, "application_services", name));
  if (enabled(config, "state_services", "postgres") || enabled(config, "state_services", "redis")) names.push("state");
  names.push("k6");
  return names;
}

function threadWorkloadNames(config) {
  const names = THREAD_APP_NAMES.filter((name) => enabled(config, "application_services", name));
  if (enabled(config, "state_services", "postgres")) names.push("postgres");
  if (enabled(config, "state_services", "redis")) names.push("redis");
  names.push("k6");
  return names;
}

function recommendAllocation(topology, config, hostCoreCount = Math.ceil(topology.physical_core_count / 4), selection = {}) {
  const cores = topology.cores.filter((core) => core.online);
  const workloads = workloadNames(config);
  if (!Number.isInteger(hostCoreCount) || hostCoreCount < 1) throw new Error("Host physical-core count must be a positive integer");
  if (hostCoreCount + workloads.length > cores.length) {
    throw new Error(`Need ${hostCoreCount} host + ${workloads.length} workload physical cores; only ${cores.length} are online`);
  }
  const byId = new Map(cores.map((core) => [core.id, core]));
  const requestedHostIds = selection.hostCoreIds || cores.slice(0, hostCoreCount).map((core) => core.id);
  const host = requestedHostIds.map((id) => byId.get(id));
  if (host.some((core) => !core) || new Set(requestedHostIds).size !== requestedHostIds.length) throw new Error("Host core selection contains invalid or duplicate IDs");
  if (host.length !== hostCoreCount) throw new Error(`Select exactly ${hostCoreCount} host physical cores`);
  const available = cores.filter((core) => !host.includes(core));
  const selected = workloads.map((name, index) => {
    const id = selection.workloadCoreIds?.[name];
    return id ? byId.get(id) : available[index];
  });
  if (selected.some((core) => !core || host.includes(core))) throw new Error("Workload selection contains an invalid or host-owned core");
  if (new Set(selected.map((core) => core.id)).size !== selected.length) throw new Error("Each workload group requires a distinct physical core");
  const assignments = {};
  workloads.forEach((name, index) => {
    const core = selected[index];
    const selectedCpu = (key, fallback) => {
      const requested = selection.workloadLogicalCpus?.[key];
      if (requested === undefined || requested === null || requested === "") return fallback;
      const cpu = Number(requested);
      if (!Number.isInteger(cpu) || !core.logical_cpus.includes(cpu)) {
        throw new Error(`${key} logical CPU ${requested} does not belong to ${core.id}`);
      }
      return cpu;
    };
    if (name === "state") {
      assignments.postgres = String(selectedCpu("postgres", core.logical_cpus[0]));
      assignments.redis = String(selectedCpu("redis", core.logical_cpus[1] ?? core.logical_cpus[0]));
    } else {
      assignments[name] = String(selectedCpu(name, core.logical_cpus[0]));
    }
  });
  const hostCpus = host.flatMap((core) => core.logical_cpus);
  const isolated = cores.filter((core) => !host.includes(core)).flatMap((core) => core.logical_cpus);
  const used = new Set(Object.values(assignments).flatMap((value) => value.split(",").map(Number)));
  return {
    version: 1,
    topology_fingerprint: topology.fingerprint,
    host_physical_cores: host.map((core) => core.id),
    host_logical_cpus: formatCpuList(hostCpus),
    isolated_logical_cpus: formatCpuList(isolated),
    assignments,
    reserved_logical_cpus: formatCpuList(isolated.filter((cpu) => !used.has(cpu))),
  };
}

function allocateSelectedWorkloads(topology, config, workloadCoreIds, workloadLogicalCpus = {}) {
  const cores = topology.cores.filter((core) => core.online);
  const workloads = workloadNames(config);
  const selectedIds = workloads.map((name) => workloadCoreIds[name]);
  if (selectedIds.some((id) => !id)) throw new Error("Select one physical core for every workload");
  if (new Set(selectedIds).size !== selectedIds.length) throw new Error("Each workload group requires a distinct physical core");
  const knownIds = new Set(cores.map((core) => core.id));
  if (selectedIds.some((id) => !knownIds.has(id))) throw new Error("Workload selection contains an invalid core");
  const selected = new Set(selectedIds);
  const hostCoreIds = cores.filter((core) => !selected.has(core.id)).map((core) => core.id);
  if (!hostCoreIds.length) throw new Error("At least one physical core must remain for the host OS");
  return recommendAllocation(topology, config, hostCoreIds.length, {
    hostCoreIds,
    workloadCoreIds,
    workloadLogicalCpus,
  });
}

function allocateSelectedThreads(topology, config, assignments) {
  const cores = topology.cores.filter((core) => core.online);
  const workloads = threadWorkloadNames(config);
  const online = new Set(cores.flatMap((core) => core.logical_cpus));
  const selected = workloads.map((name) => Number(assignments[name]));
  if (selected.some((cpu) => !Number.isInteger(cpu) || !online.has(cpu))) {
    throw new Error("Select one available logical CPU for every workload");
  }
  if (new Set(selected).size !== selected.length) throw new Error("Each workload requires a distinct logical CPU");
  const isolated = new Set(selected);
  const hostCpus = [...online].filter((cpu) => !isolated.has(cpu));
  if (!hostCpus.length) throw new Error("At least one logical CPU must remain for the host OS");
  const hostPhysicalCores = cores
    .filter((core) => core.logical_cpus.every((cpu) => !isolated.has(cpu)))
    .map((core) => core.id);
  return {
    version: 1,
    topology_fingerprint: topology.fingerprint,
    host_physical_cores: hostPhysicalCores,
    host_logical_cpus: formatCpuList(hostCpus),
    isolated_logical_cpus: formatCpuList(selected),
    assignments: Object.fromEntries(workloads.map((name, index) => [name, String(selected[index])])),
    reserved_logical_cpus: "",
  };
}

function validateAllocation(topology, allocation) {
  const errors = [];
  if (allocation.topology_fingerprint !== topology.fingerprint) errors.push("CPU topology fingerprint changed");
  const online = new Set(topology.cores.filter((core) => core.online).flatMap((core) => core.logical_cpus));
  const assigned = new Map();
  for (const [name, raw] of Object.entries(allocation.assignments || {})) {
    for (const value of String(raw).split(",")) {
      const cpu = Number(value);
      if (!online.has(cpu)) errors.push(`${name} uses unavailable CPU ${value}`);
      if (assigned.has(cpu)) {
        const pair = new Set([name, assigned.get(cpu)]);
        if (!(pair.size === 2 && pair.has("postgres") && pair.has("redis"))) {
          errors.push(`CPU ${cpu} is assigned to both ${assigned.get(cpu)} and ${name}`);
        }
      }
      assigned.set(cpu, name);
    }
  }
  const host = new Set(expandCpuList(allocation.host_logical_cpus));
  for (const [cpu, name] of assigned) if (host.has(cpu)) errors.push(`${name} CPU ${cpu} overlaps the host set`);
  return errors;
}

function expandCpuList(value) {
  if (!value) return [];
  const result = [];
  for (const part of String(value).split(",")) {
    const [start, end = start] = part.split("-").map(Number);
    if (!Number.isInteger(start) || !Number.isInteger(end) || end < start) throw new Error(`Invalid CPU list: ${value}`);
    for (let cpu = start; cpu <= end; cpu += 1) result.push(cpu);
  }
  return result;
}

module.exports = {
  allocateSelectedThreads,
  allocateSelectedWorkloads,
  expandCpuList,
  recommendAllocation,
  threadWorkloadNames,
  validateAllocation,
  workloadNames,
};
