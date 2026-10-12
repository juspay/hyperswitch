"use strict";

const fs = require("fs");
const path = require("path");
const { discoverTopology } = require("../cpu/lib/topology");
const { validateAllocation } = require("../cpu/lib/allocation");

function allocationPath(config, configDir) {
  const configured = process.env.CPU_ALLOCATION || config.cpu_pinning?.allocation_file;
  return configured
    ? (path.isAbsolute(configured) ? configured : path.resolve(configDir, configured))
    : path.resolve(configDir, "generated/cpu/allocation.json");
}

function loadCpuAllocation(config, configDir, options = {}) {
  if (config.cpu_pinning?.enabled === false) return null;
  const file = allocationPath(config, configDir);
  if (!fs.existsSync(file)) {
    if (process.env.CPU_ALLOCATION || config.cpu_pinning?.allocation_file) throw new Error(`CPU allocation not found: ${file}`);
    return null;
  }
  const allocation = JSON.parse(fs.readFileSync(file, "utf8"));
  if (options.validate !== false) {
    const errors = validateAllocation(discoverTopology(), allocation);
    if (errors.length) throw new Error(`Invalid CPU allocation: ${errors.join("; ")}`);
  }
  return { ...allocation, file };
}

function applyServiceCpusets(config, allocation) {
  if (!allocation) return;
  for (const section of ["state_services", "application_services"]) {
    for (const [name, service] of Object.entries(config[section] || {})) {
      service.cpuset = allocation.assignments[name] || allocation.host_logical_cpus;
    }
  }
}

module.exports = { allocationPath, applyServiceCpusets, loadCpuAllocation };
