"use strict";

const fs = require("fs");
const path = require("path");

const MANAGED_KEYS = ["isolcpus", "nohz_full", "rcu_nocbs", "irqaffinity", "systemd.cpu_affinity"];

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'"'"'`)}'`;
}

function currentManagedOptions(cmdline) {
  return String(cmdline || "").trim().split(/\s+/).filter((item) => MANAGED_KEYS.some((key) => item.startsWith(`${key}=`)));
}

function summary(topology, allocation) {
  const lines = [
    `Topology: ${topology.physical_core_count} physical cores, ${topology.logical_cpu_count} logical CPUs, SMT width ${topology.smt_width}`,
    `Host CPUs: ${allocation.host_logical_cpus}`,
    `Isolated CPUs: ${allocation.isolated_logical_cpus}`,
    `Reserved/unused isolated CPUs: ${allocation.reserved_logical_cpus || "none"}`,
    "Assignments:",
  ];
  for (const [name, cpus] of Object.entries(allocation.assignments)) lines.push(`  ${name}: ${cpus}`);
  return `${lines.join("\n")}\n`;
}

function scripts(topology, allocation, cmdline) {
  const options = [
    `isolcpus=domain,managed_irq,${allocation.isolated_logical_cpus}`,
    `nohz_full=${allocation.isolated_logical_cpus}`,
    `rcu_nocbs=${allocation.isolated_logical_cpus}`,
    `irqaffinity=${allocation.host_logical_cpus}`,
    `systemd.cpu_affinity=${allocation.host_logical_cpus}`,
  ];
  const previous = currentManagedOptions(cmdline);
  const topologyCheck = `node "$(dirname "$0")/../../../cpu/bin/cpu.js" verify-topology "$(dirname "$0")/allocation.json"`;
  const runtimeCheck = `node "$(dirname "$0")/../../../cpu/bin/cpu.js" verify-runtime "$(dirname "$0")/allocation.json"`;
  const deleteCurrent = `for key in ${MANAGED_KEYS.join(" ")}; do\n  current="$(tr ' ' '\\n' </proc/cmdline | awk -F= -v key="$key" '$1 == key { print; exit }')"\n  [ -z "$current" ] || sudo kernelstub --delete-options "$current"\ndone`;
  const add = options.map((option) => `sudo kernelstub --add-options ${shellQuote(option)}`).join("\n");
  const rollbackAdd = previous.map((option) => `sudo kernelstub --add-options ${shellQuote(option)}`).join("\n") || ": # no previous managed options";
  return {
    "apply.sh": `#!/usr/bin/env bash\nset -euo pipefail\n${topologyCheck}\ncommand -v kernelstub >/dev/null || { echo "Unsupported bootloader: apply these options manually:" >&2; printf '%s\\n' ${options.map(shellQuote).join(" ")}; exit 2; }\n${deleteCurrent}\n${add}\necho "CPU isolation configured. Reboot, then run: just cpu-verify"\n`,
    "rollback.sh": `#!/usr/bin/env bash\nset -euo pipefail\ncommand -v kernelstub >/dev/null || { echo "kernelstub is required for automatic rollback" >&2; exit 2; }\n${deleteCurrent}\n${rollbackAdd}\necho "Previous managed kernel options restored. Reboot required."\n`,
    "verify.sh": `#!/usr/bin/env bash\nset -euo pipefail\n${topologyCheck}\ncmdline="$(cat /proc/cmdline)"\nfor option in ${options.map(shellQuote).join(" ")}; do\n  grep -Fqw -- "$option" <<<"$cmdline" || { echo "missing kernel option: $option" >&2; exit 1; }\ndone\n${runtimeCheck}\necho "Kernel isolation and running container cpusets match allocation."\n`,
  };
}

function writeArtifacts(outputDir, topology, allocation, cmdline = "") {
  fs.mkdirSync(outputDir, { recursive: true });
  fs.writeFileSync(path.join(outputDir, "topology.json"), `${JSON.stringify(topology, null, 2)}\n`);
  fs.writeFileSync(path.join(outputDir, "allocation.json"), `${JSON.stringify(allocation, null, 2)}\n`);
  fs.writeFileSync(path.join(outputDir, "summary.txt"), summary(topology, allocation));
  for (const [name, contents] of Object.entries(scripts(topology, allocation, cmdline))) {
    const file = path.join(outputDir, name);
    fs.writeFileSync(file, contents, { mode: 0o755 });
    fs.chmodSync(file, 0o755);
  }
}

module.exports = { currentManagedOptions, scripts, summary, writeArtifacts };
